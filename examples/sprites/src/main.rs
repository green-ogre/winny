use winny::{
    gfx::{
        cgmath::{Quaternion, Rad, Rotation3},
        sprite::{Sprite, SpriteBundle, SpritePlugin},
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
                    // Sprite rotation will stretch in a non square window / viewport
                    window_size: Vec2f::new(1200., 1200.),
                    viewport_size: Vec2f::new(1200., 1200.),
                    ..Default::default()
                },
                ..Default::default()
            },
            SpritePlugin::default(),
            EguiPlugin::<UiState>::new(),
        ))
        .insert_resource(UiState::new())
        .add_systems(Schedule::StartUp, startup)
        .add_systems(Schedule::Update, update_sprite)
        .run();
}

fn startup(mut commands: Commands, mut server: ResMut<AssetServer>) {
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                position: Vec3f::new(0., 0., 0.),
                ..Default::default()
            },
            handle: server.load("shrek.png"),
        },
        // Required for the `SpriteRenderer` to "see" the Sprite
        Transform::default(),
        Marked,
    ));

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                position: Vec3f::new(0., 0., 0.),
                scale: Vec2f::new(0.8, 0.8),
                ..Default::default()
            },
            handle: server.load("shrek.png"),
        },
        // Required for the `SpriteRenderer` to "see" the Sprite
        Transform {
            translation: Vec3f::new(500., 0., 0.),
            ..Default::default()
        },
    ));

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                position: Vec3f::new(0., 0., 0.),
                ..Default::default()
            },
            handle: server.load("donkey.png"),
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

fn update_sprite(
    ui_state: Res<UiState>,
    mut sprite: Query<(Mut<Sprite>, Mut<Transform>), With<Marked>>,
) {
    let Some((sprite, transform)) = sprite.iter_mut().next() else {
        return;
    };

    *sprite = ui_state.sprite;
    *transform = ui_state.transform;
}

#[derive(Resource)]
struct UiState {
    sprite: Sprite,
    transform: Transform,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            sprite: Sprite {
                z: 10,
                ..Default::default()
            },
            transform: Transform::default(),
        }
    }
}

impl UiRenderState for UiState {
    fn ui(&mut self) -> impl FnOnce(&egui::Context) {
        use winny::gfx::egui::*;
        |ctx: &Context| {
            egui::Window::new("Shrek Sprite").show(ctx, |ui| {
                ui.collapsing("Sprite", |ui| {
                    ui.collapsing("position", |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.sprite.position.x, 0.0..=1920.).text("x"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.sprite.position.y, 0.0..=1080.).text("y"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.sprite.position.z, 0.0..=1080.).text("z"),
                        );
                    });
                    ui.collapsing("scale", |ui| {
                        ui.add(egui::Slider::new(&mut self.sprite.scale.v[0], 0.0..=1.).text("x"));
                        ui.add(egui::Slider::new(&mut self.sprite.scale.v[1], 0.0..=1.).text("y"));
                    });
                    ui.collapsing("mask", |ui| {
                        ui.add(egui::Slider::new(&mut self.sprite.mask.v[0], 0.0..=1.).text("r"));
                        ui.add(egui::Slider::new(&mut self.sprite.mask.v[1], 0.0..=1.).text("g"));
                        ui.add(egui::Slider::new(&mut self.sprite.mask.v[2], 0.0..=1.).text("b"));
                        ui.add(egui::Slider::new(&mut self.sprite.mask.v[3], 0.0..=1.).text("a"));
                    });
                    ui.add(
                        egui::Slider::new(&mut self.sprite.rotation.0, 0.0..=360.).text("rotation"),
                    );
                    ui.add(egui::Slider::new(&mut self.sprite.z, 0..=1000).text("z"));
                    ui.add(egui::Checkbox::new(&mut self.sprite.v_flip, "v_flip"));
                    ui.add(egui::Checkbox::new(&mut self.sprite.h_flip, "h_flip"));
                });

                let mut quat_z = 0.0;
                ui.collapsing("Transform", |ui| {
                    ui.collapsing("translation", |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.transform.translation.x, 0.0..=1920.)
                                .text("x"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.transform.translation.y, 0.0..=1080.)
                                .text("y"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.transform.translation.z, -1000.0..=1000.)
                                .text("z"),
                        );
                    });

                    ui.collapsing("rotation", |ui| {
                        ui.add(egui::Slider::new(&mut quat_z, 0.0..=6.3).text("z-axis"));
                    });

                    ui.collapsing("scale", |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.transform.scale.v[0], 0.0..=2.).text("x"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.transform.scale.v[1], 0.0..=2.).text("y"),
                        );
                    });
                });

                self.transform.rotation = Quaternion::<f32>::from_angle_z(Rad(quat_z));
            });
        }
    }
}
