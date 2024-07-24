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
            EguiPlugin,
        ))
        .run();
}

fn run_ui(mut egui: ResMut<EguiRenderer>) {
    // egui.draw(|ctx| {
    //     egui::Window::new("I am egui!")
    //         .resizable(true)
    //         .show(&ctx, |ui| {
    //             ui.label("Hello world!");
    //             if ui.button("Click me").clicked() {
    //                 info!("You clicked me!");
    //             }
    //         });
    // });
}
