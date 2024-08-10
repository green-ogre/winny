use std::io::Read;

use winny::{
    app::input::mouse_and_key::MouseState,
    asset::server::AssetServer,
    gfx::{
        camera::Camera2dBundle,
        mesh2d::{Mesh2d, Mesh2dPlugin, Points, Triangle},
    },
    math::vector::Vec2f,
    prelude::*,
};

fn main() {
    App::default()
        .add_plugins(
            DefaultPlugins {
                window: WindowPlugin {
                    title: "lighting-2d",
                    close_on_escape: true,
                    window_size: Vec2f::new(1000., 1000.),
                    viewport_size: Vec2f::new(1000., 1000.),
                    ..Default::default()
                },
                ..Default::default()
            },
            // Lighting2dPlugin,
        )
        .insert_resource(Mouse(0.0, 0.0))
        .register_resource::<GlobalMesh>()
        .insert_resource(GlobalMesh::default())
        .insert_resource(GlobalPoints(Points::default()))
        .insert_resource(SavePath(String::from("../../../res/saved/player_mesh.msh")))
        .add_systems(AppSchedule::PostStartUp, startup)
        .add_systems(Schedule::Update, update)
        .run();
}

#[derive(Resource)]
struct GlobalPoints(Points);

#[derive(Resource, Serialize, Deserialize, Default)]
struct PlayerPoints {
    mesh: Mesh2d,
}

#[derive(Resource, Serialize, Deserialize, Default)]
struct GlobalMesh {
    mesh: Mesh2d,
}

#[derive(Resource)]
struct Mouse(f32, f32);

#[derive(Resource, AsEgui)]
struct SavePath(String);

fn startup(mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                scale: Vec2f::new(0.3, 0.3),
                ..Default::default()
            },
            material: Material2d::default(),
            handle: server.load("res/crosshair.png"),
        },
        Transform::default(),
    ));
}

fn update(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh2d>>,
    mouse_input: EventReader<MouseInput>,
    mouse_motion: EventReader<MouseMotion>,
    mut state: ResMut<Mouse>,
    mut global_points: ResMut<GlobalPoints>,
    mut global_mesh: ResMut<GlobalMesh>,
    key_input: EventReader<KeyInput>,
    mesh_entities: Query<Entity, With<Handle<Mesh2d>>>,
    window: Res<Window>,
    save_path: Res<SavePath>,
) {
    for input in mouse_motion.read() {
        state.0 = input.0 as f32 - window.viewport.width() / 2.0;
        state.1 = input.1 as f32 - window.viewport.height() / 2.0;
    }

    for input in mouse_input.read() {
        if input.state == KeyState::Pressed {
            match input.button {
                MouseButton::Left => {
                    info!("adding point");
                    global_points.0.add(Vec2f::new(state.0, -state.1));
                }
                MouseButton::Right => {
                    info!("removing point");
                    global_points.0.pop();
                    for entity in mesh_entities.iter() {
                        commands.get_entity(entity).despawn();
                    }
                }
            }

            if let Some(mesh) = Mesh2d::from_points(global_points.0.clone()) {
                println!("{:#?}", mesh);
                global_mesh.mesh = mesh.clone();
                let handle = meshes.add(mesh);
                commands.spawn((Transform::default(), handle));
                for entity in mesh_entities.iter() {
                    commands.get_entity(entity).despawn();
                }
            }
        }
    }

    for KeyInput { code, state, .. } in key_input.peak_read() {
        if *state == KeyState::Pressed {
            if *code == KeyCode::S {
                save_struct(&global_mesh.mesh, &save_path.0);
            }
        }
    }
}

pub trait LoaderApp {
    fn save_load_resource<R: Serialize + Deserialize + Resource + Default>(&mut self) -> &mut Self;
}

impl LoaderApp for App {
    fn save_load_resource<R: Serialize + Deserialize + Resource + Default>(&mut self) -> &mut Self {
        self.add_systems(Schedule::StartUp, load_resource::<R>)
            .add_systems(Schedule::Exit, save_resource::<R>)
    }
}

fn load_resource<R: Serialize + Deserialize + Resource + Default>(mut commands: Commands) {
    let camera =
        if let Ok(f) = std::fs::File::open(format!("res/saved/{}", std::any::type_name::<R>())) {
            info!("deserializing [{}]", std::any::type_name::<R>());
            let mut bytes = Vec::new();
            std::io::BufReader::new(f).read_to_end(&mut bytes).unwrap();
            let mut d = Deserializer::new(&mut bytes);
            if let Some(val) = R::deserialize(&mut d) {
                val
            } else {
                error!("failed to deserialize [{}]", std::any::type_name::<R>());
                R::default()
            }
        } else {
            R::default()
        };

    commands.insert_resource(camera);
}

fn save_resource<R: Serialize + Deserialize + Resource + Default>(resource: Res<R>) {
    let mut bytes = Vec::new();
    let mut s = Serializer::new(&mut bytes);
    resource.serialize(&mut s);
    match std::fs::write(format!("res/saved/{}", std::any::type_name::<R>()), &bytes) {
        Ok(_) => (),
        Err(e) => error!("failed to serialize [{}]: {e}", std::any::type_name::<R>()),
    }
}

fn save_struct<R: Serialize + Deserialize + Default>(resource: &R, path: &String) {
    info!("saving struct to: {path}");
    let mut bytes = Vec::new();
    let mut s = Serializer::new(&mut bytes);
    resource.serialize(&mut s);
    match std::fs::write(format!("{}", path), &bytes) {
        Ok(_) => (),
        Err(e) => error!("failed to serialize [{}]: {e}", std::any::type_name::<R>()),
    }
}
