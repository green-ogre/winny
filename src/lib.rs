#![allow(unused)]

use app::{
    app::AppExit,
    plugins::{Plugin, PluginSet},
    prelude::{KeyCode, KeyInput},
    time::TimePlugin,
    window::WindowPlugin,
};
use asset::AssetLoaderPlugin;
use audio::AudioPlugin;
use ecs::{EventReader, EventWriter};
use gfx::{model::ModelPlugin, texture::TexturePlugin};
use log::LogPlugin;
use render::RendererPlugin;

pub extern crate app;
pub extern crate asset;
pub extern crate audio;
pub extern crate ecs;
pub extern crate gfx;
#[cfg(feature = "hot_reload")]
pub extern crate hot_reload;
pub extern crate render;
extern crate self as winny;
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
    pub use log::LogPlugin;
    pub use util::prelude::*;
    pub use winny_math as math;
    pub use winny_math::prelude::*;
}

pub struct DefaultPlugins {
    pub window: WindowPlugin,
    pub asset_loader: AssetLoaderPlugin,
    pub log: LogPlugin,
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
            log: Default::default(),
        }
    }
}

impl Plugin for DefaultPlugins {
    fn build(&mut self, app: &mut app::app::App) {
        app.add_plugins_priority((
            self.log.clone(),
            TimePlugin,
            self.window.clone(),
            RendererPlugin,
            self.asset_loader.clone(),
            // CameraPlugin,
            TexturePlugin,
            ModelPlugin,
            AudioPlugin,
        ));
    }
}
