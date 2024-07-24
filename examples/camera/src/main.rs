use winny::{gfx::sprite::SpritePlugin, prelude::*};

fn main() {
    App::default()
        .add_plugins((
            DefaultPlugins {
                window: WindowPlugin {
                    close_on_escape: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            SpritePlugin,
        ))
        .add_systems(Schedule::StartUp, startup)
        .run();
}

fn startup(mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn(CameraBundle2d::default());
}
