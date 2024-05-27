use image::GenericImageView;
use logger::info;

// TODO: name this texture
#[derive(Debug)]
pub struct Texture {
    pub tex: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    // pub fn from_image(
    //     img_bytes: &[u8],
    //     device: &wgpu::Device,
    //     queue: &wgpu::Queue,
    // ) -> Result<Self, ()> {
    //     let img = image::load_from_memory(img_bytes).map_err(|err| {
    //         logger::error!("Could not read image from bytes: {}", err);
    //         ()
    //     })?;
    //     let rgba = img.to_rgba8();
    //     let dimensions = img.dimensions();

    //     Ok(Self::from_bytes(&rgba, dimensions, device, queue))
    // }

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
