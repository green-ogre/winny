use crate::prelude::*;
use crate::window::Window;
use crate::window::WindowPlugin;
use crate::{
    input::mouse_and_key,
    window::{ViewPort, WindowResized},
};
use ecs::sets::IntoSystemStorage;
use ecs::{
    events::Events,
    schedule::{ScheduleLabel, Scheduler},
    WinnyEvent, WinnyScheduleLabel, *,
};
use math::vector::Vec2f;
use mouse_and_key::MouseMotion;
use std::{collections::VecDeque, sync::Arc};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{self, DeviceEvent, DeviceId, ElementState},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
};

#[derive(Debug, WinnyEvent)]
pub struct AppExit;

#[derive(WinnyScheduleLabel, Debug, Clone, Copy)]
pub enum Schedule {
    PreUpdate,
    Update,
    PostUpdate,
    StartUp,
    Exit,
}

#[derive(WinnyScheduleLabel, Debug, Clone, Copy)]
pub enum AppSchedule {
    Platform,
    RenderStartup,
    PreStartUp,
    FlushEvents,
    Resized,
    SubmitEncoder,
    PrepareRender,
    PreRender,
    Render,
    RenderLighting,
    PostRender,
    Present,
}

pub struct App {
    world: World,
    scheduler: Scheduler,
    plugins: VecDeque<Box<dyn Plugin>>,
    egui_registry: Option<ecs::egui_widget::EguiRegistery>,
}

impl Default for App {
    fn default() -> Self {
        let mut world = World::default();
        world.register_event::<AppExit>();

        App {
            world,
            scheduler: Scheduler::default(),
            plugins: VecDeque::new(),
            egui_registry: Some(ecs::egui_widget::EguiRegistery::default()),
        }
    }
}

impl App {
    pub(crate) fn empty() -> Self {
        Self {
            world: World::default(),
            scheduler: Scheduler::default(),
            plugins: VecDeque::new(),
            egui_registry: Some(ecs::egui_widget::EguiRegistery::default()),
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
        self.add_systems(AppSchedule::FlushEvents, flush_event_queue::<E>);

        self
    }

    pub fn add_systems<M, B: IntoSystemStorage<M>>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: B,
    ) -> &mut Self {
        self.scheduler.add_systems(schedule, systems);

        self
    }

    pub fn egui_component<C: Component + ecs::egui_widget::AsEgui>(&mut self) -> &mut Self {
        #[cfg(feature = "widgets")]
        self.egui_registry
            .as_mut()
            .unwrap()
            .register_component::<C>();
        self
    }

    pub fn egui_resource<R: Resource + ecs::egui_widget::AsEgui>(&mut self) -> &mut Self {
        #[cfg(feature = "widgets")]
        self.egui_registry
            .as_mut()
            .unwrap()
            .register_resource::<R>();
        self
    }

    pub fn egui_blacklist<T: 'static>(&mut self) -> &mut Self {
        #[cfg(feature = "widgets")]
        self.egui_registry.as_mut().unwrap().blacklist::<T>();
        self
    }

    pub fn run(&mut self) {
        while let Some(mut plugin) = self.plugins.pop_front() {
            plugin.build(self);
        }

        let registry = self.egui_registry.take().unwrap();
        self.insert_resource(registry);

        self.scheduler.init_schedule(&mut self.world);

        let mut app = App::empty();
        std::mem::swap(self, &mut app);
        let mut win_app = WinitApp::new(app);

        let mut event_loop = EventLoop::builder();
        let event_loop = event_loop.build().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        let _ = event_loop.run_app(&mut win_app);
    }
}

fn flush_event_queue<E: Event>(queue: EventReader<E>) {
    queue.flush();
}

fn update(scheduler: &mut Scheduler, world: &mut World) {
    scheduler.run_schedule(world, AppSchedule::Platform);
    scheduler.run_schedule(world, Schedule::PreUpdate);
    scheduler.run_schedule(world, Schedule::Update);
    scheduler.run_schedule(world, Schedule::PostUpdate);
}

fn startup(scheduler: &mut Scheduler, world: &mut World) {
    scheduler.run_schedule(world, AppSchedule::RenderStartup);
    scheduler.run_schedule(world, AppSchedule::PreStartUp);
    scheduler.run_schedule(world, Schedule::StartUp);
}

fn flush_events(scheduler: &mut Scheduler, world: &mut World) {
    scheduler.run_schedule(world, AppSchedule::FlushEvents);
}

fn resized(scheduler: &mut Scheduler, world: &mut World) {
    scheduler.run_schedule(world, AppSchedule::Resized);
}

fn render(scheduler: &mut Scheduler, world: &mut World) {
    scheduler.run_schedule(world, AppSchedule::PrepareRender);
    scheduler.run_schedule(world, AppSchedule::PreRender);
    scheduler.run_schedule(world, AppSchedule::Render);
    scheduler.run_schedule(world, AppSchedule::RenderLighting);
    scheduler.run_schedule(world, AppSchedule::PostRender);
    scheduler.run_schedule(world, AppSchedule::Present);
}

fn exit(scheduler: &mut Scheduler, world: &mut World) {
    scheduler.run_schedule(world, Schedule::Exit);
}

fn check_for_exit(world: &mut World) -> bool {
    world
        .resource_mut::<Events<AppExit>>()
        .read()
        .next()
        .is_some()
}

