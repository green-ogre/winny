use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};

use app::{
    app::{App, AppExit},
    plugins::Plugin,
};
use ecs::{EventWriter, IntoSystemStorage, Res, ResMut, WinnyResource};
use gfx::renderer::{Renderer, RendererPlugin};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceEvent, ElementState, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::WindowBuilder,
};

pub mod prelude;

#[cfg(feature = "egui")]
use gfx::gui::EguiRenderer;

#[cfg(feature = "controller")]
use gilrs::Gilrs;

pub mod input;
#[cfg(feature = "controller")]
pub use input::controller::*;
pub use input::mouse_and_key::*;

#[derive(Debug, WinnyResource, Clone, Copy)]
pub struct WindowPlugin {
    pub inner_size: (u32, u32),
    pub virtual_size: (u32, u32),
    pub position: (u32, u32),
    pub close_on_escape: bool,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        Self {
            inner_size: (1920, 1080),
            virtual_size: (1920, 1080),
            position: (10, 10),
            close_on_escape: false,
        }
    }
}

impl Plugin for WindowPlugin {
    fn build(&mut self, app: &mut App) {
        app.insert_resource(self.clone());

        app.register_event::<KeyInput>();
        app.register_event::<MouseInput>();
        #[cfg(feature = "controller")]
        app.register_event::<ControllerInput>();
        #[cfg(feature = "controller")]
        app.insert_resource(ControllerAxisState::new());
        app.insert_resource(MouseState::default());

        app.add_systems(
            ecs::Schedule::Render,
            (
                gfx::sprite::bind_new_sprite_bundles,
                gfx::renderer::update_sprite_data,
                gfx::renderer::render,
            )
                .chain(),
        );

        app.add_systems(
            ecs::Schedule::Platform,
            (
                (
                    pipe_winit_events,
                    #[cfg(feature = "controller")]
                    pipe_controller_input,
                    handle_winit_events,
                ),
                gfx::renderer::create_context,
            )
                .chain(),
        );

        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(self.inner_size.0, self.inner_size.1))
            .with_position(PhysicalPosition::new(self.position.0, self.position.1))
            .build(&event_loop)
            .unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);

        #[cfg(feature = "controller")]
        let (controller_input_sender, controller_input_reciever) =
            channel::<(ControllerInput, ControllerAxisState)>();
        #[cfg(feature = "controller")]
        spawn_controller_thread(controller_input_sender);

        let (wwetx, wwerx) = channel();
        let (wdetx, wderx) = channel::<DeviceEvent>();
        let (wetx, werx) = channel::<()>();

        #[cfg(feature = "controller")]
        let channels = WindowChannels::new(controller_input_reciever, wwerx, wderx, wetx);
        #[cfg(not(feature = "controller"))]
        let channels = WindowChannels::new(wwerx, wderx, wetx);

        app.insert_resource(channels)
            .add_plugins(RendererPlugin::new(
                window,
                self.inner_size,
                self.virtual_size,
            ))
            .set_window_callback(Box::new(|| {
                let _ = event_loop.run(move |event, elwt| match event {
                    winit::event::Event::WindowEvent { event, .. } => {
                        if let Ok(_) = werx.try_recv() {
                            logger::info!("Exiting");
                            elwt.exit();
                        } else {
                            let _ = wwetx.send(event.clone());
                        }
                    }
                    winit::event::Event::DeviceEvent { event, .. } => {
                        wdetx.send(event).unwrap();
                    }
                    _ => (),
                });
            }));
    }
}

#[derive(Debug, WinnyResource)]
struct WindowChannels {
    #[cfg(feature = "controller")]
    pub cirx: Receiver<(ControllerInput, ControllerAxisState)>,
    pub wwerx: Receiver<WindowEvent>,
    pub wderx: Receiver<DeviceEvent>,
    pub wetx: Sender<()>,
}

unsafe impl Send for WindowChannels {}
unsafe impl Sync for WindowChannels {}

impl WindowChannels {
    #[cfg(feature = "controller")]
    pub fn new(
        controller_input_reciever: Receiver<(ControllerInput, ControllerAxisState)>,
        winit_window_event_rx: Receiver<WindowEvent>,
        winit_device_event_rx: Receiver<DeviceEvent>,
        winit_exit_tx: Sender<()>,
    ) -> Self {
        Self {
            cirx: controller_input_reciever.into(),
            wwerx: winit_window_event_rx.into(),
            wderx: winit_device_event_rx.into(),
            wetx: winit_exit_tx.into(),
        }
    }

    #[cfg(not(feature = "controller"))]
    pub fn new(
        winit_window_event_rx: Receiver<WindowEvent>,
        winit_device_event_rx: Receiver<DeviceEvent>,
        winit_exit_tx: Sender<()>,
    ) -> Self {
        Self {
            wwerx: winit_window_event_rx.into(),
            wderx: winit_device_event_rx.into(),
            wetx: winit_exit_tx.into(),
        }
    }
}

fn pipe_winit_events(
    channels: Res<WindowChannels>,
    mut user_input: EventWriter<MouseInput>,
    mouse_state: Res<MouseState>,
) {
    for event in channels.wderx.try_iter() {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                user_input.send(MouseInput::new(
                    delta.0,
                    delta.1,
                    mouse_state.last_mouse_position.0,
                    mouse_state.last_mouse_position.1,
                    None,
                    mouse_state.last_held_key,
                ));
            }
            _ => (),
        }
    }
}

