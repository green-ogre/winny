use std::sync::Arc;

use super::*;
use app::{RedrawRequest, WindowCreated, WindowResized, WinitEvent};
use ecs::WinnyResource;
use plugins::Plugin;
use prelude::{KeyInput, MouseInput};

#[derive(Debug, WinnyResource, Clone, Copy)]
pub struct WindowPlugin {
    pub title: &'static str,
    pub inner_size: (u32, u32),
    pub virtual_size: (u32, u32),
    pub position: (u32, u32),
    pub close_on_escape: bool,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        Self {
            title: "Winny",
            inner_size: (1920, 1080),
            virtual_size: (1920, 1080),
            position: (10, 10),
            close_on_escape: false,
        }
    }
}

impl Plugin for WindowPlugin {
    fn build(&mut self, app: &mut crate::app::App) {
        app.insert_resource(self.clone())
            .register_event::<RedrawRequest>()
            .register_event::<WindowResized>()
            .register_event::<MouseInput>()
            .register_event::<KeyInput>()
            .register_event::<WindowCreated>()
            .register_event::<WinitEvent>();
    }
}

#[derive(WinnyResource)]
pub struct WinitWindow(pub Arc<winit::window::Window>);
