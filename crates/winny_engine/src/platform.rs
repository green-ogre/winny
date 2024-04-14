use core::time;
use std::{
    env,
    error::Error,
    ffi::OsString,
    io::Read,
    marker::PhantomData,
    sync::mpsc::channel,
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ecs::{Event, EventWriter, ResMut, Resource, Scheduler, TypeGetter, World};
use gilrs::{EventType, Gilrs};
use log::{debug, error, info, trace};
use winit::{
    event::{ElementState, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowBuilder},
};

struct State<'w> {
    surface: wgpu::Surface<'w>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
}

impl<'w> State<'w> {
    async fn new(window: &'w Window) -> Self {
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            // magic numbers
            desired_maximum_frame_latency: 2,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[1],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        info!("{:?}", surface_caps.present_modes);
        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            size,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn update(&mut self) {
        // todo!()
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub async fn game_loop(
    scheduler: &mut Scheduler,
    world: &mut World,
    title: Option<String>,
    target_fps: Option<f64>,
) {
    // For wgpu
    env_logger::init();

    world.register_event::<KeyInput>();
    world.register_event::<ControllerInput>();
    world.insert_resource(ControllerAxisState::new());

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title(title.unwrap_or("winny".into()))
        .build(&event_loop)
        .unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut state = State::new(&window).await;

    scheduler.startup(world);

    let target_frame_len = target_fps.map(|target| Some(1.0 / target)).unwrap_or(None);
    let mut perf = PerfCounter::new(target_frame_len);

    let (controller_input_sender, controller_input_reciever) =
        channel::<(ControllerInput, ControllerAxisState)>();

    // handle controller input
    std::thread::spawn(move || {
        let mut gilrs = Gilrs::new().unwrap();

        // Iterate over all connected gamepads
        for (_id, gamepad) in gilrs.gamepads() {
            info!("{} is {:?}", gamepad.name(), gamepad.power_info());
        }

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

    let _ = event_loop.run(move |event, elwt| match event {
        winit::event::Event::WindowEvent {
            event: WindowEvent::Resized(new_size),
            ..
        } => {
            state.resize(new_size);
        }
        winit::event::Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            scheduler.exit(world);

            perf.exit_stats();
            elwt.exit();
        }
        winit::event::Event::AboutToWait => {
            perf.start();

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
                    // info!("Controller Input: {:#?}", input);
                    controller_event.send(input);
                    *controller_axis_state.as_mut() = axis_state;
                }
            }

            perf.start_debug_event();

            // Update World
            scheduler.run(world);
            world.flush_events();

            perf.stop_debug_event();
            debug!(
                "Update World: {}ms",
                perf.query_last_debug_event()
                    .unwrap_or_default()
                    .as_millis()
            );

            perf.start_debug_event();

            // Render
            state.update();
            match state.render() {
                Ok(_) => {}
                // Reconfigure the surface if lost
                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                // The system is out of memory, we should probably quit
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    scheduler.exit(world);

                    perf.exit_stats();
                    elwt.exit();
                }
                // All other errors (Outdated, Timeout) should be resolved by the next frame
                Err(e) => error!("{:?}", e),
            }

            perf.stop_debug_event();
            debug!(
                "Render: {}ms",
                perf.query_last_debug_event()
                    .unwrap_or_default()
                    .as_millis()
            );

            while !perf.should_advance() {}
            perf.stop();
        }
        winit::event::Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } => {
            let mut user_input = unsafe { EventWriter::new(world.as_unsafe_world()) };

            if let PhysicalKey::Code(key_code) = event.physical_key {
                user_input.send(KeyInput::new(
                    KeyCode::new(key_code),
                    match event.state {
                        ElementState::Pressed => KeyState::Pressed,
                        ElementState::Released => KeyState::Released,
                    },
                ));
            }
        }
        _ => (),
    });
}

fn read_file(path: &String) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut f = std::fs::File::open(path)?;
    let metadata = f.metadata()?;
    let mut buf = vec![0u8; metadata.len() as usize];
    f.read(&mut buf)?;

    Ok(buf)
}

pub struct InputBuffer {
    buf: [Option<KeyInput>; 10],
    index: u8,
    len: u8,
}

impl Default for InputBuffer {
    fn default() -> Self {
        InputBuffer {
            buf: [None; 10],
            index: 0,
            len: 10,
        }
    }
}

impl InputBuffer {
    pub fn push(&mut self, e: KeyInput) {
        if self.index < self.len - 1 {
            self.buf[(self.index + 1) as usize] = Some(e);
            self.index += 1;
        } else {
            panic!("Need a bigger input buffer");
        }
    }

    pub fn pop(&mut self) -> Option<KeyInput> {
        let val = std::mem::replace(&mut self.buf[self.index as usize], None);
        self.index = self.index.saturating_sub(1);

        val
    }
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
            _ => KeyCode::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, Event, TypeGetter)]
pub struct KeyInput {
    pub code: KeyCode,
    pub state: KeyState,
}

impl KeyInput {
    pub fn new(code: KeyCode, state: KeyState) -> Self {
        Self { code, state }
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

#[derive(Debug, Event, TypeGetter)]
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

#[derive(Debug, TypeGetter, Resource, Clone, Copy)]
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

pub struct PerfCounter {
    begin: Option<SystemTime>,
    begin_debug_event: Option<SystemTime>,
    end: Option<SystemTime>,
    end_debug_event: Option<SystemTime>,
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
}

impl PerfCounter {
    pub fn new(target_frame_len: Option<f64>) -> Self {
        Self {
            begin: None,
            begin_debug_event: None,
            end: None,
            end_debug_event: None,
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
        }
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

        // trace!(
        //     "> Measured Frame Length: {},\tTarget Frame Length: {},\tLoss: {}",
        //     self.current_frame_len().unwrap_or_default().as_secs_f64(),
        //     self.target_frame_len.unwrap_or_default(),
        //     (self.current_frame_len().unwrap_or_default().as_secs_f64()
        //         - self.target_frame_len.unwrap_or_default())
        //     .abs()
        // );
        self.frames_sum += self.current_frame_len().unwrap_or_default().as_secs_f64();

        self.frames += 1;

        self.duration = self
            .end
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .expect("time is a construct")
            .as_secs_f64()
            - self.start_of_second.as_secs_f64();

        if self.duration >= 1.0 {
            self.start_of_second = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is a construct");
            self.total_frames += self.frames;

            trace!(
                "< Frames {},\tDuration: {},\tExpected {} Frames: {},\tLost Frames: {}",
                self.frames,
                self.duration,
                self.frames,
                self.frames_sum,
                self.lost_frames
            );

            if self.lost_frames > self.highest_lost_frames {
                self.highest_lost_frames = self.lost_frames;
            }
            self.frames = 0;
            self.lost_frames = 0;
            self.frames_sum = 0.0;
            self.iterations += 1;
        }
    }

    pub fn stop_debug_event(&mut self) {
        self.end_debug_event = Some(SystemTime::now());
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
            self.total_frames / self.iterations,
            self.lost_frames_sum,
            self.lost_frames_sum / self.iterations,
            self.highest_lost_frames
        );
    }
}
