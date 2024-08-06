use winny::{
    gfx::{
        particle::{ParticleBundle, ParticleEmitter},
        render_pipeline::material::Material2d,
        transform::Transform,
    },
    prelude::*,
};

fn main() {
    App::default()
        .add_plugins(DefaultPlugins {
            window: WindowPlugin {
                close_on_escape: true,
                title: "Particles-example",
                window_size: Vec2f::new(1200., 1200.),
                viewport_size: Vec2f::new(1200., 1200.),
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
        ParticleBundle {
            emitter: ParticleEmitter {
                num_particles: 300,
                width: 300.,
                height: 300.,
                lifetime: 6.0..6.5,
                particle_scale: Vec2f::new(0.05, 0.05),
                ..Default::default()
            },
            material: Material2d::default(),
            handle: server.load("res/particle.png"),
        },
        Transform {
            ..Default::default()
        },
    ));
}
