use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use super::*;
use ecs::WinnyResource;
use plugins::Plugin;
use prelude::{KeyInput, MouseInput};

#[derive(Debug, WinnyResource, Clone, Copy)]
pub struct WindowPlugin {
    pub title: &'static str,
    pub inner_size: (u32, u32),
    pub virtual_size: (u32, u32),
    pub position: (u32, u32),
}

impl Default for WindowPlugin {
    fn default() -> Self {
        Self {
            title: "Winny",
            inner_size: (1920, 1080),
            virtual_size: (1920, 1080),
            position: (10, 10),
        }
    }
}

impl Plugin for WindowPlugin {
    fn build(&mut self, app: &mut crate::app::App) {
        app.insert_resource(self.clone())
            .register_event::<MouseInput>()
            .register_event::<KeyInput>();
    }
}

#[derive(WinnyResource)]
pub struct WinitWindow(pub Arc<winit::window::Window>);

impl Deref for WinitWindow {
    type Target = Arc<winit::window::Window>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WinitWindow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
