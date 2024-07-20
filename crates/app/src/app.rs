#![allow(unused)]
use std::{
    collections::VecDeque,
    sync::{mpsc::channel, Arc},
    thread::JoinHandle,
};

use ecs::{prelude::*, Events, Scheduler, UnsafeWorldCell, WinnyEvent, WinnyResource, World};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{self, DeviceEvent, DeviceId, ElementState, WindowEvent},
    event_loop::{self, ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

use crate::{
    plugins::{Plugin, PluginSet},
    prelude::KeyState,
    window::WindowResized,
};
use crate::{
    prelude::{KeyCode, KeyInput, MouseInput, WindowPlugin},
    window::WinitWindow,
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
            scheduler: Scheduler::default(),
            plugins: VecDeque::new(),
        }
    }
}

impl App {
    pub(crate) fn empty() -> Self {
        Self {
            world: World::default(),
            scheduler: Scheduler::default(),
            plugins: VecDeque::new(),
        }
    }

    pub(crate) fn add_plugin_boxed(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push_back(plugin);
    }

    pub(crate) fn add_plugin_priority_boxed(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push_front(plugin);
    }

    pub fn world(&self) -> &World {
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
        for p in plugins.get().into_iter().rev() {
            self.add_plugin_priority_boxed(p);
        }

        self
    }

    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.insert_resource(resource);

