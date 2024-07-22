#![allow(unused)]

use app::{
    app::AppExit,
    plugins::{Plugin, PluginSet},
    prelude::{KeyCode, KeyInput},
    window::WindowPlugin,
};
use asset::AssetLoaderPlugin;
use ecs::{EventReader, EventWriter};
// use gfx::sprite::SpritePlugin;

pub extern crate app;
pub extern crate asset;
pub extern crate audio;
pub extern crate ecs;
pub extern crate gfx;
#[cfg(feature = "hot_reload")]
pub extern crate hot_reload;
pub extern crate log;
pub extern crate render;
pub extern crate util;
pub extern crate winny_math as math;

pub mod prelude {
    pub use crate::DefaultPlugins;
    pub use app::prelude::*;
    pub use asset::prelude::*;
    pub use audio::prelude::*;
    pub use ecs::prelude::*;
    pub use gfx::prelude::*;
    #[cfg(feature = "hot_reload")]
    pub use hot_reload::prelude::*;
    pub use log::*;
    pub use util::prelude::*;
    pub use winny_math as math;
    pub use winny_math::prelude::*;
}

pub struct CloseOnEscape;

impl Plugin for CloseOnEscape {
    fn build(&mut self, app: &mut app::app::App) {
        app.add_systems(ecs::Schedule::Platform, should_exit);
    }
}

fn should_exit(mut event_writer: EventWriter<AppExit>, key_input: EventReader<KeyInput>) {
    for input in key_input.peak_read() {
        if input.code == KeyCode::Escape {
            event_writer.send(AppExit);
        }
    }
}

pub struct DefaultPlugins {
    pub window: WindowPlugin,
    pub asset_loader: AssetLoaderPlugin,
    pub log: bool,
}

impl Default for DefaultPlugins {
    fn default() -> Self {
        Self {
            window: WindowPlugin {
                ..Default::default()
            },
            asset_loader: AssetLoaderPlugin {
                asset_folder: "res/".into(),
            },
            log: true,
        }
    }
}

impl Plugin for DefaultPlugins {
    fn build(&mut self, app: &mut app::app::App) {
        app.add_plugins_priority((
            self.window.clone(),
            render::RendererPlugin,
            self.asset_loader.clone(),
            gfx::texture::TexturePlugin,
            gfx::model::ModelPlugin,
            CloseOnEscape,
            audio::AudioPlugin,
        ));
        if self.log {
            app.add_plugins_priority(log::LogPlugin);
        }
    }
}

impl DefaultPlugins {
    pub fn without_log(mut self) -> Self {
        self.log = false;
        self
    }
}
