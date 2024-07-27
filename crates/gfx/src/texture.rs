use std::{fmt::Display, future::Future, io::Cursor};

// use crate::sprite::{SpriteBinding, SpriteData};
use asset::{load_binary, reader::ByteReader, Asset, AssetApp, AssetLoaderError};

use ecs::WinnyComponent;
use image::{DynamicImage, GenericImageView};
use util::tracing::trace;

use render::{Dimensions, RenderConfig, RenderContext, RenderDevice, RenderQueue};

struct TextureAtlasLoader;

impl asset::AssetLoader for TextureAtlasLoader {
    type Asset = TextureAtlas;

    fn extensions(&self) -> Vec<&'static str> {
        vec!["png"]
    }

    fn load(
        _context: RenderContext,
        _reader: asset::reader::ByteReader<std::io::Cursor<Vec<u8>>>,
        _path: String,
        _ext: &str,
    ) -> impl Future<Output = Result<Self::Asset, AssetLoaderError>> {
        async move { unimplemented!() }
    }
}

struct ImageAssetLoader;

impl asset::AssetLoader for ImageAssetLoader {
    type Asset = Image;

    fn extensions(&self) -> Vec<&'static str> {
        vec!["png"]
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

    fn extensions(&self) -> Vec<&'static str> {
        vec!["png"]
    }

    fn load(
        context: RenderContext,
        reader: asset::reader::ByteReader<std::io::Cursor<Vec<u8>>>,
        path: String,
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
                        Some(path.as_str()),
                    ))
                }
                _ => Err(AssetLoaderError::UnsupportedFileExtension),
            }
        }
    }
}

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

#[derive(WinnyComponent)]
pub struct TextureDimensions(pub Dimensions);

impl TextureDimensions {
    pub fn from_texture(texture: &Texture) -> Self {
        Self(Dimensions(texture.tex.width(), texture.tex.height()))
    }

    pub fn from_texture_atlas(atlas: &TextureAtlas) -> Self {
        Self(Dimensions(
            atlas.texture.tex.width() * atlas.width,
            atlas.texture.tex.height() * atlas.height,
        ))
    }
}

#[derive(Debug)]
pub struct Texture {
    pub tex: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl asset::Asset for Texture {}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for Texture {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for Texture {}

impl Texture {
    pub async fn load_texture(
        file_name: &str,
        device: &RenderDevice,
        queue: &RenderQueue,
    ) -> Result<Texture, ()> {
        trace!("Loading texture: {:?}", file_name);
        let data = load_binary(file_name).await.map_err(|_| ())?;
        Texture::from_image_bytes(device, queue, &data, file_name)
    }

    pub fn from_image_bytes(
        device: &RenderDevice,
        queue: &RenderQueue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self, ()> {
        let img = image::load_from_memory(bytes).map_err(|_| ())?;
        Ok(Self::from_image(device, queue, &img, Some(label)))
    }

    pub fn from_image(
        device: &RenderDevice,
        queue: &RenderQueue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Self {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            tex: texture,
            view,
            sampler,
        }
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

        let diffuse_tex = device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("diffuse_texture"),
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &diffuse_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // The actual pixel data
            bytes,
            // The layout of the texture
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = diffuse_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            tex: diffuse_tex,
            view,
            sampler,
        }
    }

    pub fn empty(
        dimensions: (u32, u32),
        device: &RenderDevice,
        format: wgpu::TextureFormat,
        override_usage: Option<wgpu::TextureUsages>,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let diffuse_tex = device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: override_usage.unwrap_or_else(|| {
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST
            }),
            label: Some("diffuse_texture"),
            view_formats: &[],
        });

        let view = diffuse_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            tex: diffuse_tex,
            view,
            sampler,
        }
    }
}

#[derive(WinnyComponent, Debug)]
pub struct TextureAtlas {
    // Number of Textures horizontally
    pub width: u32,
    // Number of Textures vertically
    pub height: u32,
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

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1 * height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1 * height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture = Texture {
            tex: texture,
            view,
            sampler,
        };

        Ok(TextureAtlas {
            width,
            height,
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT // 3.
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let tex = device.create_texture(&desc);

        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            // 4.
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual), // 5.
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self { tex, view, sampler }
    }
}
