use winny::prelude::*;

fn main() {
    App::default()
        .add_plugins((
            DefaultPlugins {
                window: WindowPlugin {
                    close_on_escape: true,
                    title: "Particles-example",
                    ..Default::default()
                },
                ..Default::default()
            },
            EguiPlugin::<UiState>::new(),
        ))
        .insert_resource(UiState::new())
        .add_systems(Schedule::StartUp, startup)
        .run();
}

fn startup(mut _commands: Commands, mut _server: ResMut<AssetServer>) {}

#[derive(Resource)]
struct UiState {}

impl UiState {
    pub fn new() -> Self {
        Self {}
    }
}

impl UiRenderState for UiState {
    fn ui(&mut self) -> impl FnOnce(&egui::Context) {
        use winny::gfx::egui::*;
        |ctx: &Context| {
            egui::Window::new("Particles").show(ctx, |_ui| {
                // ui.collapsing("Sprite", |ui| {});
            });
        }
    }
}
