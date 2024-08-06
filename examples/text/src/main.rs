use winny::{
    app::render::*,
    gfx::text::{TextPlugin, TextRenderer},
    prelude::*,
};

pub fn main() {
    App::default()
        .add_plugins((
            DefaultPlugins {
                window: WindowPlugin {
                    title: "text_example",
                    close_on_escape: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            TextPlugin::new("res/DejaVuSans.ttf"),
        ))
        .add_systems(Schedule::StartUp, startup)
        .add_systems(Schedule::PostUpdate, draw_text)
        .run();
}

fn startup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn draw_text(mut text_renderer: Option<ResMut<TextRenderer>>, context: Res<RenderContext>) {
    let Some(text_renderer) = &mut text_renderer else {
        return;
    };

    use winny::gfx::wgpu_text::glyph_brush::*;

    text_renderer.draw(&context, || {
        let corner = Section::default()
            .add_text(
                Text::new("Hello, world!")
                    .with_scale(40.0)
                    .with_color([0.9, 0.5, 0.5, 1.0]),
            )
            .with_bounds((
                context.config.width() as f32,
                context.config.height() as f32,
            ))
            .with_layout(Layout::default().line_breaker(BuiltInLineBreaker::UnicodeLineBreaker));

        let middle = Section::default()
            .add_text(
                Text::new("I am centered!")
                    .with_scale(40.0)
                    .with_color([0.9, 0.5, 0.5, 1.0]),
            )
            .with_bounds((
                context.config.width() as f32,
                context.config.height() as f32,
            ))
            .with_screen_position((
                context.config.width() as f32 / 2.,
                context.config.height() as f32 / 2.,
            ))
            .with_layout(
                Layout::default()
                    .h_align(HorizontalAlign::Center)
                    .v_align(VerticalAlign::Center),
            );

        vec![corner, middle]
    });
}