#[cfg(feature = "egui")]
fn handle_winit_events(
    mut egui_renderer: Option<ResMut<EguiRenderer>>,
    mut renderer: ResMut<Renderer>,
    mut window_plugin: ResMut<WindowPlugin>,
    mut user_key_input: EventWriter<KeyInput>,
    mut user_mouse_input: EventWriter<MouseInput>,
    mut mouse_state: ResMut<MouseState>,
    mut app_exit: EventWriter<AppExit>,
    channels: Res<WindowChannels>,
) {
    for event in channels.wwerx.try_iter() {
        if let Some(egui_renderer) = egui_renderer.as_mut() {
            if let Some(response) = egui_renderer.handle_input(&renderer.window, Some(&event), None)
            {
                if response.consumed {
                    continue;
                }
            }
        }

        inner_handle_winit_events(
            &mut renderer,
            &mut window_plugin,
            &mut user_key_input,
            &mut user_mouse_input,
            &mut mouse_state,
            &channels,
            event,
            &mut app_exit,
        );
    }
}

#[cfg(not(feature = "egui"))]
fn handle_winit_events(
    mut renderer: ResMut<Renderer>,
    mut window_plugin: ResMut<WindowPlugin>,
    mut user_key_input: EventWriter<KeyInput>,
    mut user_mouse_input: EventWriter<MouseInput>,
    mut mouse_state: ResMut<MouseState>,
    mut app_exit: EventWriter<AppExit>,
    channels: Res<WindowChannels>,
) {
    for event in channels.wwerx.try_iter() {
        inner_handle_winit_events(
            &mut renderer,
            &mut window_plugin,
            &mut user_key_input,
            &mut user_mouse_input,
            &mut mouse_state,
            &channels,
            event,
            &mut app_exit,
        );
    }
}

fn inner_handle_winit_events(
    renderer: &mut Renderer,
    window_plugin: &mut WindowPlugin,
    user_key_input: &mut EventWriter<KeyInput>,
    user_mouse_input: &mut EventWriter<MouseInput>,
    mouse_state: &mut MouseState,
    channels: &WindowChannels,
    event: WindowEvent,
    app_exit: &mut EventWriter<AppExit>,
) -> bool {
    match event {
        WindowEvent::CursorMoved { position, .. } => {
            user_mouse_input.send(MouseInput::new(
                0.0,
                0.0,
                position.x,
                position.y,
                None,
                mouse_state.last_held_key,
            ));
            mouse_state.last_mouse_position = (position.x, position.y);
        }
        WindowEvent::MouseInput { state, button, .. } => {
            let mut held = None;

            mouse_state.last_held_key = if state == ElementState::Pressed {
                let button = match button {
                    winit::event::MouseButton::Left => Some(MouseButton::Left),
                    winit::event::MouseButton::Right => Some(MouseButton::Right),
                    _ => None,
                };

                if button == mouse_state.last_held_key {
                    held = button;
                    None
                } else {
                    button
                }
            } else {
                None
            };

            user_mouse_input.send(MouseInput::new(
                0.0,
                0.0,
                mouse_state.last_mouse_position.0,
                mouse_state.last_mouse_position.1,
                mouse_state.last_held_key,
                held,
            ));
        }
        WindowEvent::Resized(new_size) => {
            renderer.resize(new_size.into());
        }
        WindowEvent::CloseRequested => {
            channels.wetx.send(()).unwrap();
            app_exit.send(AppExit);
            return false;
        }
        WindowEvent::KeyboardInput {
            event: key_event, ..
        } => {
            if window_plugin.close_on_escape {
                if key_event.physical_key == PhysicalKey::Code(winit::keyboard::KeyCode::Escape) {
                    channels.wetx.send(()).unwrap();
                    app_exit.send(AppExit);
                    return false;
                }
            }

            if let PhysicalKey::Code(key_code) = key_event.physical_key {
                user_key_input.send(KeyInput::new(
                    KeyCode::new(key_code),
                    match key_event.state {
                        ElementState::Pressed => KeyState::Pressed,
                        ElementState::Released => KeyState::Released,
                    },
                ));
            }
        }
        _ => {}
    }

    true
}

#[cfg(feature = "controller")]
fn spawn_controller_thread(
    controller_input_sender: Sender<(ControllerInput, ControllerAxisState)>,
) {
    use logger::error;

    std::thread::spawn(move || {
        let mut gilrs = Gilrs::new().unwrap();
        let mut controller_axis_state = ControllerAxisState::new();

        loop {
            while let Some(gilrs::Event { event, .. }) = gilrs.next_event() {
                let input = ControllerInputState::from(event);

                if let Some(new_axis_state) = input.axis_state() {
                    controller_axis_state.apply_new_state(new_axis_state);
                }

                if controller_input_sender
                    .send((ControllerInput::new(input), controller_axis_state))
                    .is_err()
                {
                    error!("Error sending controller input");
                }
            }
        }
    });
}

#[cfg(feature = "controller")]
fn pipe_controller_input(
    channels: Res<WindowChannels>,
    mut controller_event: EventWriter<ControllerInput>,
    mut controller_axis_state: ResMut<ControllerAxisState>,
) {
    for (input, axis_state) in channels.cirx.try_iter() {
        controller_event.send(input);
        *controller_axis_state = axis_state;
    }
}
