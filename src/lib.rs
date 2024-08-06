use app::{plugins::Plugin, time::TimePlugin, window::WindowPlugin};
use asset::AssetLoaderPlugin;
use audio::AudioPlugin;
#[cfg(feature = "editor")]
use editor::EditorPlugin;
use gfx::gui::EguiPlugin;
use gfx::render::RendererPlugin;
use gfx::{
    render_pipeline::{
        bind_group::BindGroupPlugin,
        material::{Material2d, MaterialPlugin},
        shader::ShaderPlugin,
    },
    sprite::SpritePlugin,
    texture::TexturePlugin,
};
use log::LogPlugin;

pub extern crate app;
pub extern crate asset;
pub extern crate audio;
pub extern crate ecs;
#[cfg(feature = "hot_reload")]
pub extern crate editor;
pub extern crate gfx;
#[cfg(feature = "hot_reload")]
pub extern crate hot_reload;
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
    pub log: LogPlugin,
}

impl Default for DefaultPlugins {
    fn default() -> Self {
        Self {
            window: WindowPlugin {
                ..Default::default()
            },
            log: Default::default(),
        }
    }
}

impl Plugin for DefaultPlugins {
    fn build(&mut self, app: &mut app::app::App) {
        app.add_plugins_priority((
            RendererPlugin,
            BindGroupPlugin,
            self.log.clone(),
            self.window.clone(),
            AssetLoaderPlugin,
            TexturePlugin,
            TimePlugin,
            SpritePlugin,
            AudioPlugin,
            ShaderPlugin,
            // ModelPlugin,
        ))
        .add_plugins((
            #[cfg(feature = "egui")]
            EguiPlugin,
            #[cfg(feature = "editor")]
            EditorPlugin,
            MaterialPlugin::<Material2d>::new(),
        ));
    }
}
