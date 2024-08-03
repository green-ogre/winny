use app::render::{Dimensions, RenderConfig, RenderContext, RenderDevice, RenderQueue};
use asset::{load_binary, reader::ByteReader, Asset, AssetApp, AssetLoaderError};
use ecs::WinnyComponent;
use image::{DynamicImage, GenericImageView};
use std::{fmt::Display, future::Future, io::Cursor};
use util::tracing::trace;
use wgpu::util::DeviceExt;

pub struct TexturePlugin;

impl app::plugins::Plugin for TexturePlugin {
    fn build(&mut self, app: &mut app::app::App) {
        let texture_loader = TextureAssetLoader {};
        let image_loader = ImageAssetLoader {};
        let atlas_loader = TextureAtlasLoader {};

        app.register_asset_loader::<Texture>(texture_loader)
            .register_asset_loader::<TextureAtlas>(atlas_loader)
            .register_asset_loader::<Image>(image_loader);
    }
}

struct TextureAtlasLoader;

impl asset::AssetLoader for TextureAtlasLoader {
    type Asset = TextureAtlas;

    fn extensions(&self) -> &'static [&'static str] {
        &["png"]
    }

    fn load(
        _context: RenderContext,
        _reader: asset::reader::ByteReader<std::io::Cursor<Vec<u8>>>,
        _path: String,
        _ext: &str,
    ) -> impl Future<Output = Result<Self::Asset, AssetLoaderError>> {
        async move { panic!() }
    }
}

struct ImageAssetLoader;

impl asset::AssetLoader for ImageAssetLoader {
    type Asset = Image;

    fn extensions(&self) -> &'static [&'static str] {
        &["png"]
    }

    fn load(
        _context: RenderContext,
        reader: asset::reader::ByteReader<std::io::Cursor<Vec<u8>>>,
        _path: String,
        ext: &str,
    ) -> impl Future<Output = Result<Self::Asset, AssetLoaderError>> {
        async move {
            match ext {
                "png" => Image::new(reader),
                _ => Err(AssetLoaderError::UnsupportedFileExtension),
            }
        }
    }
}

struct TextureAssetLoader;

impl asset::AssetLoader for TextureAssetLoader {
    type Asset = Texture;

    fn extensions(&self) -> &'static [&'static str] {
        &["png"]
    }

    fn load(
        context: RenderContext,
        reader: asset::reader::ByteReader<std::io::Cursor<Vec<u8>>>,
        _path: String,
        ext: &str,
    ) -> impl Future<Output = Result<Self::Asset, AssetLoaderError>> {
        async move {
            match ext {
                "png" => {
                    let source = Image::new(reader)?;
                    Ok(Texture::from_image(
                        &context.device,
                        &context.queue,
                        &source.image,
                    ))
                }
                _ => Err(AssetLoaderError::UnsupportedFileExtension),
            }
        }
    }
}

pub struct Image {
    pub image: DynamicImage,
}

impl Asset for Image {}

impl Image {
    pub fn new(mut reader: ByteReader<Cursor<Vec<u8>>>) -> Result<Self, AssetLoaderError> {
        let data = reader
            .read_all()
            .map_err(|_| AssetLoaderError::FailedToParse)?;
        let image = image::load_from_memory(&data).map_err(|_| AssetLoaderError::FailedToParse)?;

        Ok(Self { image })
    }
}

/// Dimensions of a [`Texture`] in pixels.
#[derive(WinnyComponent, Debug, Clone, Copy)]
pub struct TextureDimensions(Dimensions<f32>);

impl TextureDimensions {
    pub fn from_texture(texture: &Texture) -> Self {
        Self(Dimensions::new(
            texture.texture.width() as f32,
            texture.texture.height() as f32,
        ))
    }

    pub fn from_texture_atlas(atlas: &TextureAtlas) -> Self {
        Self::from_texture(&atlas.texture)
    }

    pub fn width(&self) -> f32 {
        self.0.width()
    }

    pub fn height(&self) -> f32 {
        self.0.height()
    }
}

///// Configuration for a [`wgpu::Texture`] and [`wgpu::Sampler`] within the [`asset::AssetServer`].
//#[derive(WinnyComponent, Debug)]
//pub struct TextureImportSettings {
//    pub texture: Arc<wgpu::TextureDescriptor>,
//    pub sampler: Arc<wgpu::SamplerDescriptor>,
//}

/// Handle to a GPU texture. Provides a [`wgpu::TextureView`] and [`wgpu::Sampler`].
#[derive(Debug)]
pub struct Texture {
    texture: wgpu::Texture,
}

impl asset::Asset for Texture {}

