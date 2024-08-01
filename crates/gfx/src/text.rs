use app::app::AppSchedule;
use asset::{
    Asset, AssetApp, AssetLoader, AssetLoaderError, AssetLoaderEvent, AssetServer, Assets, Handle,
};
use ecs::{Commands, EventReader, Res, ResMut, WinnyResource};
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{Extra, Section};
use wgpu_text::{BrushBuilder, TextBrush};

use app::plugins::Plugin;
use render::{RenderConfig, RenderContext, RenderDevice, RenderEncoder, RenderQueue, RenderView};

pub struct TextPlugin {
    text_path: String,
}

impl TextPlugin {
    pub fn new<P: Into<String>>(path: P) -> Self {
        Self {
            text_path: path.into(),
        }
    }
}

impl Plugin for TextPlugin {
    fn build(&mut self, app: &mut app::app::App) {
        let loader = TextAssetLoader;
        app.register_asset_loader::<Ttf>(loader)
            .insert_resource(TextPath(self.text_path.clone()))
            .register_resource::<TextHandle>()
            .register_resource::<TextRenderer>()
            .add_systems(AppSchedule::PreStartUp, startup)
            .add_systems(AppSchedule::Render, render_pass);
    }
}

pub struct Ttf {
    bytes: &'static [u8],
}

impl Asset for Ttf {}

struct TextAssetLoader;

impl AssetLoader for TextAssetLoader {
    type Asset = Ttf;

    fn extensions(&self) -> Vec<&'static str> {
        vec!["ttf"]
    }

    fn load(
        _context: render::RenderContext,
        mut reader: asset::reader::ByteReader<std::io::Cursor<Vec<u8>>>,
        _path: String,
        _ext: &str,
    ) -> impl std::future::Future<Output = Result<Self::Asset, asset::AssetLoaderError>> {
        async move {
            match reader.read_all() {
                Ok(bytes) => Ok(Ttf {
                    bytes: Box::leak(Box::new(bytes)),
                }),
                Err(_) => Err(AssetLoaderError::FailedToBuild),
            }
        }
    }
}

#[derive(WinnyResource)]
struct TextHandle(Handle<Ttf>);

#[derive(WinnyResource)]
struct TextPath(String);

fn startup(mut commands: Commands, mut server: ResMut<AssetServer>, path: Res<TextPath>) {
    let handle = server.load(path.0.as_str());
    commands.insert_resource(TextHandle(handle));
    commands.run_system_once_when(text_setup, should_run_text_setup);
}

fn should_run_text_setup(events: EventReader<AssetLoaderEvent<Ttf>>) -> bool {
    events.peak().is_some()
}

fn text_setup(
    mut commands: Commands,
    context: Res<RenderContext>,
    fonts: Res<Assets<Ttf>>,
    handle: Res<TextHandle>,
) {
    let font_bytes = &fonts.get(&handle.0).unwrap().bytes;
    commands.insert_resource(TextRenderer::new(
        font_bytes,
        &context.device,
        &context.config,
    ));
}

#[derive(WinnyResource)]
pub struct TextRenderer {
    brush: TextBrush<FontRef<'static>>,
}

impl TextRenderer {
    pub fn new(font_bytes: &'static [u8], device: &RenderDevice, config: &RenderConfig) -> Self {
        let brush = BrushBuilder::using_font_bytes(&font_bytes).unwrap().build(
            device,
            config.width() as u32,
            config.height() as u32,
            config.format(),
        );

        Self { brush }
    }

    pub fn draw<'a, F>(&mut self, device: &RenderDevice, queue: &RenderQueue, f: F)
    where
        F: FnOnce() -> Vec<Section<'a, Extra>>,
    {
        let sections = f();
        if let Err(e) = self.brush.queue(&device, &queue, sections) {
            panic!("{e}");
        };
    }
}

fn render_pass(
    mut text_renderer: Option<ResMut<TextRenderer>>,
    mut encoder: ResMut<RenderEncoder>,
    view: Res<RenderView>,
) {
    let Some(text_renderer) = &mut text_renderer else {
        return;
    };

    {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("text"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        text_renderer.brush.draw(&mut rpass)
    }
}
