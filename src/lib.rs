use app::core::App;
use app::{plugins::Plugin, time::TimePlugin, window::WindowPlugin};
use asset::AssetLoaderPlugin;
use audio::AudioPlugin;
#[cfg(feature = "editor")]
use editor::EditorPlugin;
use gfx::camera::CameraPlugin;
use gfx::gui::EguiPlugin;
use gfx::mesh2d::Mesh2dPlugin;
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
use gfx::{ColorMaterial, MaterialEguiPlugin, TransformPlugin};
use log::LogPlugin;

pub extern crate app;
pub extern crate asset;
pub extern crate audio;
pub extern crate cereal;
pub extern crate ecs;
pub extern crate gfx;
#[cfg(feature = "hot_reload")]
pub extern crate hot_reload;
pub extern crate log;
pub extern crate math;
pub extern crate util;

extern crate self as winny;

pub mod prelude {
    pub use crate::DefaultPlugins;
    pub use app::prelude::*;
    pub use asset::*;
    pub use audio::*;
    pub use cereal::*;
    pub use ecs::*;
    pub use gfx::*;
    #[cfg(feature = "hot_reload")]
    pub use hot_reload::prelude::*;
    pub use log::*;
    pub use math::*;
    pub use util::*;
}

#[derive(Debug)]
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
    fn build(&mut self, app: &mut App) {
        app.add_plugins_priority((
            ShaderPlugin,
            EguiPlugin,
            #[cfg(feature = "editor")]
            EditorPlugin,
            TransformPlugin,
            CameraPlugin,
            MaterialEguiPlugin,
            MaterialPlugin::<Material2d>::new(),
            MaterialPlugin::<ColorMaterial>::new(),
        ))
        .add_plugins_priority((
            RendererPlugin,
            BindGroupPlugin,
            self.log.clone(),
            self.window.clone(),
            AssetLoaderPlugin,
            TexturePlugin,
            TimePlugin,
            Mesh2dPlugin,
            SpritePlugin,
            AudioPlugin,
        ));
    }
}
