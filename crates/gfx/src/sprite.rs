use asset::{Asset, AssetLoader, Handle, LoadedAsset};

use self::renderer::Renderer;

use super::*;

#[derive(Debug, Clone, Copy)]
pub struct RGBA {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl RGBA {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn clear() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.0,
        }
    }

    pub fn white() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
}

#[derive(Debug, Clone, WinnyBundle)]
pub struct SpriteBundle {
    pub sprite: Sprite,
    pub handle: Handle<Sprite>,
}

#[derive(Debug, WinnyComponent, Clone, Copy)]
pub struct Sprite {
    pub scale: f32,
    pub rotation: f32,
    pub position: Vec2f,
    pub mask: RGBA,
    pub offset: Vec2f,
    pub v_flip: bool,
    pub z: f32,
}

impl Asset for Sprite {}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            scale: 1.0,
            rotation: 0.0,
            position: Vec2f::new(0.0, 0.0),
            mask: RGBA::clear(),
            offset: Vec2f::zero(),
            v_flip: false,
            z: 0.0,
        }
    }
}

impl Sprite {
    pub fn to_raw(&self, renderer: &Renderer) -> SpriteInstance {
        SpriteInstance {
            position: [
                self.position.x / renderer.virtual_size[0] as f32,
                self.position.y / renderer.virtual_size[0] as f32,
                self.z,
                0.0,
            ],
            mask: [self.mask.r, self.mask.g, self.mask.b, self.mask.a],
        }
    }

    pub fn to_vertices(&self) -> [SpriteVertex; 3] {
        let x = self.offset.x * self.scale;
        let y = self.offset.y * self.scale;

        if self.v_flip {
            [
                SpriteVertex::new(
                    Matrix2x2f::rotation_2d(Vec2f::new(-x, -y), self.rotation),
                    Vec2f::zero(),
                ),
                SpriteVertex::new(
                    Matrix2x2f::rotation_2d(Vec2f::new(-x, 2.0 * self.scale - y), self.rotation),
                    Vec2f::new(0.0, 2.0),
                ),
                SpriteVertex::new(
                    Matrix2x2f::rotation_2d(Vec2f::new(2.0 * self.scale - x, -y), self.rotation),
                    Vec2f::new(2.0, 0.0),
                ),
            ]
        } else {
            [
                SpriteVertex::new(
                    Matrix2x2f::rotation_2d(Vec2f::new(-x, -y), self.rotation),
                    Vec2f::new(0.0, 1.0),
                ),
                SpriteVertex::new(
                    Matrix2x2f::rotation_2d(Vec2f::new(-x, 2.0 * self.scale - y), self.rotation),
                    Vec2f::new(0.0, -1.0),
                ),
                SpriteVertex::new(
                    Matrix2x2f::rotation_2d(Vec2f::new(2.0 * self.scale - x, -y), self.rotation),
                    Vec2f::new(2.0, 1.0),
                ),
            ]
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 4],
    pub tex_coord: [f32; 2],
    pub _padding: [f32; 2],
}

impl SpriteVertex {
    pub fn new(position: Vec2f, tex_coord: Vec2f) -> Self {
        Self {
            position: [position.x, position.y, 0.0, 0.0],
            tex_coord: [tex_coord.x, tex_coord.y],
            _padding: [0.0, 0.0],
        }
    }
}

impl Vertex for SpriteVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[derive(Debug)]
pub struct SpriteData {
    pub bytes: Vec<u8>,
    pub dimensions: (u32, u32),
}

impl Asset for SpriteData {}

impl SpriteData {
    // pub fn write_to_texture(&self, dimensions: (u32, u32), data: &[u8], renderer: &Renderer) {
    //     let size = wgpu::Extent3d {
    //         width: dimensions.0,
    //         height: dimensions.1,
    //         depth_or_array_layers: 1,
    //     };

    //     renderer.queue.write_texture(
    //         self.texture.tex.as_image_copy(),
    //         &data,
    //         wgpu::ImageDataLayout {
    //             offset: 0,
    //             bytes_per_row: Some(4 * dimensions.0),
    //             rows_per_image: Some(dimensions.1),
    //         },
    //         size,
    //     );
    //     renderer.queue.submit([]);
    // }
}

#[derive(Debug, WinnyComponent)]
pub struct SpriteBinding {
    pub bind_group: wgpu::BindGroup,
}

impl SpriteBinding {
    pub fn from_texture(texture: &Texture, renderer: &Renderer) -> Self {
        let layout = renderer
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("bind group layout for sprite"),
            });

        let bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
                label: Some("bind group for sprite"),
            });

        Self { bind_group }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    position: [f32; 4],
    mask: [f32; 4],
}

impl Vertex for SpriteInstance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SpriteInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct SpriteAssetLoader;

impl AssetLoader for SpriteAssetLoader {
    type Asset = SpriteData;

    fn extensions(&self) -> Vec<String> {
        // TODO: feature flags
        vec!["png".into()]
    }

    fn load(
        reader: asset::reader::ByteReader<std::fs::File>,
        ext: &str,
    ) -> Result<asset::LoadedAsset<Self::Asset>, ()> {
        let (bytes, dimensions) =
            png::to_bytes(reader).map_err(|err| logger::error!("{:?}", err))?;
        Ok(LoadedAsset::new(SpriteData { bytes, dimensions }))
    }
}
