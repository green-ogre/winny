use app::{app::Schedule, plugins::Plugin};
use asset::{AssetServer, Handle};
use ecs::{Commands, Res, ResMut, WinnyResource};

use crate::texture::Texture;

pub struct NoisePlugin {
    path: &'static str,
}

impl NoisePlugin {
    pub fn new(path: &'static str) -> Self {
        Self { path }
    }
}

impl Plugin for NoisePlugin {
    fn build(&mut self, app: &mut app::app::App) {
        app.register_resource::<NoiseTexture>()
            .insert_resource(NoiseTexturePath(self.path))
            .add_systems(Schedule::StartUp, startup);
    }
}

fn startup(mut commands: Commands, mut server: ResMut<AssetServer>, path: Res<NoiseTexturePath>) {
    commands.insert_resource(NoiseTexture(server.load(path.0)));
}

#[derive(WinnyResource)]
pub struct NoiseTexture(pub Handle<Texture>);

#[derive(WinnyResource)]
pub struct NoiseTexturePath(pub &'static str);