impl Texture {
    pub const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    pub async fn load_texture(
        file_name: &str,
        device: &RenderDevice,
        queue: &RenderQueue,
    ) -> Result<Texture, ()> {
        let data = load_binary(file_name).await.map_err(|_| ())?;
        Texture::from_image_bytes(device, queue, &data)
    }

    pub fn from_image_bytes(
        device: &RenderDevice,
        queue: &RenderQueue,
        bytes: &[u8],
    ) -> Result<Self, ()> {
        let img = image::load_from_memory(bytes).map_err(|_| ())?;
        Ok(Self::from_image(device, queue, &img))
    }

    pub fn from_image(
        device: &RenderDevice,
        queue: &RenderQueue,
        img: &image::DynamicImage,
    ) -> Self {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        Self::from_bytes(&rgba, dimensions, device, queue)
    }

    pub fn from_bytes(
        bytes: &[u8],
        dimensions: (u32, u32),
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: Texture::TEXTURE_FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                label: None,
                view_formats: &[],
            },
            Default::default(),
            bytes,
        );

        Self { texture }
    }

    pub fn empty(
        dimensions: Dimensions<u32>,
        device: &RenderDevice,
        usage: wgpu::TextureUsages,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: dimensions.width(),
            height: dimensions.height(),
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Texture::TEXTURE_FORMAT,
            usage,
            label: None,
            view_formats: &[],
        });

        Self { texture }
    }

    pub fn create_view(&self) -> wgpu::TextureView {
        self.texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn create_sampler(
        &self,
        context: &RenderContext,
        filter_type: &SamplerFilterType,
    ) -> wgpu::Sampler {
        let desc = match filter_type {
            SamplerFilterType::Nearest => wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            },
            SamplerFilterType::Linear => wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            },
        };

        context.device.create_sampler(&desc)
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn width(&self) -> u32 {
        self.texture.width()
    }

    pub fn height(&self) -> u32 {
        self.texture.height()
    }
}

pub enum SamplerFilterType {
    Nearest,
    Linear,
}

/// Handle to a sprite sheet [`Texture`].
#[derive(WinnyComponent, Debug)]
pub struct TextureAtlas {
    /// Number of Textures in 2D
    pub dimensions: Dimensions<u32>,
    pub texture: Texture,
}

impl Asset for TextureAtlas {}

impl TextureAtlas {
    pub fn build_atlas(
        device: &RenderDevice,
        queue: &RenderQueue,
        textures: Vec<Image>,
    ) -> Result<Self, AtlasError> {
        let dimensions = if let Some(first) = textures.first() {
            first.image.dimensions()
        } else {
            return Err(AtlasError::InputEmpty);
        };

        if textures.iter().any(|t| t.image.dimensions() != dimensions) {
            return Err(AtlasError::NonUniformDimensions);
        }

        let height = textures.len() as u32;
        let width = 1;

        let rgba = textures
            .into_iter()
            .map(|i| i.image.to_rgba8().to_vec())
            .flatten()
            .collect::<Vec<u8>>();

        let texture = Texture::from_bytes(
            &rgba,
            (dimensions.0 * width, dimensions.1 * height),
            device,
            queue,
        );

        let dimensions = Dimensions::new(width, height);

        Ok(TextureAtlas {
            dimensions,
            texture,
        })
    }

    pub fn from_image(
        device: &RenderDevice,
        queue: &RenderQueue,
        atlas_dimensions: &Dimensions<u32>,
        image: &Image,
    ) -> Result<Self, AtlasError> {
        let texture = Texture::from_image(device, queue, &image.image);

        Ok(TextureAtlas {
            dimensions: atlas_dimensions.clone(),
            texture,
        })
    }
}

#[derive(Debug)]
pub enum AtlasError {
    NonUniformDimensions,
    InputEmpty,
}

impl Display for AtlasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonUniformDimensions => {
                write!(f, "cannot build atlas texture with non uniform dimensions")
            }
            Self::InputEmpty => {
                write!(f, "input was empty")
            }
        }
    }
}

impl std::error::Error for AtlasError {}

/// Handle to a GPU texture. Provides a [`wgpu::TextureView`] and [`wgpu::Sampler`].
pub struct DepthTexture {
    pub tex: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl DepthTexture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn new(device: &RenderDevice, config: &RenderConfig, label: &str) -> Self {
        trace!("creating new depth texture: {:?}, {:?}", config, device);
        let size = wgpu::Extent3d {
            width: config.width() as u32,
            height: config.height() as u32,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let tex = device.create_texture(&desc);

        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self { tex, view, sampler }
    }
}
