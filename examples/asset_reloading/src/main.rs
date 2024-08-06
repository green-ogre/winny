use winny::prelude::*;

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
            TomlPlugin,
            WatcherPlugin,
        ))
        .register_resource::<TomlFile>()
        .add_systems(Schedule::StartUp, startup)
        .add_systems(Schedule::Update, print_toml)
        .run();
}

#[derive(Resource)]
struct TomlFile(Handle<Toml>);

fn startup(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(TomlFile(server.load("res/test.toml")));
    commands.spawn((
        FileWatcherBundle {
            watcher: FileWatcher::new("res/test.toml").unwrap(),
        },
        WatchForAsset,
    ));
}

fn print_toml(toml: Res<TomlFile>, tomls: Res<Assets<Toml>>, reader: EventReader<KeyInput>) {
    for event in reader.read() {
        if matches!(
            event,
            KeyInput {
                code: KeyCode::R,
                state: KeyState::Pressed,
            }
        ) {
            if let Some(toml) = tomls.get(&toml.0) {
                let head = toml.head();
                if let Some(size) = head.get("slime").get("size").as_integer() {
                    println!("{size:#?}");
                }
            }
        }
    }
}
