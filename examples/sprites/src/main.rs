use winny::{
    gfx::{
        render_pipeline::material::Material2d,
        sprite::{Sprite, SpriteBundle},
        transform::Transform,
    },
    prelude::*,
};

fn main() {
    App::default()
        .add_plugins(DefaultPlugins {
            window: WindowPlugin {
                maximized: true,
                close_on_escape: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .add_systems(Schedule::StartUp, startup)
        .run();
}

fn startup(mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        SpriteBundle {
            material: Material2d::default(),
            sprite: Sprite {
                position: Vec3f::new(0., 0., 0.),
                ..Default::default()
            },
            handle: server.load("res/shrek.png"),
        },
        // Required for the `SpriteRenderer` to "see" the Sprite
        Transform::default(),
        Marked,
    ));

    commands.spawn((
        SpriteBundle {
            material: Material2d::default(),
            sprite: Sprite {
                position: Vec3f::new(0., 0., 0.),
                scale: Vec2f::new(0.8, 0.8),
                ..Default::default()
            },
            handle: server.load("res/shrek.png"),
        },
        // Required for the `SpriteRenderer` to "see" the Sprite
        Transform {
            translation: Vec3f::new(500., 0., 0.),
            ..Default::default()
        },
    ));

    commands.spawn((
        SpriteBundle {
            material: Material2d::default(),
            sprite: Sprite {
                position: Vec3f::new(0., 0., 0.),
                ..Default::default()
            },
            handle: server.load("res/donkey.png"),
        },
        // Required for the `SpriteRenderer` to "see" the Sprite
        Transform {
            translation: Vec3f::new(-600., 0., 0.),
            ..Default::default()
        },
    ));
}

#[derive(Component)]
struct Marked;
