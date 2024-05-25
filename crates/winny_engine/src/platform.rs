use std::{
    collections::VecDeque,
    sync::mpsc::{channel, Receiver, Sender},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ecs::{EventWriter, ResMut, Schedule, Scheduler, WinnyEvent, WinnyResource, World};
use gfx::{
    gui::{begin_frame, EguiRenderer},
    render, update_sprite_data, DeltaT, Renderer,
};
use gilrs::{EventType, Gilrs};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceEvent, ElementState, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::WindowBuilder,
};

use logger::*;

use crate::window::WindowPlugin;

// TODO: controlled frame rate
// fn stall_untill_next_frame(mut perf: ResMut<PerfCounter>) {
//     while !perf.should_advance() {}
// }

pub async fn game_loop(mut world: World, mut scheduler: Scheduler) {
    let event_loop = EventLoop::new().unwrap();

    let window_plugin = world.resource::<WindowPlugin>();
    let close_on_escape = window_plugin.close_on_escape;
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(
            window_plugin.inner_size.0,
            window_plugin.inner_size.1,
        ))
        .with_position(PhysicalPosition::new(
            window_plugin.position.0,
            window_plugin.position.1,
        ))
        .build(&event_loop)
        .unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let renderer = Renderer::new(
        window,
        [window_plugin.inner_size.0, window_plugin.inner_size.1],
        [window_plugin.virtual_size.0, window_plugin.virtual_size.1],
    )
    .await;
    let egui_renderer = EguiRenderer::new(
        &renderer.device,
        renderer.config.format,
        1,
        &renderer.window,
    );
    world.insert_resource(renderer);
    world.insert_resource(egui_renderer);

    world.insert_resource(DeltaT(0.0));
    world.register_event::<KeyInput>();
    world.register_event::<MouseInput>();
    world.register_event::<ControllerInput>();
    world.insert_resource(ControllerAxisState::new());

    scheduler.add_systems(ecs::Schedule::PreUpdate, begin_frame);
    scheduler.add_systems(ecs::Schedule::PostUpdate, update_sprite_data);
    scheduler.add_systems(ecs::Schedule::Render, render);

    scheduler.startup(&world);

    // let target_fps = Some(60.0);
    let target_fps: Option<f64> = None;
    let target_frame_len = target_fps.map(|target| 1.0 / target);
    let mut perf = PerfCounter::new(target_frame_len);

    let (controller_input_sender, controller_input_reciever) =
        channel::<(ControllerInput, ControllerAxisState)>();
    create_controller_thread(controller_input_sender);

    let (winit_window_event_tx, winit_window_event_rx) = channel();
    let (winit_device_event_tx, winit_device_event_rx) = channel::<DeviceEvent>();
    // This is necessary because exiting the winit event_loop will exit the program, so a message
    // is sent to the event_loop when the game_loop has finished exiting
    let (winit_exit_tx, winit_exit_rx) = channel();

    // TODO: fix this shit
    let mut last_mouse_position: (f64, f64) = (0.0, 0.0);
    let mut last_held_key: Option<MouseButton> = None;

    // This is the main game loop
    std::thread::spawn(move || loop {
        perf.start();

        let renderer = unsafe {
            world
                .as_unsafe_world()
                .read_and_write()
                .resource_mut::<Renderer>()
        };

        let mut egui_renderer = unsafe {
            world
                .as_unsafe_world()
                .read_and_write()
                .resource_mut::<EguiRenderer>()
        };

        perf.start_debug_event();
        let mut exit_loop = false;
        for event in winit_window_event_rx.try_iter() {
            if let Some(response) = egui_renderer.handle_input(&renderer.window, Some(&event), None)
            {
                if response.consumed {
                    continue;
                }
            }

            match event {
                WindowEvent::CursorMoved { position, .. } => {
                    let mut user_input = unsafe { EventWriter::new(world.as_unsafe_world()) };

                    user_input.send(MouseInput::new(
                        0.0,
                        0.0,
                        position.x,
                        position.y,
                        last_held_key,
                    ));
                    last_mouse_position = (position.x, position.y);
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let mut user_input = unsafe { EventWriter::new(world.as_unsafe_world()) };

                    last_held_key = if state == ElementState::Pressed {
                        match button {
                            winit::event::MouseButton::Left => Some(MouseButton::Left),
                            winit::event::MouseButton::Right => Some(MouseButton::Right),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    user_input.send(MouseInput::new(
                        0.0,
                        0.0,
                        last_mouse_position.0,
                        last_mouse_position.1,
                        last_held_key,
                    ));
                }
                WindowEvent::Resized(new_size) => {
                    let mut renderer = world.resource_mut::<Renderer>();
                    renderer.resize(new_size.into());
                }
                WindowEvent::CloseRequested => {
                    exit_game(&perf, &mut world, &mut scheduler);
                    winit_exit_tx.send(()).unwrap();
                    exit_loop = true;
                    break;
                }
                WindowEvent::KeyboardInput {
                    event: key_event, ..
                } => {
                    if close_on_escape {
                        if key_event.physical_key
                            == PhysicalKey::Code(winit::keyboard::KeyCode::Escape)
                        {
                            exit_game(&perf, &mut world, &mut scheduler);
                            winit_exit_tx.send(()).unwrap();
                            exit_loop = true;
                            break;
                        }
                    }

                    let mut user_input = unsafe { EventWriter::new(world.as_unsafe_world()) };

                    if let PhysicalKey::Code(key_code) = key_event.physical_key {
                        user_input.send(KeyInput::new(
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
        }

        if exit_loop {
            break;
        }

        for event in winit_device_event_rx.try_iter() {
            match event {
                DeviceEvent::MouseMotion { delta } => {
                    let mut user_input = unsafe { EventWriter::new(world.as_unsafe_world()) };

                    user_input.send(MouseInput::new(
                        delta.0,
                        delta.1,
                        last_mouse_position.0,
                        last_mouse_position.1,
                        last_held_key,
                    ));

                    egui_renderer.handle_input(&renderer.window, None, Some(delta));
                }
                _ => (),
            }
        }
        perf.stop_debug_event();
        perf.log_last_debug_event("Platform: Handle Winit events");

        update_and_render(
            &mut perf,
            &mut world,
            &mut scheduler,
            &controller_input_reciever,
        );

        perf.stop();
    });

    // Pipe these events into the update and render thread
    let _ = event_loop.run(move |event, elwt| match event {
        winit::event::Event::WindowEvent { event, .. } => {
            if let Ok(_) = winit_exit_rx.try_recv() {
                elwt.exit();
            } else {
                let _ = winit_window_event_tx.send(event.clone());
            }
        }
        winit::event::Event::DeviceEvent { event, .. } => {
            winit_device_event_tx.send(event).unwrap();
        }
        _ => (),
    });
}

fn create_controller_thread(
    controller_input_sender: Sender<(ControllerInput, ControllerAxisState)>,
) {
    std::thread::spawn(move || {
        let mut gilrs = Gilrs::new().unwrap();

        // for (_id, gamepad) in gilrs.gamepads() {
        //     info!("{} is {:?}", gamepad.name(), gamepad.power_info());
        // }

        let mut controller_axis_state = ControllerAxisState::new();

        // Examine new events
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

fn update_and_render(
    perf: &mut PerfCounter,
    world: &mut World,
    scheduler: &mut Scheduler,
    controller_input_reciever: &Receiver<(ControllerInput, ControllerAxisState)>,
) {
    perf.start_debug_event();
    // Pipe window input into World
    {
        // SAFETY:
        //
        // The EventWriter and ResMut have mutually exclusive, mutable access to different
        // memory and are dropped before the scheduler runs. Nobody can access this memory
        // at the same time.
        let mut controller_event = unsafe { EventWriter::new(world.as_unsafe_world()) };
        let mut controller_axis_state = unsafe { ResMut::new(world.as_unsafe_world()) };
        for (input, axis_state) in controller_input_reciever.try_iter() {
            controller_event.send(input);
            *controller_axis_state = axis_state;
        }
    }
    perf.stop_debug_event();
    perf.log_last_debug_event("ECS: Pipe input into world");

    // Insert last frame time
    {
        let mut dt = unsafe { ResMut::new(world.as_unsafe_world()) };
        *dt = DeltaT(perf.last_frame_duration().unwrap_or_default().as_secs_f64());
    }

    perf.start_debug_event();
    scheduler.run_schedule(Schedule::PreUpdate, world);
    perf.stop_debug_event();
    perf.log_last_debug_event("ECS: PreUpdate");

    perf.start_debug_event();
    scheduler.run_schedule(Schedule::Update, world);
    perf.stop_debug_event();
    perf.log_last_debug_event("ECS: Update");

    perf.start_debug_event();
    scheduler.run_schedule(Schedule::PostUpdate, world);
    perf.stop_debug_event();
    perf.log_last_debug_event("ECS: PostUpdate");

    perf.start_debug_event();
    scheduler.run_schedule(Schedule::Render, world);
    perf.stop_debug_event();
    perf.log_last_debug_event("ECS: Render");

    world.flush_events();
}

pub fn exit_game(perf: &PerfCounter, world: &World, scheduler: &mut Scheduler) {
    scheduler.exit(world);
    perf.exit_stats();
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum KeyCode {
    Unknown,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Space,
    Shift,
}

impl KeyCode {
    pub fn new(code: winit::keyboard::KeyCode) -> Self {
        match code {
            winit::keyboard::KeyCode::KeyA => KeyCode::A,
            winit::keyboard::KeyCode::KeyB => KeyCode::B,
            winit::keyboard::KeyCode::KeyC => KeyCode::C,
            winit::keyboard::KeyCode::KeyD => KeyCode::D,
            winit::keyboard::KeyCode::KeyE => KeyCode::E,
            winit::keyboard::KeyCode::KeyF => KeyCode::F,
            winit::keyboard::KeyCode::KeyG => KeyCode::G,
            winit::keyboard::KeyCode::KeyH => KeyCode::H,
            winit::keyboard::KeyCode::KeyI => KeyCode::I,
            winit::keyboard::KeyCode::KeyJ => KeyCode::J,
            winit::keyboard::KeyCode::KeyK => KeyCode::K,
            winit::keyboard::KeyCode::KeyL => KeyCode::L,
            winit::keyboard::KeyCode::KeyM => KeyCode::M,
            winit::keyboard::KeyCode::KeyN => KeyCode::N,
            winit::keyboard::KeyCode::KeyO => KeyCode::O,
            winit::keyboard::KeyCode::KeyP => KeyCode::P,
            winit::keyboard::KeyCode::KeyQ => KeyCode::Q,
            winit::keyboard::KeyCode::KeyR => KeyCode::R,
            winit::keyboard::KeyCode::KeyS => KeyCode::S,
            winit::keyboard::KeyCode::KeyT => KeyCode::T,
            winit::keyboard::KeyCode::KeyU => KeyCode::U,
            winit::keyboard::KeyCode::KeyV => KeyCode::V,
            winit::keyboard::KeyCode::KeyW => KeyCode::W,
            winit::keyboard::KeyCode::KeyX => KeyCode::X,
            winit::keyboard::KeyCode::KeyY => KeyCode::Y,
            winit::keyboard::KeyCode::KeyZ => KeyCode::Z,
            winit::keyboard::KeyCode::Space => KeyCode::Space,
            winit::keyboard::KeyCode::ShiftLeft => KeyCode::Shift,
            winit::keyboard::KeyCode::ShiftRight => KeyCode::Shift,
            _ => KeyCode::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, WinnyEvent)]
pub struct KeyInput {
    pub code: KeyCode,
    pub state: KeyState,
}

impl KeyInput {
    pub fn new(code: KeyCode, state: KeyState) -> Self {
        Self { code, state }
    }
}

#[derive(Debug, Clone, Copy, WinnyEvent)]
pub struct MouseInput {
    pub dx: f64,
    pub dy: f64,
    pub x: f64,
    pub y: f64,
    pub button_pressed: Option<MouseButton>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
}

impl MouseInput {
    pub fn new(dx: f64, dy: f64, x: f64, y: f64, button: Option<MouseButton>) -> Self {
        Self {
            dx,
            dy,
            x,
            y,
            button_pressed: button,
        }
    }
}

#[derive(Debug)]
pub enum ControllerInputState {
    ButtonPressed(Button),
    ButtonRepeated(Button),
    ButtonReleased(Button),
    AxisChanged(Axis, f32),
    Connected,
    Disconnected,
    Unknown,
}

#[derive(Debug, WinnyEvent)]
pub struct ControllerInput {
    pub input: ControllerInputState,
}

impl ControllerInput {
    pub fn new(input: ControllerInputState) -> Self {
        Self { input }
    }
}

impl ControllerInputState {
    pub fn from(value: EventType) -> Self {
        match value {
            EventType::ButtonPressed(b, _) => ControllerInputState::ButtonPressed(Button::from(b)),
            EventType::ButtonRepeated(b, _) => {
                ControllerInputState::ButtonRepeated(Button::from(b))
            }
            EventType::ButtonReleased(b, _) => {
                ControllerInputState::ButtonReleased(Button::from(b))
            }
            EventType::AxisChanged(a, f, _) => ControllerInputState::AxisChanged(Axis::from(a), f),
            EventType::Connected => ControllerInputState::Connected,
            EventType::Disconnected => ControllerInputState::Disconnected,
            _ => ControllerInputState::Unknown,
        }
    }

    pub fn axis_state(&self) -> Option<AxisState> {
        match self {
            Self::AxisChanged(axis, f) => Some(AxisState::new(*axis, *f)),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Button {
    A,
    X,
    Y,
    B,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    LeftStick,
    RigthStick,
    LeftTrigger,
    LeftBumper,
    RightTrigger,
    RightBumper,
    Unknown,
}

impl Button {
    pub fn from(value: gilrs::Button) -> Self {
        match value {
            gilrs::Button::DPadUp => Button::DPadUp,
            gilrs::Button::DPadDown => Button::DPadDown,
            gilrs::Button::DPadLeft => Button::DPadLeft,
            gilrs::Button::DPadRight => Button::DPadRight,
            gilrs::Button::South => Button::A,
            gilrs::Button::West => Button::X,
            gilrs::Button::North => Button::Y,
            gilrs::Button::East => Button::B,
            gilrs::Button::LeftThumb => Button::LeftStick,
            gilrs::Button::RightThumb => Button::RigthStick,
            gilrs::Button::RightTrigger2 => Button::RightTrigger,
            gilrs::Button::RightTrigger => Button::RightBumper,
            gilrs::Button::LeftTrigger2 => Button::LeftTrigger,
            gilrs::Button::LeftTrigger => Button::LeftBumper,
            _ => Button::Unknown,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Axis {
    LeftStickX,
    LeftStickY,
    LeftZ,
    RightStickX,
    RightStickY,
    RightZ,
    Unknown,
}

impl Axis {
    pub fn from(axis: gilrs::Axis) -> Self {
        match axis {
            gilrs::Axis::Unknown => Self::Unknown,
            gilrs::Axis::LeftStickX => Self::LeftStickX,
            gilrs::Axis::LeftStickY => Self::LeftStickY,
            gilrs::Axis::LeftZ => Self::LeftZ,
            gilrs::Axis::RightStickX => Self::RightStickX,
            gilrs::Axis::RightStickY => Self::RightStickY,
            gilrs::Axis::RightZ => Self::RightZ,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AxisState {
    pub axis: Axis,
    pub value: f32,
}

impl AxisState {
    pub fn new(axis: Axis, value: f32) -> Self {
        Self { axis, value }
    }
}

#[derive(Debug, WinnyResource, Clone, Copy)]
pub struct ControllerAxisState {
    left_stick_x: AxisState,
    left_stick_y: AxisState,
    left_z: AxisState,
    right_stick_x: AxisState,
    right_stick_y: AxisState,
    right_z: AxisState,
}

impl ControllerAxisState {
    pub fn new() -> Self {
        Self {
            left_stick_x: AxisState::new(Axis::LeftStickX, 0.0),
            left_stick_y: AxisState::new(Axis::LeftStickY, 0.0),
            left_z: AxisState::new(Axis::LeftZ, 0.0),
            right_stick_x: AxisState::new(Axis::RightStickX, 0.0),
            right_stick_y: AxisState::new(Axis::RightStickY, 0.0),
            right_z: AxisState::new(Axis::RightZ, 0.0),
        }
    }

    pub fn iter_non_zero(self) -> impl Iterator<Item = AxisState> {
        let mut state_vec = vec![];
        if self.left_stick_x.value != 0.0 {
            state_vec.push(self.left_stick_x);
        }
        if self.left_stick_y.value != 0.0 {
            state_vec.push(self.left_stick_y);
        }
        if self.left_z.value != 0.0 {
            state_vec.push(self.left_z);
        }
        if self.right_stick_x.value != 0.0 {
            state_vec.push(self.right_stick_x);
        }
        if self.right_stick_y.value != 0.0 {
            state_vec.push(self.right_stick_y);
        }
        if self.right_z.value != 0.0 {
            state_vec.push(self.right_z);
        }

        state_vec.into_iter()
    }

    pub fn apply_new_state(&mut self, new_state: AxisState) {
        match new_state.axis {
            Axis::Unknown => error!("unknown stick state"),
            Axis::LeftStickX => self.left_stick_x.value = new_state.value,
            Axis::LeftStickY => self.left_stick_y.value = new_state.value,
            Axis::LeftZ => self.left_z.value = new_state.value,
            Axis::RightStickX => self.right_stick_x.value = new_state.value,
            Axis::RightStickY => self.right_stick_y.value = new_state.value,
            Axis::RightZ => self.right_z.value = new_state.value,
        }
    }
}

#[derive(Debug, WinnyResource)]
pub struct PerfCounter {
    begin: Option<SystemTime>,
    begin_debug_event: Option<SystemTime>,
    end: Option<SystemTime>,
    end_debug_event: Option<SystemTime>,
    last_fram_duration: Option<Duration>,
    frames: usize,
    total_frames: usize,
    lost_frames: usize,
    lost_frames_sum: usize,
    highest_lost_frames: usize,
    frames_sum: f64,
    iterations: usize,
    target_frame_len: Option<f64>,
    duration: f64,
    start_of_second: Duration,
    debug_events: VecDeque<(String, Duration)>,
}

impl PerfCounter {
    pub fn new(target_frame_len: Option<f64>) -> Self {
        Self {
            begin: None,
            begin_debug_event: None,
            end: None,
            end_debug_event: None,
            last_fram_duration: None,
            frames: 0,
            total_frames: 0,
            lost_frames: 0,
            lost_frames_sum: 0,
            highest_lost_frames: 0,
            frames_sum: 0.0,
            iterations: 0,
            target_frame_len,
            duration: 0.0,
            start_of_second: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is a construct"),
            debug_events: VecDeque::new(),
        }
    }

    pub fn last_frame_duration(&self) -> Option<Duration> {
        self.last_fram_duration
    }

    pub fn start(&mut self) {
        self.begin = Some(SystemTime::now());
    }

    pub fn start_debug_event(&mut self) {
        self.begin_debug_event = Some(SystemTime::now());
    }

    pub fn current_frame_len(&self) -> Result<Duration, std::time::SystemTimeError> {
        Ok(SystemTime::now().duration_since(self.begin.unwrap())?)
    }

    pub fn should_advance(&self) -> bool {
        self.target_frame_len.is_none()
            || self
                .current_frame_len()
                .map(|dur| dur.as_secs_f64())
                .unwrap_or_default()
                >= self.target_frame_len.unwrap()
    }

    pub fn stop(&mut self) {
        self.end = Some(SystemTime::now());

        // info!(
        //     "> Measured Frame Length: {},\tTarget Frame Length: {},\tLoss: {}",
        //     self.current_frame_len().unwrap_or_default().as_secs_f64(),
        //     self.target_frame_len.unwrap_or_default(),
        //     (self.current_frame_len().unwrap_or_default().as_secs_f64()
        //         - self.target_frame_len.unwrap_or_default())
        //     .abs()
        // );
        self.frames_sum += self.current_frame_len().unwrap_or_default().as_secs_f64();

        self.frames += 1;

        self.last_fram_duration = Some(self.current_frame_len().unwrap_or_default());

        self.duration = self
            .end
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .expect("time is a construct")
            .as_secs_f64()
            - self.start_of_second.as_secs_f64();

        if self.duration >= 1.0 {
            for (label, duration) in self.debug_events.drain(..) {
                info!("{} => {}", label, duration.as_secs_f32());
            }

            self.start_of_second = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is a construct");
            self.total_frames += self.frames;

            info!(
                "# Frames {},\tDuration: {},\tExpected {} Frames: {},\tLost Frames: {}",
                self.frames, self.duration, self.frames, self.frames_sum, self.lost_frames
            );

            if self.lost_frames > self.highest_lost_frames {
                self.highest_lost_frames = self.lost_frames;
            }
            self.frames = 0;
            self.lost_frames = 0;
            self.frames_sum = 0.0;
            self.iterations += 1;
        }

        self.debug_events.drain(..);
    }

    pub fn stop_debug_event(&mut self) {
        self.end_debug_event = Some(SystemTime::now());
    }

    pub fn log_last_debug_event(&mut self, label: &str) {
        if let Some(duration) = self.query_last_debug_event() {
            self.debug_events.push_back((label.into(), duration));
        }
    }

    pub fn query_last_debug_event(&self) -> Option<Duration> {
        if let Some(start) = self.begin_debug_event {
            if let Some(end) = self.end_debug_event {
                let dur = end.duration_since(start);
                if dur.is_ok() {
                    return Some(dur.unwrap());
                } else {
                    return None;
                }
            }
        }

        None
    }

    pub fn exit_stats(&self) {
        info!(
            ">> Iterations: {},\tFPS: {},\tTotal Lost Frames: {},\tAverage: {},\tHigh:{}",
            self.iterations,
            self.total_frames / self.iterations.max(1),
            self.lost_frames_sum,
            self.lost_frames_sum / self.iterations.max(1),
            self.highest_lost_frames
        );
    }
}
