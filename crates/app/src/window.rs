use self::{
    app::AppSchedule,
    prelude::{AppExit, KeyCode, MouseMotion},
};
use super::*;
#[cfg(feature = "editor")]
use ecs::egui_widget::Widget;
use ecs::{
    events::{EventReader, EventWriter},
    WinnyResource,
};
use plugins::Plugin;
use prelude::{KeyInput, MouseInput};
use std::sync::Arc;
use winny_math::vector::Vec2f;

pub extern crate winit;

#[derive(Debug, WinnyResource, Clone, Copy)]
pub struct WindowPlugin {
    pub title: &'static str,
    pub window_size: Vec2f,
    pub viewport_size: Vec2f,
    pub close_on_escape: bool,
    pub maximized: bool,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        Self {
            title: "Winny",
            window_size: [1920.0, 1080.0].into(),
            viewport_size: [1920.0, 1080.0].into(),
            close_on_escape: false,
            maximized: false,
        }
    }
}

impl Plugin for WindowPlugin {
    fn build(&mut self, app: &mut crate::app::App) {
        if self.close_on_escape {
            app.add_systems(AppSchedule::Platform, should_exit);
        }

        app.insert_resource(self.clone())
            .register_resource::<Window>()
            .register_resource::<WindowResized>()
            .register_event::<MouseInput>()
            .register_event::<MouseMotion>()
            .register_event::<KeyInput>();
    }
}

#[derive(Debug, WinnyResource)]
pub struct WindowResized(pub u32, pub u32);

#[derive(WinnyResource)]
pub struct Window {
    pub winit_window: Arc<winit::window::Window>,
    pub viewport: ViewPort,
    pub is_init: bool,
}

impl Window {
    pub fn new(winit_window: Arc<winit::window::Window>, viewport: ViewPort) -> Self {
        Self {
            winit_window,
            viewport,
            is_init: true,
        }
    }

    pub fn set_title(&mut self, title: &str) {
        self.winit_window.set_title(title);
        self.is_init = false;
    }
}

fn should_exit(mut event_writer: EventWriter<AppExit>, key_input: EventReader<KeyInput>) {
    for input in key_input.peak_read() {
        if input.code == KeyCode::Escape {
            event_writer.send(AppExit);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ViewPort {
    // top left
    pub min: Vec2f,
    // bottom right
    pub max: Vec2f,
}

#[cfg(feature = "editor")]
impl Widget for ViewPort {
    fn display(&mut self, ui: &mut ecs::prelude::egui::Ui) {
        ui.with_layout(
            ecs::prelude::egui::Layout::left_to_right(ecs::prelude::egui::Align::TOP),
            |ui| {
                ui.label("min: ");
                self.min.display(ui);
            },
        );
        ui.with_layout(
            ecs::prelude::egui::Layout::left_to_right(ecs::prelude::egui::Align::TOP),
            |ui| {
                ui.label("max: ");
                self.max.display(ui);
            },
        );
    }
}

impl ViewPort {
    pub fn new(min: Vec2f, max: Vec2f) -> Self {
        Self { min, max }
    }

    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn center(&self) -> Vec2f {
        Vec2f::new(
            self.width() / 2.0 + self.min.x,
            self.height() / 2.0 + self.min.y,
        )
    }
}
