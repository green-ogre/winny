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
            EguiPlugin,
            EditorPlugin,
        ))
        .run();
}
