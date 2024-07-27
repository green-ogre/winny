use winny::{
    gfx::{
        particle::{ParticleBundle, ParticleEmitter, ParticlePlugin},
        transform::Transform,
    },
    prelude::*,
};

fn main() {
    App::default()
        .add_plugins((
            DefaultPlugins {
                window: WindowPlugin {
                    close_on_escape: true,
                    title: "Particles-example",
                    window_size: Vec2f::new(1200., 1200.),
                    ..Default::default()
                },
                ..Default::default()
            },
            EguiPlugin::<UiState>::new(),
            ParticlePlugin,
        ))
        .insert_resource(UiState::new())
        .add_systems(Schedule::StartUp, startup)
        .add_systems(Schedule::Update, update_emitter)
        .run();
}

fn startup(mut commands: Commands, mut server: ResMut<AssetServer>) {
    commands.spawn(ParticleBundle {
        emitter: ParticleEmitter::default(),
        handle: server.load("particle.png"),
        transform: Transform::default(),
    });
}

fn update_emitter(
    ui_state: Res<UiState>,
    mut emitter: Query<(Mut<ParticleEmitter>, Mut<Transform>)>,
) {
    let Some((emitter, transform)) = emitter.iter_mut().next() else {
        return;
    };

    *emitter = ui_state.emitter.clone();
    *transform = ui_state.transform;
}

#[derive(Resource)]
struct UiState {
    transform: Transform,
    emitter: ParticleEmitter,
    lifetime_min: f32,
    lifetime_max: f32,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            transform: Transform::default(),
            emitter: ParticleEmitter {
                num_particles: 1000,
                particle_scale: Vec2f::new(0.05, 0.05),
                ..Default::default()
            },
            lifetime_min: 0.5,
            lifetime_max: 1.5,
        }
    }
}

// pub struct ParticleEmitter {
//     pub is_emitting: bool,
//     pub num_particles: usize,
//     pub lifetime: f32,
//     pub radius: f32,
//     pub particle_scale: Vec2f,
//     pub particle_rotation: Radf,
// }

impl UiRenderState for UiState {
    fn ui(&mut self) -> impl FnOnce(&egui::Context) {
        use winny::gfx::egui::*;
        |ctx: &Context| {
            egui::Window::new("Particles").show(ctx, |ui| {
                egui::CollapsingHeader::new("ParticleEmitter")
                    .open(Some(true))
                    .show(ui, |ui| {
                        ui.add(egui::Checkbox::new(
                            &mut self.emitter.is_emitting,
                            "is_emitting",
                        ));

                        ui.add(
                            egui::Slider::new(&mut self.lifetime_min, 0.0..=10.)
                                .text("lifetime_min"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.lifetime_max, 0.0..=10.)
                                .text("lifetime_max"),
                        );
                        if self.lifetime_min > self.lifetime_max {
                            self.lifetime_max = self.lifetime_min;
                        }
                        if self.lifetime_min == self.lifetime_max {
                            self.lifetime_max += 0.01;
                        }
                        self.emitter.lifetime = self.lifetime_min..self.lifetime_max;

                        // ui.add(
                        //     egui::Slider::new(&mut self.emitter.radius, 0.0..=1000.).text("radius"),
                        // );
                        egui::CollapsingHeader::new("particle_scale")
                            .open(Some(true))
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.emitter.particle_scale.v[0],
                                        0.0..=2.,
                                    )
                                    .text("x"),
                                );
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.emitter.particle_scale.v[1],
                                        0.0..=2.,
                                    )
                                    .text("y"),
                                );
                            });
                        ui.add(
                            egui::Slider::new(&mut self.emitter.particle_rotation.0, 0.0..=13.)
                                .text("particle_rotation"),
                        );
                    });

                let mut quat_z = 0.0;
                egui::CollapsingHeader::new("Transform")
                    .open(Some(true))
                    .show(ui, |ui| {
                        egui::CollapsingHeader::new("Translation")
                            .open(Some(true))
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.transform.translation.x,
                                        -600.0..=600.,
                                    )
                                    .text("x"),
                                );
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.transform.translation.y,
                                        -600.0..=600.,
                                    )
                                    .text("y"),
                                );
                                ui.add(
                                    egui::Slider::new(
                                        &mut self.transform.translation.z,
                                        -1000.0..=1000.,
                                    )
                                    .text("z"),
                                );
                            });

                        egui::CollapsingHeader::new("Rotation")
                            .open(Some(true))
                            .show(ui, |ui| {
                                ui.add(egui::Slider::new(&mut quat_z, 0.0..=6.3).text("z-axis"));
                            });

                        egui::CollapsingHeader::new("Scale")
                            .open(Some(true))
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Slider::new(&mut self.transform.scale.v[0], 0.0..=2.)
                                        .text("x"),
                                );
                                ui.add(
                                    egui::Slider::new(&mut self.transform.scale.v[1], 0.0..=2.)
                                        .text("y"),
                                );
                            });
                    });

                use winny::gfx::cgmath::{Quaternion, Rad, Rotation3};
                self.transform.rotation = Quaternion::<f32>::from_angle_z(Rad(quat_z));
            });
        }
    }
}