        self
    }

    pub fn register_resource<R: Resource>(&mut self) -> &mut Self {
        self.world.register_resource::<R>();

        self
    }

    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        self.world.register_event::<E>();
        self.add_systems(Schedule::FlushEvents, flush_event_queue::<E>);

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

    fn insert_winit_event(&mut self, event: WinitEvent) {
        match event {
            WinitEvent::KeyboardInput(key) => {
                if let PhysicalKey::Code(key_code) = key.physical_key {
                    self.world_mut().push_event(KeyInput::new(
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
        }
    }

    pub fn run(&mut self) {
        while let Some(mut plugin) = self.plugins.pop_front() {
            plugin.build(self);
        }

        self.scheduler.build_schedule(&mut self.world);

        // println!("{:#?}", self.scheduler);
        // panic!();

        let mut app = App::empty();
        std::mem::swap(self, &mut app);
        let mut win_app = WinitApp::new(app);

        let mut event_loop = EventLoop::builder();
        let event_loop = event_loop.build().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        let _ = event_loop.run_app(&mut win_app);
    }

    fn update(&mut self) -> Result<(), ExitCode> {
        update_ecs(&mut self.world, &mut self.scheduler)
    }

    fn startup(&mut self) {
        self.scheduler.startup(&mut self.world);
    }

    fn flush_events(&mut self) {
        self.scheduler.flush_events(&mut self.world);
    }

    fn exit(&mut self) {
        self.scheduler.exit(&mut self.world);
    }
}

fn flush_event_queue<E: Event>(queue: EventReader<E>) {
    queue.flush();
}

#[derive(Debug, WinnyResource)]
pub struct DeltaT(pub f64);

fn update_ecs(world: &mut World, scheduler: &mut Scheduler) -> Result<(), ExitCode> {
    run_and_handle_panic(world, scheduler, |world, scheduler| {
        scheduler.run(world);
        check_for_exit(world, scheduler)
    })
}

fn resize_ecs(world: &mut World, scheduler: &mut Scheduler) -> Result<(), ExitCode> {
    run_and_handle_panic(world, scheduler, |world, scheduler| {
        scheduler.resized(world);
        false
    })
}

fn render_ecs(world: &mut World, scheduler: &mut Scheduler) -> Result<(), ExitCode> {
    run_and_handle_panic(world, scheduler, |world, scheduler| {
        scheduler.render(world);
        check_for_exit(world, scheduler)
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn run_and_handle_panic<F>(
    world: &mut World,
    scheduler: &mut Scheduler,
    f: F,
) -> Result<(), ExitCode>
where
    F: FnOnce(&mut World, &mut Scheduler) -> bool + Send,
{
    let mut exit = false;
    let mut panicking = false;
    std::thread::scope(|s| {
        let h = s.spawn(|| exit = f(world, scheduler));

        if let Err(_) = h.join() {
            panicking = true;
        }
    });

    if panicking {
        Err(ExitCode::Panicking)
    } else if exit {
        Err(ExitCode::ExitApp)
    } else {
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
fn run_and_handle_panic<F>(
    world: &mut World,
    scheduler: &mut Scheduler,
    f: F,
) -> Result<(), ExitCode>
where
    F: FnOnce(&mut World, &mut Scheduler) -> bool,
{
    if f(world, scheduler) {
        Err(ExitCode::ExitApp)
    } else {
        Ok(())
    }
}

fn check_for_exit(world: &mut World, scheduler: &mut Scheduler) -> bool {
    world
        .resource_mut::<Events<AppExit>>()
        .read()
        .next()
        .is_some()
}

#[derive(Debug, WinnyEvent)]
pub enum WinitEvent {
    KeyboardInput(winit::event::KeyEvent),
    MouseInput(winit::event::ElementState, winit::event::MouseButton),
    MouseMotion(f64, f64),
}

struct WinitApp {
    app: App,
    exit_requested: bool,
    created_window: bool,
    startup: bool,
    presented_frames: u32,
    clock: chrono::DateTime<chrono::Local>,
}

enum ExitCode {
    ExitApp,
    Panicking,
}

impl WinitApp {
    pub fn new(app: App) -> Self {
        Self {
            app,
            exit_requested: false,
            created_window: false,
            startup: false,
            presented_frames: 0,
            clock: chrono::Local::now(),
        }
    }

    pub fn update(&mut self) -> Result<(), ExitCode> {
        self.app.update()
    }

    pub fn render(&mut self) -> Result<(), ExitCode> {
        render_ecs(&mut self.app.world, &mut self.app.scheduler)
    }
}

// impl ApplicationHandler<ControlFlowEvent> for WinitApp {
impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.created_window {
            return;
        }

        util::tracing::info!("App resumed: Initializing");
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

        #[cfg(target_arch = "wasm32")]
        {
            use crate::window::winit;
            use winit::platform::web::WindowExtWebSys;
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("winny-wasm")?;
                    let canvas = web_sys::Element::from(window.canvas()?);
                    dst.append_child(&canvas).ok()?;
                    Some(())
                })
                .expect("Couldn't append canvas to document body.");

            use winit::dpi::PhysicalSize;
            if let Some(size) = window.request_inner_size(PhysicalSize::new(16 * 60, 16 * 60)) {
                util::tracing::info!("requested inner window size: {size:?}");
            } else {
                util::tracing::info!("failed to request size, awaiting resized event");
                let window = WinitWindow(Arc::new(window));
                self.app.insert_resource(window);
                self.created_window = true;
                return;
            }
        }

        let window = WinitWindow(Arc::new(window));
        self.app.insert_resource(window);
        self.created_window = true;
        self.app.startup();
        self.startup = true;
    }

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        window_event: event::WindowEvent,
    ) {
        match window_event {
            winit::event::WindowEvent::CloseRequested => {
                self.exit_requested = true;
            }
            winit::event::WindowEvent::Resized(size) => {
                #[cfg(target_arch = "wasm32")]
                if !self.startup {
                    self.app.startup();
                    self.startup = true;
                }
                self.app
                    .world
                    .insert_resource(WindowResized(size.width, size.height));
                resize_ecs(&mut self.app.world, &mut self.app.scheduler);
                self.app.world.take_resource::<WindowResized>();
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } => self
                .app
                .insert_winit_event(WinitEvent::KeyboardInput(event)),
            winit::event::WindowEvent::RedrawRequested => {
                // self.render();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if !self.startup {
            return;
        }

        let app = &mut self.app;
        let start = chrono::Local::now();
        if let Err(e) = self.update() {
            match e {
                ExitCode::ExitApp => self.exit_requested = true,
                ExitCode::Panicking => {
                    event_loop.exit();
                    return;
                }
            }
        }
        let update_end = chrono::Local::now().signed_duration_since(start);

        let start = chrono::Local::now();
        if let Err(e) = self.render() {
            match e {
                ExitCode::ExitApp => self.exit_requested = true,
                ExitCode::Panicking => {
                    event_loop.exit();
                    return;
                }
            }
        }
        let render_end = chrono::Local::now().signed_duration_since(start);
        self.presented_frames += 1;

        self.app.scheduler.flush_events(&mut self.app.world);

        if chrono::Local::now().signed_duration_since(self.clock) >= chrono::TimeDelta::seconds(1) {
            let fps = self.presented_frames;
            let title = self.app.world().resource::<WindowPlugin>().title;
            let window = self.app.world().resource::<WinitWindow>();
            window.set_title(
                format!(
                    "{} - {} - {}ms - {}ms",
                    title,
                    fps,
                    update_end.num_milliseconds(),
                    render_end.num_milliseconds()
                )
                .as_str(),
            );
            self.presented_frames = 0;
            self.clock = chrono::Local::now();
        }

        if self.exit_requested {
            self.app.exit();
            event_loop.exit();
        }
    }
}

// fn user_event(&mut self, _event_loop: &event_loop::ActiveEventLoop, event: ControlFlowEvent) {
//     if let Some(app) = &mut self.app {
//         match event {
//             ControlFlowEvent::Update => {
//                 // app.update();
//             }
//             ControlFlowEvent::Render => {
//                 // app.window.request_redraw();
//                 // app.render();
//             }
//             ControlFlowEvent::Fps(fps) => {
//                 app.window
//                     .set_title(format!("scerm-beta-0.0.1 - {}", fps).as_str());
//             }
//         }
//     }
// }

// enum ControlFlowEvent {
//     Update,
//     Render,
//     Fps(u32),
// }

// let event_loop = EventLoop::with_user_event().build().unwrap();
// let event_loop_proxy = event_loop.create_proxy();

// std::thread::spawn(move || {
//     let dt = Duration::from_millis(10);
//     let mut updates = 0;
//     let mut presented_frames = 0;
//     let mut last_frame = SystemTime::now();
//     let mut clock = SystemTime::now();
//     let mut accumulator = Duration::default();

//     loop {
//         let now = SystemTime::now();
//         let frame_time = now.duration_since(last_frame).unwrap_or_default();
//         last_frame = now;
//         accumulator += frame_time;

//         if accumulator >= dt {
//             let _ = event_loop_proxy.send_event(ControlFlowEvent::Update);
//             let _ = event_loop_proxy.send_event(ControlFlowEvent::Render);
//             accumulator = accumulator.saturating_sub(dt);
//             updates += 1;
//             presented_frames += 1;
//         }

//         if SystemTime::now().duration_since(clock).unwrap_or_default() >= Duration::from_secs(1)
//         {
//             let _ = event_loop_proxy.send_event(ControlFlowEvent::Fps(presented_frames));
//             presented_frames = 0;
//             updates = 0;
//             clock = now;
//             accumulator = Duration::default();
//         }
//     }
// });
