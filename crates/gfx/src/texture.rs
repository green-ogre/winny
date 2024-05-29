use std::path::PathBuf;

#[cfg(feature = "png")]
use logger::info;

#[cfg(feature = "png")]
use crate::png;

#[cfg(feature = "png")]
pub fn load_texture_png(
    file_name: PathBuf,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Texture, ()> {
    info!("Loading texture: {:?}", file_name);
    let (bytes, dimensions) = png::to_bytes(file_name).map_err(|err| logger::error!("{err}"))?;
    Ok(Texture::from_bytes(&bytes, dimensions, device, queue))
}

#[allow(unused)]
pub fn load_texture(
    file_name: PathBuf,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Texture, ()> {
    let Some(ext) = file_name.extension() else {
        logger::error!(
            "Could not determine image extension: {:?}",
            file_name.as_path()
        );
        return Err(());
    };

    match ext.to_str().unwrap_or_default() {
        #[cfg(feature = "png")]
        "png" => load_texture_png(file_name, device, queue),
        _ => {
            logger::error!("Cannot parse file format: {:?}", ext);
            return Err(());
        }
    }
}

#[derive(Debug)]
pub struct Texture {
    pub tex: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
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
}
