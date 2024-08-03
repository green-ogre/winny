use app::render::RenderContext;
use app::{app::Schedule, plugins::Plugin};
use asset::{Asset, AssetApp, AssetLoader, AssetServer};
use ecs::{Res, ResMut};

pub const DEFAULT_MATERIAL_2D_PATH_SPRITE: &'static str =
    "winny/shaders/default/material2d_sprite.wgsl";
pub const DEFAULT_MATERIAL_2D_PATH_PARTICLE: &'static str =
    "winny/shaders/default/material2d_particle.wgsl";

pub struct ShaderPlugin;

impl Plugin for ShaderPlugin {
    fn build(&mut self, app: &mut app::app::App) {
        let vert_loader = VertexShaderLoader;
        let frag_loader = FragmentShaderLoader;
        app.add_systems(Schedule::StartUp, startup)
            .register_asset_loader::<VertexShader>(vert_loader)
            .register_asset_loader::<FragmentShader>(frag_loader);
    }
}

// Load default shaders
fn startup(mut server: ResMut<AssetServer>, context: Res<RenderContext>) {
    let frag_shader = wgpu::ShaderModuleDescriptor {
        label: Some("frag"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("../shaders/material2d_particle.wgsl").into(),
        ),
    };
    server.store_asset(
        DEFAULT_MATERIAL_2D_PATH_PARTICLE.into(),
        FragmentShader(context.device.create_shader_module(frag_shader)),
    );

    let frag_shader = wgpu::ShaderModuleDescriptor {
        label: Some("frag"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/material2d_sprite.wgsl").into()),
    };
    server.store_asset(
        DEFAULT_MATERIAL_2D_PATH_SPRITE.into(),
        FragmentShader(context.device.create_shader_module(frag_shader)),
    );
}

pub struct VertexShader(pub wgpu::ShaderModule);
impl Asset for VertexShader {}

pub struct FragmentShader(pub wgpu::ShaderModule);
impl Asset for FragmentShader {}

pub struct VertexShaderLoader;

impl AssetLoader for VertexShaderLoader {
    type Asset = VertexShader;

    fn extensions(&self) -> &'static [&'static str] {
        &["wgsl"]
    }

    fn load(
        context: app::render::RenderContext,
        mut reader: asset::reader::ByteReader<std::io::Cursor<Vec<u8>>>,
        _path: String,
        _ext: &str,
    ) -> impl std::future::Future<Output = Result<Self::Asset, asset::AssetLoaderError>> {
        async move {
            let string = reader.read_all_to_string()?;
            let vert_shader = wgpu::ShaderModuleDescriptor {
                label: Some("vert"),
                source: wgpu::ShaderSource::Wgsl(string.into()),
            };
            Ok(VertexShader(
                context.device.create_shader_module(vert_shader),
            ))
        }
    }
}

pub struct FragmentShaderLoader;

impl AssetLoader for FragmentShaderLoader {
    type Asset = FragmentShader;

    fn extensions(&self) -> &'static [&'static str] {
        &["wgsl"]
    }

    fn load(
        context: app::render::RenderContext,
        mut reader: asset::reader::ByteReader<std::io::Cursor<Vec<u8>>>,
        _path: String,
        _ext: &str,
    ) -> impl std::future::Future<Output = Result<Self::Asset, asset::AssetLoaderError>> {
        async move {
            let string = reader.read_all_to_string()?;
            let frag_shader = wgpu::ShaderModuleDescriptor {
                label: Some("frag"),
                source: wgpu::ShaderSource::Wgsl(string.into()),
            };
            Ok(FragmentShader(
                context.device.create_shader_module(frag_shader),
            ))
        }
    }
}
