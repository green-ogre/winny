use std::sync::Arc;

use self::prelude::{AppExit, KeyCode};

use super::*;
use ecs::{EventReader, EventWriter, WinnyResource};
use plugins::Plugin;
use prelude::{KeyInput, MouseInput};
use winny_math::vector::Vec2f;

pub extern crate winit;

#[derive(Debug, WinnyResource, Clone, Copy)]
pub struct WindowPlugin {
    pub title: &'static str,
    pub window_size: (u32, u32),
    pub viewport_size: (u32, u32),
    pub close_on_escape: bool,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        Self {
            title: "Winny",
            window_size: (1920, 1080),
            viewport_size: (1920, 1080),
            close_on_escape: false,
        }
    }
}

impl Plugin for WindowPlugin {
    fn build(&mut self, app: &mut crate::app::App) {
        if self.close_on_escape {
            app.add_systems(ecs::Schedule::Platform, should_exit);
        }

        app.insert_resource(self.clone())
            .register_resource::<Window>()
            .register_resource::<WindowResized>()
            .register_event::<MouseInput>()
            .register_event::<KeyInput>();
    }
}

#[derive(Debug, WinnyResource)]
pub struct WindowResized(pub u32, pub u32);

#[derive(WinnyResource)]
pub struct Window {
    window: Arc<winit::window::Window>,
    viewport: ViewPort,
}

impl Window {
    pub fn new(window: Arc<winit::window::Window>, viewport: ViewPort) -> Self {
        Self { window, viewport }
    }

    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }

    pub fn viewport(&self) -> ViewPort {
        self.viewport
    }

    pub fn window(&self) -> Arc<winit::window::Window> {
        Arc::clone(&self.window)
    }
}

fn should_exit(mut event_writer: EventWriter<AppExit>, key_input: EventReader<KeyInput>) {
    for input in key_input.peak_read() {
        if input.code == KeyCode::Escape {
            event_writer.send(AppExit);
        }
    }
}

#[derive(Clone, Copy)]
pub struct ViewPort {
    pub top_left: Vec2f,
    pub width: f32,
    pub height: f32,
}

impl ViewPort {
    pub fn new(width: f32, height: f32, top_left: Vec2f) -> Self {
        Self {
            width,
            height,
            top_left,
        }
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }
}
