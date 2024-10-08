use ecs::{WinnyEvent, WinnyResource};

#[derive(Debug, Default, WinnyResource)]
pub struct MouseState {
    pub last_mouse_position: (f64, f64),
    pub last_held_key: Option<MouseButton>,
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
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,
    Space,
    Shift,
    Escape,
    Enter,
    Tab,
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
            winit::keyboard::KeyCode::Digit0 => KeyCode::Key0,
            winit::keyboard::KeyCode::Digit1 => KeyCode::Key1,
            winit::keyboard::KeyCode::Digit2 => KeyCode::Key2,
            winit::keyboard::KeyCode::Digit3 => KeyCode::Key3,
            winit::keyboard::KeyCode::Digit4 => KeyCode::Key4,
            winit::keyboard::KeyCode::Digit5 => KeyCode::Key5,
            winit::keyboard::KeyCode::Digit6 => KeyCode::Key6,
            winit::keyboard::KeyCode::Digit7 => KeyCode::Key7,
            winit::keyboard::KeyCode::Digit8 => KeyCode::Key8,
            winit::keyboard::KeyCode::Digit9 => KeyCode::Key9,
            winit::keyboard::KeyCode::Space => KeyCode::Space,
            winit::keyboard::KeyCode::ShiftLeft => KeyCode::Shift,
            winit::keyboard::KeyCode::ShiftRight => KeyCode::Shift,
            winit::keyboard::KeyCode::Escape => KeyCode::Escape,
            winit::keyboard::KeyCode::Enter => KeyCode::Enter,
            winit::keyboard::KeyCode::Tab => KeyCode::Tab,
            _ => KeyCode::Unknown,
        }
    }
}

#[derive(Debug, Clone, WinnyEvent)]
pub struct KeyInput {
    pub code: KeyCode,
    pub state: KeyState,
    pub text: Option<String>,
}

impl KeyInput {
    pub fn new(code: KeyCode, state: KeyState, text: Option<String>) -> Self {
        Self { code, state, text }
    }
}

#[derive(WinnyEvent, Debug, Clone, Copy)]
pub struct MouseWheel(pub MouseScrollDelta);

#[derive(Debug, Clone, Copy)]
pub enum MouseScrollDelta {
    LineDelta(f32, f32),
    PixelDelta(f32, f32),
}

#[derive(Debug, Clone, Copy, WinnyEvent)]
pub struct MouseInput {
    pub button: MouseButton,
    pub state: KeyState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
}

impl MouseInput {
    pub fn new(button: MouseButton, state: KeyState) -> Self {
        Self { button, state }
    }
}

#[derive(Debug, Clone, Copy, WinnyEvent)]
pub struct MouseMotion(pub f64, pub f64);
