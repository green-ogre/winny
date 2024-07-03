#![allow(unused)]
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ecs::{prelude::*, Events, Scheduler, WinnyEvent, WinnyResource, World};
use logger::{error, info};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceEvent, DeviceId, ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

use crate::{
    perf::PerfCounter,
    prelude::{KeyCode, KeyInput, MouseInput, WindowPlugin},
    window::WinitWindow,
};
use crate::{
    plugins::{Plugin, PluginSet},
    prelude::KeyState,
    renderer::{RenderContext, Renderer},
};

#[derive(Debug, WinnyEvent)]
pub struct AppExit;

pub struct App {
    world: World,
    scheduler: Scheduler,
    plugins: VecDeque<Box<dyn Plugin>>,
}

impl Default for App {
    fn default() -> Self {
        let mut world = World::default();
        world.register_event::<AppExit>();
        world.insert_resource(DeltaT(0.0));

        App {
            world,
            scheduler: Scheduler::new(),
            plugins: VecDeque::new(),
        }
    }
}

fn run_schedule(schedule: Schedule, scheduler: &mut Scheduler, world: &mut World) {
    scheduler.run_schedule(schedule, world);
}

impl App {
    pub(crate) fn empty() -> Self {
        Self {
            world: World::default(),
            scheduler: Scheduler::new(),
            plugins: VecDeque::new(),
        }
    }

    pub(crate) fn add_plugin_boxed(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push_back(plugin);
    }

    pub(crate) fn add_plugin_priority_boxed(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push_front(plugin);
    }

    pub(crate) fn run_schedule(&mut self, schedule: Schedule) {
        run_schedule(schedule, &mut self.scheduler, &mut self.world);
    }

    pub fn world(&mut self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn add_plugins<T: PluginSet>(&mut self, plugins: T) -> &mut Self {
        for p in plugins.get().into_iter() {
            self.add_plugin_boxed(p);
        }

        self
    }

    // Should be used for plugins that are dependencies of child plugins
    pub fn add_plugins_priority<T: PluginSet>(&mut self, plugins: T) -> &mut Self {
        for p in plugins.get().into_iter() {
            self.add_plugin_priority_boxed(p);
        }

        self
    }

    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        self.world.register_event::<E>();
        self.add_systems(Schedule::FlushEvents, |queue: EventReader<E>| {
            let _ = queue.read();
        });
        self
    }

    pub fn add_systems<M, B: IntoSystemStorage<M>>(
        &mut self,
        schedule: Schedule,
        systems: B,
    ) -> &mut Self {
        self.scheduler.add_systems(schedule, systems);
        self
    }

    pub fn insert_winit_events(&mut self, events: impl Iterator<Item = WinitEvent>) -> &mut Self {
        for event in events {
            match event {
                WinitEvent::KeyboardInput(key) => {
                    let window_plugin = self.world().resource::<WindowPlugin>();
                    if window_plugin.close_on_escape {
                        if key.physical_key == PhysicalKey::Code(winit::keyboard::KeyCode::Escape) {
                            let mut app_exit = self.world_mut().resource_mut::<Events<AppExit>>();
                            app_exit.push(AppExit);
                        }
                    }

                    let mut key_input = self.world_mut().resource_mut::<Events<KeyInput>>();

                    if let PhysicalKey::Code(key_code) = key.physical_key {
                        key_input.push(KeyInput::new(
                            KeyCode::new(key_code),
                            match key.state {
                                ElementState::Pressed => KeyState::Pressed,
                                ElementState::Released => KeyState::Released,
                            },
                        ));
                    }
                }
                // TODO: mouse input
                WinitEvent::MouseInput(state, button) => {
                    let mut mouse_input = self.world_mut().resource_mut::<Events<MouseInput>>();
                }
                WinitEvent::MouseMotion(x, y) => {
                    let mut mouse_input = self.world_mut().resource_mut::<Events<MouseInput>>();
                }
                WinitEvent::RedrawRequested => {
                    let mut redraw = self.world_mut().resource_mut::<Events<RedrawRequest>>();
                    redraw.push(RedrawRequest);
                }
                WinitEvent::WindowCreated => {
                    let mut event = self.world_mut().resource_mut::<Events<WindowCreated>>();
                    event.push(WindowCreated);
                }
            }
        }

        self
    }

