#![allow(unused)]

use app::{app::PerfPlugin, plugins::Plugin};
use asset::AssetLoaderPlugin;
use window::WindowPlugin;

pub extern crate app;
pub extern crate asset;
pub extern crate ecs;
pub extern crate logger;
pub extern crate winny_math as math;

#[cfg(feature = "window")]
pub extern crate window;

#[cfg(feature = "window")]
pub extern crate gfx;

#[cfg(feature = "audio")]
pub extern crate audio;

pub mod prelude {
    #[cfg(feature = "hot_reload")]
    pub use hot_reload::prelude::*;

    #[cfg(feature = "window")]
    pub use window::prelude::*;

    #[cfg(feature = "window")]
    pub use gfx::prelude::*;

    #[cfg(feature = "audio")]
    pub use audio::prelude::*;

    pub use app::prelude::*;
    pub use asset::prelude::*;
    pub use ecs::prelude::*;
    pub use logger::*;
    pub use winny_math as math;
    pub use winny_math::prelude::*;
}

pub struct DefaultPlugins {
    pub window: WindowPlugin,
    pub perf: PerfPlugin,
    pub asset_loader: AssetLoaderPlugin,
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
        }
    }
}

impl Plugin for DefaultPlugins {
    fn build(&mut self, app: &mut app::app::App) {
        app.add_plugins((
            self.window.clone(),
            self.perf.clone(),
            self.asset_loader.clone(),
        ));
    }
}
