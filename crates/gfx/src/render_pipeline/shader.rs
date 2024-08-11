use app::prelude::*;
use asset::*;
use std::ops::Deref;

#[derive(Debug)]
pub struct ShaderPlugin;

impl Plugin for ShaderPlugin {
    fn build(&mut self, app: &mut App) {
        let vert_loader = VertexShaderLoader;
        let frag_loader = FragmentShaderLoader;
        app.register_asset::<VertexShaderSource>()
            .register_asset::<FragmentShaderSource>()
            .register_asset_loader::<VertexShaderSource>(vert_loader)
            .register_asset_loader::<FragmentShaderSource>(frag_loader);
    }
}

pub struct VertexShader(pub wgpu::ShaderModule);

impl Deref for VertexShader {
    type Target = wgpu::ShaderModule;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct VertexShaderSource(pub String, pub Option<VertexShader>);

impl Asset for VertexShaderSource {}

impl VertexShaderSource {
    fn create_shader(&mut self, context: &RenderContext) {
        let vert_shader = wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(self.0.clone().into()),
        };
        let shader = VertexShader(context.device.create_shader_module(vert_shader));
        self.1 = Some(shader);
    }

    pub fn shader(&mut self, context: &RenderContext) -> &VertexShader {
        if self.1.is_none() {
            self.create_shader(context);
        }

        self.1.as_ref().unwrap()
    }
}

pub struct FragmentShader(pub wgpu::ShaderModule);

impl Deref for FragmentShader {
    type Target = wgpu::ShaderModule;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct FragmentShaderSource(pub String, pub Option<FragmentShader>);

impl Asset for FragmentShaderSource {}

impl FragmentShaderSource {
    fn create_shader(&mut self, context: &RenderContext) {
        let frag_shader = wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(self.0.clone().into()),
        };
        let shader = FragmentShader(context.device.create_shader_module(frag_shader));
        self.1 = Some(shader);
    }

    pub fn shader(&mut self, context: &RenderContext) -> &FragmentShader {
        if self.1.is_none() {
            self.create_shader(context);
        }

        self.1.as_ref().unwrap()
    }
}

pub struct VertexShaderLoader;

impl AssetLoader for VertexShaderLoader {
    type Asset = VertexShaderSource;
    type Settings = ();

    fn extensions(&self) -> &'static [&'static str] {
        &["wgsl"]
    }

    async fn load(
        mut reader: asset::reader::ByteReader<std::io::Cursor<Vec<u8>>>,
        _settings: Self::Settings,
        _path: String,
        _ext: &str,
    ) -> Result<Self::Asset, asset::AssetLoaderError> {
        let string = reader.read_all_to_string()?;
        Ok(VertexShaderSource(string, None))
    }
}

pub struct FragmentShaderLoader;

impl AssetLoader for FragmentShaderLoader {
    type Asset = FragmentShaderSource;
    type Settings = ();

    fn extensions(&self) -> &'static [&'static str] {
        &["wgsl"]
    }

    async fn load(
        mut reader: asset::reader::ByteReader<std::io::Cursor<Vec<u8>>>,
        _settings: Self::Settings,
        _path: String,
        _ext: &str,
    ) -> Result<Self::Asset, asset::AssetLoaderError> {
        let string = reader.read_all_to_string()?;
        Ok(FragmentShaderSource(string, None))
    }
}