    pub fn run(&mut self) {
        logger::init();

        while let Some(mut plugin) = self.plugins.pop_front() {
            plugin.build(self);
        }

        self.scheduler.build_schedule();
        self.scheduler.init_systems(&self.world);

        let mut app = App::empty();
        std::mem::swap(self, &mut app);
        let mut win_app = WinitApp::new(app);

        let mut event_loop = EventLoop::builder();
        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::EventLoopBuilderExtWindows;
            event_loop.with_any_thread(true);
        }
        let event_loop = event_loop.build().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);

        let _ = event_loop.run_app(&mut win_app);
    }

    fn update(&mut self) -> bool {
        let start = SystemTime::now();
        if !update_ecs(&mut self.world, &mut self.scheduler) {
            return false;
        }
        let end = SystemTime::now();
        let dt = DeltaT(end.duration_since(start).unwrap_or_default().as_secs_f64());
        update_delta_t(&mut self.world, dt);

        true
    }
}

// TODO: better panics => this is useful for exiting if non main scope panics
fn set_panic_hook() {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let line = line!();
        let column = column!();
        let file = file!();
        error!("[{}:{}:{}] Panic => Exiting...", file, line, column);
        orig_hook(panic_info);
        std::process::exit(1);
    }));
}

#[derive(Debug, WinnyResource)]
pub struct DeltaT(pub f64);

fn update_ecs(world: &mut World, scheduler: &mut Scheduler) -> bool {
    if world
        .resource_ids
        .contains_key(&std::any::TypeId::of::<PerfCounter>())
    {
        // TODO: fix me
        let mut perf = world.resource_mut::<PerfCounter>().clone();
        perf.start();
        run_schedule_and_log(scheduler, &mut perf, world, Schedule::Platform);
        run_schedule_and_log(scheduler, &mut perf, world, Schedule::PreUpdate);
        run_schedule_and_log(scheduler, &mut perf, world, Schedule::Update);
        run_schedule_and_log(scheduler, &mut perf, world, Schedule::PostUpdate);
        run_schedule_and_log(scheduler, &mut perf, world, Schedule::Render);
        let exit = check_for_exit(world, scheduler);
        run_schedule_and_log(scheduler, &mut perf, world, Schedule::FlushEvents);
        perf.stop();
        *world.resource_mut::<PerfCounter>() = perf;
        !exit
    } else {
        scheduler.run(world);
        !check_for_exit(world, scheduler)
    }
}

fn run_schedule_and_log(
    scheduler: &mut Scheduler,
    perf: &mut PerfCounter,
    world: &mut World,
    schedule: Schedule,
) {
    perf.start_debug_event();
    scheduler.run_schedule(schedule, world);
    perf.stop_debug_event();
    perf.log_last_debug_event(format!("ECS: {:?}", schedule).as_str());
}

fn update_delta_t(world: &mut World, delta_t: DeltaT) {
    let mut dt = world.resource_mut::<DeltaT>();
    *dt = delta_t;
}

fn check_for_exit(world: &mut World, scheduler: &mut Scheduler) -> bool {
    world
        .resource_mut::<Events<AppExit>>()
        .read()
        .next()
        .is_some()
}

#[derive(WinnyEvent)]
pub struct RedrawRequest;

#[derive(WinnyEvent)]
pub struct WindowResized(u32, u32);

#[derive(WinnyEvent)]
pub struct WindowCreated;

#[derive(WinnyEvent)]
pub enum WinitEvent {
    KeyboardInput(winit::event::KeyEvent),
    MouseInput(winit::event::ElementState, winit::event::MouseButton),
    RedrawRequested,
    MouseMotion(f64, f64),
    WindowCreated,
}

struct WinitApp {
    app: App,
    winit_events: Vec<WinitEvent>,
    window: Option<Arc<Window>>,
}

impl WinitApp {
    pub fn new(app: App) -> Self {
        let winit_events = Vec::new();
        let created_window = false;

        Self {
            app,
            winit_events,
            window: None,
        }
    }
}

impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_plugin = self.app.world().resource::<WindowPlugin>();
            let window_attributes = Window::default_attributes()
                .with_title(window_plugin.title)
                .with_inner_size(PhysicalSize::new(
                    window_plugin.inner_size.0,
                    window_plugin.inner_size.1,
                ))
                .with_position(PhysicalPosition::new(
                    window_plugin.position.0,
                    window_plugin.position.1,
                ));
            let window = event_loop.create_window(window_attributes).unwrap();
            // self.app
            //     .world_mut()
            //     .insert_resource(WinitWindow(Arc::new(window)));
            let mut event = self.app.world_mut().resource_mut::<Events<WindowCreated>>();
            event.push(WindowCreated);

            let window = Arc::new(window);

            let renderer = Renderer::new(
                Arc::clone(&window),
                window.inner_size().into(),
                window.inner_size().into(),
            );
            let renderer_context =
                RenderContext::new(Arc::clone(&renderer.device), Arc::clone(&renderer.queue));

            self.app
                .insert_resource(renderer)
                .insert_resource(renderer_context);
            self.window = Some(window);
            self.app.run_schedule(Schedule::StartUp);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => resize(&mut self.app, size),
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                self.winit_events.push(WinitEvent::KeyboardInput(event))
            }
            // WindowEvent::CursorMoved { position, .. } => self
            //     .winit_events
            //     .push(WinitEvent::CursorMoved((position.x, position.y))),
            WindowEvent::MouseInput { state, button, .. } => self
                .winit_events
                .push(WinitEvent::MouseInput(state, button)),
            WindowEvent::RedrawRequested => self.winit_events.push(WinitEvent::RedrawRequested),
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseMotion { delta } => self
                .winit_events
                .push(WinitEvent::MouseMotion(delta.0, delta.1)),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.app.insert_winit_events(self.winit_events.drain(..));
        if self.window.is_some() {
            if !self.app.update() {
                event_loop.exit();
            }
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.app.run_schedule(Schedule::Exit);
    }
}

// TODO: handle window resize
fn resize(app: &mut App, size: PhysicalSize<u32>) {
    let mut resize = app.world_mut().resource_mut::<Events<WindowResized>>();
    resize.push(WindowResized(size.width, size.height));
}

