#![allow(unused)]

use app::{
    app::PerfPlugin,
    plugins::{Plugin, PluginSet},
};
use asset::AssetLoaderPlugin;
use gfx::{renderer::RendererPlugin, sprite::SpritePlugin};
use window::WindowPlugin;

pub extern crate app;
pub extern crate asset;
pub extern crate ecs;
pub extern crate gfx;
pub extern crate logger;
pub extern crate window;
pub extern crate winny_math as math;

#[cfg(feature = "audio")]
pub extern crate audio;

pub mod prelude {
    #[cfg(feature = "hot_reload")]
    pub use hot_reload::prelude::*;

    #[cfg(feature = "audio")]
    pub use audio::prelude::*;

    pub use app::prelude::*;
    pub use asset::prelude::*;
    pub use ecs::prelude::*;
    pub use gfx::prelude::*;
    pub use logger::*;
    pub use window::prelude::*;
    pub use winny_math as math;
    pub use winny_math::prelude::*;
}

pub struct DefaultPlugins {
    pub window: WindowPlugin,
    pub perf: PerfPlugin,
    pub asset_loader: AssetLoaderPlugin,
    pub sprites: SpritePlugin,
}

impl Default for DefaultPlugins {
    fn default() -> Self {
        Self {
            window: WindowPlugin {
                close_on_escape: true,
                ..Default::default()
            },
            perf: PerfPlugin,
            asset_loader: AssetLoaderPlugin {
                asset_folder: "res/".into(),
            },
            sprites: SpritePlugin,
        }
    }
}

impl Plugin for DefaultPlugins {
    fn build(&mut self, app: &mut app::app::App) {
        app.add_plugins((
            self.window.clone(),
            self.perf.clone(),
            self.asset_loader.clone(),
            self.sprites.clone(),
        ));
    }
}
