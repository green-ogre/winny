use winny::{
    gfx::text::{TextPlugin, TextRenderer},
    prelude::*,
    render::{RenderConfig, RenderDevice, RenderQueue},
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
            TextPlugin::new("DejaVuSans.ttf"),
        ))
        .add_systems(Schedule::PreRender, draw_text)
        .run();
}

fn draw_text(
    mut text_renderer: Option<ResMut<TextRenderer>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    config: Res<RenderConfig>,
) {
    let Some(text_renderer) = &mut text_renderer else {
        return;
    };

    use winny::gfx::wgpu_text::glyph_brush::*;

    text_renderer.draw(&device, &queue, || {
        let corner = Section::default()
            .add_text(
                Text::new("Hello, world!")
                    .with_scale(40.0)
                    .with_color([0.9, 0.5, 0.5, 1.0]),
            )
            .with_bounds((config.width() as f32, config.height() as f32))
            .with_layout(Layout::default().line_breaker(BuiltInLineBreaker::UnicodeLineBreaker));

        let middle = Section::default()
            .add_text(
                Text::new("I am centered!")
                    .with_scale(40.0)
                    .with_color([0.9, 0.5, 0.5, 1.0]),
            )
            .with_bounds((config.width() as f32, config.height() as f32))
            .with_screen_position((config.width() as f32 / 2., config.height() as f32 / 2.))
            .with_layout(
                Layout::default()
                    .h_align(HorizontalAlign::Center)
                    .v_align(VerticalAlign::Center),
            );

        vec![corner, middle]
    });
}