// #[cfg(feature = "controller")]
// fn spawn_controller_thread(
//     controller_input_sender: Sender<(ControllerInput, ControllerAxisState)>,
// ) {
//     use logger::error;
//
//     std::thread::spawn(move || {
//         let mut gilrs = Gilrs::new().unwrap();
//         let mut controller_axis_state = ControllerAxisState::new();
//
//         loop {
//             while let Some(gilrs::Event { event, .. }) = gilrs.next_event() {
//                 let input = ControllerInputState::from(event);
//
//                 if let Some(new_axis_state) = input.axis_state() {
//                     controller_axis_state.apply_new_state(new_axis_state);
//                 }
//
//                 if controller_input_sender
//                     .send((ControllerInput::new(input), controller_axis_state))
//                     .is_err()
//                 {
//                     error!("Error sending controller input");
//                 }
//             }
//         }
//     });
// }
//
// #[cfg(feature = "controller")]
// fn pipe_controller_input(
//     channels: Res<WindowChannels>,
//     mut controller_event: EventWriter<ControllerInput>,
//     mut controller_axis_state: ResMut<ControllerAxisState>,
// ) {
//     for (input, axis_state) in channels.cirx.try_iter() {
//         controller_event.send(input);
//         *controller_axis_state = axis_state;
//     }
// }
//
// fn run(app: &mut App) {
//     logger::init();
//     set_panic_hook();
//     app.register_event::<AppExit>();
//     app.insert_resource(DeltaT(0.0));
//
//     app.register_event::<KeyInput>();
//     app.register_event::<MouseInput>();
//     // #[cfg(feature = "controller")]
//     // app.register_event::<ControllerInput>();
//     // #[cfg(feature = "controller")]
//     // app.insert_resource(ControllerAxisState::new());
//     app.insert_resource(MouseState::default());
//
//     let window_plugin = app.world().resource::<WindowPlugin>();
//     let event_loop = EventLoop::new().unwrap();
//     let window = WindowBuilder::new()
//         .with_inner_size(PhysicalSize::new(
//             window_plugin.inner_size.0,
//             window_plugin.inner_size.1,
//         ))
//         .with_position(PhysicalPosition::new(
//             window_plugin.position.0,
//             window_plugin.position.1,
//         ))
//         .build(&event_loop)
//         .unwrap();
//     event_loop.set_control_flow(ControlFlow::Poll);
//
//     // #[cfg(feature = "controller")]
//     // let (controller_input_sender, controller_input_reciever) =
//     //     channel::<(ControllerInput, ControllerAxisState)>();
//     // #[cfg(feature = "controller")]
//     // spawn_controller_thread(controller_input_sender);
//
//     // let (wwetx, wwerx) = channel();
//     // let (wdetx, wderx) = channel::<DeviceEvent>();
//     // let (wetx, werx) = channel::<()>();
//
//     // #[cfg(feature = "controller")]
//     // let channels = WindowChannels::new(controller_input_reciever, wwerx, wderx, wetx);
//     // #[cfg(not(feature = "controller"))]
//     // let channels = WindowChannels::new(wwerx, wderx, wetx);
//
//     while let Some(mut plugin) = app.plugins.pop_front() {
//         plugin.build(app);
//     }
//
//     app.scheduler.build_schedule();
//     app.scheduler.init_systems(&app.world);
//
//     let mut world = &mut app.world;
//     let mut scheduler = &mut app.scheduler;
//
//     // println!("{scheduler:#?}");
//
//     std::thread::scope(|s| {
//         let h = s.spawn(move || {
//             scheduler.startup(&world);
//
//             let mut start = SystemTime::now();
//             let mut end = SystemTime::now();
//             loop {
//                 let dt = DeltaT(end.duration_since(start).unwrap_or_default().as_secs_f64());
//                 start = SystemTime::now();
//                 if !update_ecs(dt, &mut world, &mut scheduler) {
//                     break;
//                 }
//                 end = SystemTime::now();
//
//                 // world.print_size();
//             }
//         });
//
//         if let Some(window_event_loop) = self.window_event_loop.take() {
//             window_event_loop();
//         } else {
//             let _ = h.join();
//         }
//     });
//
//     let _ = event_loop.run(move |event, elwt| match event {
//         winit::event::Event::WindowEvent { window_id, event } => match event {
//             WindowEvent::CursorMoved { position, .. } => {
//                 user_mouse_input.send(MouseInput::new(
//                     0.0,
//                     0.0,
//                     position.x,
//                     position.y,
//                     None,
//                     mouse_state.last_held_key,
//                 ));
//                 mouse_state.last_mouse_position = (position.x, position.y);
//             }
//             WindowEvent::MouseInput { state, button, .. } => {
//                 let mut held = None;
//
//                 mouse_state.last_held_key = if state == ElementState::Pressed {
//                     let button = match button {
//                         winit::event::MouseButton::Left => Some(MouseButton::Left),
//                         winit::event::MouseButton::Right => Some(MouseButton::Right),
//                         _ => None,
//                     };
//
//                     if button == mouse_state.last_held_key {
//                         held = button;
//                         None
//                     } else {
//                         button
//                     }
//                 } else {
//                     None
//                 };
//
//                 user_mouse_input.send(MouseInput::new(
//                     0.0,
//                     0.0,
//                     mouse_state.last_mouse_position.0,
//                     mouse_state.last_mouse_position.1,
//                     mouse_state.last_held_key,
//                     held,
//                 ));
//             }
//             WindowEvent::Resized(new_size) => {
//                 renderer.resize(new_size.into());
//             }
//             WindowEvent::CloseRequested => {
//                 exit_event_loop.send(()).unwrap();
//                 app_exit.send(AppExit);
//                 return false;
//             }
//             WindowEvent::KeyboardInput {
//                 event: key_event, ..
//             } => {
//                 if window_plugin.close_on_escape {
//                     if key_event.physical_key == PhysicalKey::Code(winit::keyboard::KeyCode::Escape)
//                     {
//                         exit_event_loop.send(()).unwrap();
//                         app_exit.send(AppExit);
//                         return false;
//                     }
//                 }
//
//                 if let PhysicalKey::Code(key_code) = key_event.physical_key {
//                     user_key_input.send(KeyInput::new(
//                         KeyCode::new(key_code),
//                         match key_event.state {
//                             ElementState::Pressed => KeyState::Pressed,
//                             ElementState::Released => KeyState::Released,
//                         },
//                     ));
//                 }
//             }
//             _ => {}
//         },
//         _ => {}
//     });
// }
