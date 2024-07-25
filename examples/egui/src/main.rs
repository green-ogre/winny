use winny::{gfx::editor::EditorPlugin, prelude::*};

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
            EditorPlugin,
        ))
        .add_systems(Schedule::StartUp, startup)
        .run();
}

fn startup(mut commands: Commands) {
    commands.spawn(CameraBundle2d::default());
}
