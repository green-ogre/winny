use winny::{gfx::camera::Camera2dBundle, math::vector::Vec4f, prelude::*};

fn main() {
    App::default()
        .add_plugins((
            DefaultPlugins {
                window: WindowPlugin {
                    title: "lighting-2d",
                    close_on_escape: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            Lighting2dPlugin,
        ))
        .add_systems(Schedule::StartUp, startup)
        .run();
}

fn startup(mut commands: Commands, mut clear: ResMut<ClearColor>) {
    clear.0 = Vec4f::new(0., 0., 0., 1.);
    commands.spawn(Camera2dBundle::default());
}
