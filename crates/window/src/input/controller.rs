#![allow(unused)]
use ecs::{WinnyEvent, WinnyResource};
use gilrs::EventType;
use logger::error;

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
