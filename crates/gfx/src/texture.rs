use app::renderer::Renderer;
// use crate::sprite::{SpriteBinding, SpriteData};
use asset::{AssetApp, AssetId, AssetLoaderEvent, Assets, Handle};
use ecs::{EventReader, EventWriter, Res, ResMut, SparseSet, WinnyEvent, WinnyResource};

use image::GenericImageView;
use util::tracing::error;

use crate::model::load_binary;

struct TextureAssetLoader;

impl asset::AssetLoader for TextureAssetLoader {
    type Asset = TextureSource;

    fn extensions(&self) -> Vec<String> {
        vec!["png".into()]
    }

    fn load(
        reader: asset::reader::ByteReader<std::fs::File>,
        ext: &str,
    ) -> Result<Self::Asset, ()> {
        match ext {
            "png" => TextureSource::new(reader),
            _ => {
                error!("Cannot load texture from extension: {}", ext);
                Err(())
            }
        }
    }
}

#[derive(Debug, WinnyResource)]
pub struct Textures {
    storage: SparseSet<AssetId, Texture>,
}

impl Default for Textures {
    fn default() -> Self {
        Self {
            storage: SparseSet::new(),
        }
    }
}

impl Textures {
    pub fn insert(&mut self, handle: Handle<TextureSource>, texture: Texture) {
        self.storage.insert(handle.id(), texture);
    }

    pub fn get(&self, handle: &Handle<TextureSource>) -> Option<&Texture> {
        self.storage.get(&handle.id())
    }
}

#[derive(Debug, WinnyEvent)]
pub struct TextureCreated {
    pub handle: Handle<TextureSource>,
}

fn insert_new_textures(
    mut textures: ResMut<Textures>,
    mut texture_created: EventWriter<TextureCreated>,
    assets: Res<Assets<TextureSource>>,
    events: EventReader<AssetLoaderEvent<TextureSource>>,
    renderer: Res<Renderer>,
) {
    for event in events.read() {
        match event {
            AssetLoaderEvent::Loaded { handle } => {
                let tex_source = assets.get(&handle).unwrap();
                let texture = Texture::from_bytes(
                    &tex_source.bytes,
                    tex_source.dimensions,
                    &renderer.device(),
                    &renderer.queue(),
                );
                textures.insert(handle.clone(), texture);
                texture_created.send(TextureCreated {
                    handle: handle.clone(),
                })
            }
            AssetLoaderEvent::Err { .. } => {}
        }
    }
}

pub struct TexturePlugin;

impl app::plugins::Plugin for TexturePlugin {
    fn build(&mut self, app: &mut app::app::App) {
        let loader = TextureAssetLoader {};
        app.register_asset_loader::<TextureSource>(loader)
            .insert_resource(Textures::default())
            .register_event::<TextureCreated>()
            .add_systems(ecs::Schedule::PreUpdate, insert_new_textures);
    }
}

pub struct TextureSource {
    pub bytes: Vec<u8>,
    pub dimensions: (u32, u32),
}

impl asset::Asset for TextureSource {}

impl TextureSource {
    pub fn new(reader: asset::reader::ByteReader<std::fs::File>) -> Result<Self, ()> {
        let (bytes, dimensions) = crate::png::to_bytes(reader).map_err(|_| ())?;

        Ok(Self { bytes, dimensions })
    }
}

#[derive(Debug)]
pub struct Texture {
    pub tex: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub fn load_texture(
        file_name: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Texture, ()> {
        let data = load_binary(file_name).map_err(|_| ())?;
        Texture::from_image_bytes(device, queue, &data, file_name)
    }

    pub fn from_image_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self, ()> {
        let img = image::load_from_memory(bytes).map_err(|_| ())?;
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Result<Self, ()> {
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

        Ok(Self {
            tex: texture,
            view,
            sampler,
        })
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
        device: &wgpu::Device,
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