struct WinitApp {
    app: App,
    exit_requested: bool,
    created_window: bool,
    startup: bool,
    presented_frames: u32,
    clock: chrono::DateTime<chrono::Local>,
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
}

// impl ApplicationHandler<ControlFlowEvent> for WinitApp {
impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.created_window {
            return;
        }

        util::tracing::trace!("App resumed: Initializing");
        let window_plugin = self.app.world().resource::<WindowPlugin>();
        let window_attributes = winit::window::Window::default_attributes()
            .with_title(window_plugin.title)
            .with_inner_size(PhysicalSize::new(
                window_plugin.window_size.x,
                window_plugin.window_size.y,
            ));
        // TODO: doesn't work?
        // window.set_maximized(window_plugin.maximized);

        #[cfg(target_arch = "wasm32")]
        let mut do_startup = true;
        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::wasm_bindgen::JsCast;
            use web_sys::HtmlCanvasElement;
            use winit::platform::web::WindowAttributesExtWebSys;

            util::tracing::info!("document");
            let document = web_sys::window().unwrap().document().unwrap();
            util::tracing::info!("canvas");
            let canvas = document
                .get_element_by_id("canvas")
                .unwrap()
                .dyn_into::<HtmlCanvasElement>()
                .unwrap();

            util::tracing::info!("with canvas");
            let window_attributes = window_attributes.with_canvas(Some(canvas));

            util::tracing::info!("create_window");
            let window = event_loop.create_window(window_attributes).unwrap();
            util::tracing::info!("New window: {:?}", *window_plugin);
            let viewport = ViewPort::new(Vec2f::new(0.0, 0.0), window_plugin.viewport_size);
            let window = Window::new(Arc::new(window), viewport);

            use winit::dpi::PhysicalSize;
            if let Some(size) = window.winit_window.request_inner_size(PhysicalSize::new(
                window_plugin.window_size.x,
                window_plugin.window_size.y,
            )) {
                util::tracing::info!("requested inner window size: {size:?}");
            } else {
                do_startup = false;
                util::tracing::info!("failed to request size, awaiting resized event");
            }

            self.app.insert_resource(window);
            self.created_window = true;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let window = event_loop.create_window(window_attributes).unwrap();
            util::tracing::info!("New window: {:?}", *window_plugin);
            let viewport = ViewPort::new(Vec2f::new(0.0, 0.0), window_plugin.viewport_size);
            let window = Window::new(Arc::new(window), viewport);
            self.app.insert_resource(window);
            self.created_window = true;
        }

        #[cfg(target_arch = "wasm32")]
        if !do_startup {
            return;
        }
        startup(&mut self.app.scheduler, &mut self.app.world);
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
                    startup(&mut self.app.scheduler, &mut self.app.world);
                    self.startup = true;
                }
                self.app
                    .world
                    .insert_resource(WindowResized(size.width, size.height));
                resized(&mut self.app.scheduler, &mut self.app.world);
                self.app.world.take_resource::<WindowResized>();
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key_code) = event.physical_key {
                    self.app.world_mut().push_event(KeyInput::new(
                        KeyCode::new(key_code),
                        match event.state {
                            ElementState::Pressed => KeyState::Pressed,
                            ElementState::Released => KeyState::Released,
                        },
                        event.text.map(|s| s.to_string()),
                    ));
                }
            }
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                self.app.world_mut().push_event(MouseInput::new(
                    match button {
                        winit::event::MouseButton::Left => MouseButton::Left,
                        winit::event::MouseButton::Right => MouseButton::Right,
                        _ => unimplemented!(),
                    },
                    match state {
                        ElementState::Pressed => KeyState::Pressed,
                        ElementState::Released => KeyState::Released,
                    },
                ));
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.app
                    .world_mut()
                    .push_event(MouseMotion(position.x, position.y));
            }
            winit::event::WindowEvent::RedrawRequested => {
                // NOTE: doesn't increase the responsiveness of quick screen resizing
                // self.render();
            }
            _ => (),
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseWheel { delta } => {
                self.app.world_mut().push_event(MouseWheel(match delta {
                    event::MouseScrollDelta::LineDelta(x, y) => MouseScrollDelta::LineDelta(x, y),
                    event::MouseScrollDelta::PixelDelta(p) => {
                        MouseScrollDelta::PixelDelta(p.x as f32, p.y as f32)
                    }
                }));
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if !self.startup {
            return;
        }

        let start = chrono::Local::now();
        update(&mut self.app.scheduler, &mut self.app.world);
        let update_end = chrono::Local::now().signed_duration_since(start);

        let start = chrono::Local::now();
        render(&mut self.app.scheduler, &mut self.app.world);
        let render_end = chrono::Local::now().signed_duration_since(start);
        self.presented_frames += 1;

        if check_for_exit(&mut self.app.world) {
            self.exit_requested = true;
        }
        flush_events(&mut self.app.scheduler, &mut self.app.world);

        if chrono::Local::now().signed_duration_since(self.clock) >= chrono::TimeDelta::seconds(1) {
            let fps = self.presented_frames;
            let title = self.app.world().resource::<WindowPlugin>().title;
            let mut window = self.app.world_mut().resource_mut::<Window>();
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
            exit(&mut self.app.scheduler, &mut self.app.world);
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
