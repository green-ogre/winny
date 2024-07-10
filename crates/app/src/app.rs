#![allow(unused)]
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ecs::{prelude::*, Events, Scheduler, WinnyEvent, WinnyResource, World};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{self, DeviceEvent, DeviceId, ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

use crate::{
    plugins::{Plugin, PluginSet},
    prelude::KeyState,
};
use crate::{
    prelude::{KeyCode, KeyInput, MouseInput, WindowPlugin},
    renderer::Renderer,
    window::WinitWindow,
};

#[derive(Debug, WinnyEvent)]
pub struct AppExit;

pub struct App {
    world: World,
    scheduler: Scheduler,
    render_passes: Vec<Box<dyn RenderPass>>,
    plugins: VecDeque<Box<dyn Plugin>>,
}

impl Default for App {
    fn default() -> Self {
        let mut world = World::default();
        world.register_event::<AppExit>();
        world.insert_resource(DeltaT(0.0));

        App {
            world,
            render_passes: Vec::new(),
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
            render_passes: Vec::new(),
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

        self.scheduler.build_schedule();
        self.scheduler.init_systems(&self.world);

        let mut app = App::empty();
        std::mem::swap(self, &mut app);
        let mut win_app = WinitApp::new(app);

        let mut event_loop = EventLoop::builder();
        let event_loop = event_loop.build().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);

        let _span = util::tracing::info_span!("event_loop").entered();
        let _ = event_loop.run_app(&mut win_app);
    }

    fn startup(&mut self) {
        self.run_schedule(Schedule::StartUp);
    }

    fn update(&mut self) -> bool {
        if !update_ecs(&mut self.world, &mut self.scheduler) {
            return false;
        }
        let end = SystemTime::now();

        true
    }

    fn exit(&mut self) {
        self.run_schedule(Schedule::Exit);
    }
}

fn flush_event_queue<E: Event>(queue: EventReader<E>) {
    queue.flush();
}

pub trait RenderPass: 'static {
    fn render_pass(
        &self,
        output: &wgpu::SurfaceTexture,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        world: &World,
    );
    fn update_for_render_pass(&self, queue: &wgpu::Queue, world: &World) {}
    fn resized(&self, world: &World) {}
}

#[derive(Debug, WinnyResource)]
pub struct DeltaT(pub f64);

fn update_ecs(world: &mut World, scheduler: &mut Scheduler) -> bool {
    scheduler.run(world);
    let exit = !check_for_exit(world, scheduler);
    scheduler.flush_events(world);

    exit
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
    startup: bool,
    presented_frames: u32,
    clock: SystemTime,
}

impl WinitApp {
    pub fn new(app: App) -> Self {
        Self {
            app,
            exit_requested: false,
            startup: false,
            presented_frames: 0,
            clock: SystemTime::now(),
        }
    }

    pub fn update(&mut self) {
        self.exit_requested = !self.app.update();
    }

    pub fn render(&mut self) {
        let world = unsafe { self.app.world().as_unsafe_world().read_only() };
        self.app.world().resource_mut::<Renderer>().render(world);
    }
}

// impl ApplicationHandler<ControlFlowEvent> for WinitApp {
impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
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
        let window = WinitWindow(Arc::new(window));
        let renderer = Renderer::new(
            Arc::clone(&window),
            self.app.render_passes.drain(..).collect(),
        );

        self.app.insert_resource(renderer).insert_resource(window);
        self.app.run_schedule(Schedule::StartUp);
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
                let world = unsafe { self.app.world().as_unsafe_world().read_only() };
                let mut renderer = self.app.world().resource_mut::<Renderer>();
                renderer.resize(world, size.width, size.height);
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } => self
                .app
                .insert_winit_event(WinitEvent::KeyboardInput(event)),
            winit::event::WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.exit_requested {
            event_loop.exit();
        }

        if !self.startup {
            return;
        }

        let start = SystemTime::now();
        self.update();
        let update_end = SystemTime::now().duration_since(start).unwrap_or_default();
        let start = SystemTime::now();
        self.render();
        let render_end = SystemTime::now().duration_since(start).unwrap_or_default();
        self.presented_frames += 1;

        if SystemTime::now()
            .duration_since(self.clock)
            .unwrap_or_default()
            >= Duration::from_secs(1)
        {
            let fps = self.presented_frames;
            let title = self.app.world().resource::<WindowPlugin>().title;
            let window = self.app.world().resource::<WinitWindow>();
            window.set_title(
                format!(
                    "{} - {} - {}ms - {}ms",
                    title,
                    fps,
                    update_end.as_millis(),
                    render_end.as_millis()
                )
                .as_str(),
            );
            self.presented_frames = 0;
            self.clock = SystemTime::now();
        }
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.app.exit();
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
