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
    W,
    S,
    A,
    D,
    H,
    J,
    K,
    L,
    E,
    I,
    Key1,
    Key2,
    Escape,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyInput {
    pub vk: KeyCode,
    pub state: KeyState,
}
