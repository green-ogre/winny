use std::{future::Future, io::Cursor};

// use crate::sprite::{SpriteBinding, SpriteData};
use asset::{load_binary, reader::ByteReader, AssetApp, AssetLoaderError};

use image::{DynamicImage, GenericImageView};
use util::tracing::info;

use render::{RenderContext, RenderDevice, RenderQueue};

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
                    let source = TextureSource::new(reader)?;
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

// #[derive(Debug, WinnyResource)]
// pub struct Textures {
//     storage: SparseSet<AssetId, Texture>,
// }
//
// impl Default for Textures {
//     fn default() -> Self {
//         Self {
//             storage: SparseSet::new(),
//         }
//     }
// }
//
// impl Textures {
//     pub fn insert(&mut self, handle: Handle<TextureSource>, texture: Texture) {
//         self.storage.insert(handle.id(), texture);
//     }
//
//     pub fn get(&self, handle: &Handle<TextureSource>) -> Option<&Texture> {
//         self.storage.get(&handle.id())
//     }
// }

// #[derive(Debug, WinnyEvent)]
// pub struct TextureCreated {
//     pub handle: Handle<TextureSource>,
// }

// fn insert_new_textures(
//     mut textures: ResMut<Textures>,
//     mut texture_created: EventWriter<TextureCreated>,
//     assets: Res<Assets<TextureSource>>,
//     events: EventReader<AssetLoaderEvent<TextureSource>>,
//     device: Res<RenderDevice>,
//     queue: Res<RenderQueue>,
// ) {
//     for event in events.read() {
//         match event {
//             AssetLoaderEvent::Loaded { handle } => {
//                 let tex_source = assets.get(&handle).unwrap();
//                 let texture =
//                     Texture::from_bytes(&tex_source.bytes, tex_source.dimensions, &device, &queue);
//                 textures.insert(handle.clone(), texture);
//                 texture_created.send(TextureCreated {
//                     handle: handle.clone(),
//                 })
//             }
//             AssetLoaderEvent::Err { .. } => {}
//         }
//     }
// }

pub struct TexturePlugin;

impl app::plugins::Plugin for TexturePlugin {
    fn build(&mut self, app: &mut app::app::App) {
        let loader = TextureAssetLoader {};
        app.register_asset_loader::<Texture>(loader);
    }
}

pub struct TextureSource {
    pub image: DynamicImage,
}

impl asset::Asset for Texture {}

impl TextureSource {
    pub fn new(mut reader: ByteReader<Cursor<Vec<u8>>>) -> Result<Self, AssetLoaderError> {
        let data = reader
            .read_all()
            .map_err(|_| AssetLoaderError::FailedToParse)?;
        let image = image::load_from_memory(&data).map_err(|_| AssetLoaderError::FailedToParse)?;

        Ok(Self { image })
    }
}

#[derive(Debug)]
pub struct Texture {
    pub tex: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

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
        info!("Loading texture: {:?}", file_name);
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

// #[derive(Debug, WinnyResource)]
// pub struct Sprites {
//     storage: SparseSet<AssetId, (Texture, SpriteBinding)>,
// }
//
// impl Sprites {
//     pub fn new() -> Self {
//         Self {
//             storage: SparseSet::new(),
//         }
//     }
//
//     pub fn insert(&mut self, handle: &Handle<SpriteData>, texture: Texture, bind: SpriteBinding) {
//         self.storage.insert(handle.id(), (texture, bind));
//     }
//
//     pub fn get_tex(&self, handle: &Handle<SpriteData>) -> Option<&Texture> {
//         self.storage.get(&handle.id()).map(|(t, _)| t)
//     }
//
//     pub fn get_tex_mut(&mut self, handle: &Handle<SpriteData>) -> Option<&mut Texture> {
//         self.storage.get_mut(&handle.id()).map(|(t, _)| t)
//     }
//
//     pub fn contains_key(&self, key: &AssetId) -> bool {
//         self.storage.contains_key(key)
//     }
//
//     pub fn iter_bindings(&self) -> impl Iterator<Item = &SpriteBinding> {
//         self.storage.values().iter().map(|(_, b)| b)
//     }
// }
