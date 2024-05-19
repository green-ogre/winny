use std::{
    env::current_dir,
    f32::consts::PI,
    io::{BufReader, Cursor},
    ops::Range,
    path::{Path, PathBuf},
};

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use cgmath::{Angle, InnerSpace, Quaternion, Rad, Rotation3, Vector3};
use ecs::{
    Bundle, Component, InternalBundle, InternalComponent, InternalTypeGetter, Resource, TypeGetter,
    WinnyComponent, WinnyResource, WinnyTypeGetter,
};
use logger::{info, warn};
use winny_math::{Matrix2x2f, Vec2f};

use self::texture::{DiffuseTexture, NormalTexture};

pub mod bitmap;
pub mod camera;
pub mod gui;
pub mod renderer;
pub mod texture;

pub use renderer::*;

pub extern crate egui;

// pub const NUM_INSTANCES_PER_ROW: u32 = 10;
// pub const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(
//     NUM_INSTANCES_PER_ROW as f32 * 0.5,
//     0.0,
//     NUM_INSTANCES_PER_ROW as f32 * 0.5,
// );

#[derive(Debug, WinnyResource, TypeGetter)]
pub struct DeltaT(pub f64);

#[derive(Debug)]
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

#[derive(Debug, InternalBundle)]
pub struct SpriteBundle {
    pub sprite: Sprite,
    pub sprite_binding: SpriteBinding,
}

#[derive(Debug, InternalComponent, TypeGetter)]
pub struct Sprite {
    pub scale: f32,
    pub rotation: f32,
    pub position: Vec2f,
    pub mask: RGBA,
    pub offset: Vec2f,
    pub z: f32,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            scale: 1.0,
            rotation: 0.0,
            position: Vec2f::new(0.0, 0.0),
            mask: RGBA::clear(),
            offset: Vec2f::zero(),
            z: 0.0,
        }
    }
}

impl Sprite {
    pub fn to_raw(&self, renderer: &Renderer) -> SpriteInstance {
        // let rot = cgmath::Matrix4::from(Quaternion::from_axis_angle(
        //     Vector3::new(0.0, 0.0, 1.0),
        //     Rad(self.rotation * PI / 180.0),
        // ));

        SpriteInstance {
            // position: cgmath::Matrix4::from_translation(cgmath::Vector3 {
            //     x: self.position.x * 2.0 / renderer.virtual_size[0] as f32,
            //     y: self.position.y * 2.0 / renderer.virtual_size[1] as f32,
            //     z: 0.0,
            // }) * rot)
            //     .into(),
            position: [
                self.position.x / renderer.virtual_size[0] as f32,
                self.position.y / renderer.virtual_size[0] as f32,
                self.z,
                0.0,
            ],
            mask: [self.mask.r, self.mask.g, self.mask.b, self.mask.a],
        }
    }

    pub fn to_vertices(&self) -> Vec<SpriteVertex> {
        let x = self.offset.x * self.scale;
        let y = self.offset.y * self.scale;

        vec![
            SpriteVertex::new(
                Matrix2x2f::rotation_2d(Vec2f::new(-x, -y), self.rotation),
                Vec2f::zero(),
            ),
            SpriteVertex::new(
                Matrix2x2f::rotation_2d(Vec2f::new(-x, self.scale - y), self.rotation),
                Vec2f::new(0.0, 1.0),
            ),
            SpriteVertex::new(
                Matrix2x2f::rotation_2d(Vec2f::new(self.scale - x, -y), self.rotation),
                Vec2f::new(1.0, 0.0),
            ),
            SpriteVertex::new(
                Matrix2x2f::rotation_2d(Vec2f::new(-x, self.scale - y), self.rotation),
                Vec2f::new(0.0, 1.0),
            ),
            SpriteVertex::new(
                Matrix2x2f::rotation_2d(Vec2f::new(self.scale - x, self.scale - y), self.rotation),
                Vec2f::new(1.0, 1.0),
            ),
            SpriteVertex::new(
                Matrix2x2f::rotation_2d(Vec2f::new(self.scale - x, -y), self.rotation),
                Vec2f::new(1.0, 0.0),
            ),
        ]
    }
}

#[derive(Debug, InternalComponent, TypeGetter)]
pub struct SpriteBinding {
    path: PathBuf,
}

#[derive(Debug)]
pub struct SpriteBindingRaw {
    texture: DiffuseTexture,
    bind_group: wgpu::BindGroup,
}

impl SpriteBindingRaw {
    pub fn initialize(path: &PathBuf, renderer: &Renderer) -> anyhow::Result<Self> {
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

        let texture = pollster::block_on(load_texture(
            path.clone(),
            &renderer.device,
            &renderer.queue,
        ))?;

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

        Ok(Self {
            texture,
            bind_group,
        })
    }
}

impl SpriteBinding {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
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

#[derive(Debug, InternalComponent, TypeGetter)]
pub struct Transform2D {
    t: Vec2f,
}

impl Transform2D {
    pub fn zero() -> Self {
        Self { t: Vec2f::zero() }
    }

    pub fn new(x: f32, y: f32) -> Self {
        Self {
            t: Vec2f::new(x, y),
        }
    }

    pub fn as_matrix(&self) -> [f32; 2] {
        self.t.as_matrix()
    }
}

// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct BoidVertex {
//     pub position: [f32; 3],
//     pub tex_coord: [f32; 2],
// }
//
// impl BoidVertex {
//     pub fn desc() -> wgpu::VertexBufferLayout<'static> {
//         use std::mem;
//         wgpu::VertexBufferLayout {
//             array_stride: mem::size_of::<BoidVertex>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Vertex,
//             attributes: &[
//                 wgpu::VertexAttribute {
//                     offset: 0,
//                     shader_location: 0,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
//                     shader_location: 1,
//                     format: wgpu::VertexFormat::Float32x2,
//                 },
//             ],
//         }
//     }
// }

// pub struct Instance {
//     pub position: cgmath::Vector3<f32>,
//     pub rotation: cgmath::Quaternion<f32>,
// }
//
// impl Instance {
//     pub fn to_raw(&self) -> InstanceRaw {
//         let model =
//             cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation);
//         InstanceRaw {
//             model: model.into(),
//             normal: cgmath::Matrix3::from(self.rotation).into(),
//         }
//     }
// }

// #[repr(C)]
// #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// #[allow(dead_code)]
// pub struct BoidRaw {
//     position: [[f32; 4]; 4],
//     color: [f32; 4],
//     num_friends: u32,
//     _padding: [f32; 3],
//     rotation: f32,
//     _padding2: [f32; 3],
// }
//
// impl Vertex for BoidRaw {
//     fn desc() -> wgpu::VertexBufferLayout<'static> {
//         use std::mem;
//         wgpu::VertexBufferLayout {
//             array_stride: mem::size_of::<BoidRaw>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Instance,
//             attributes: &[
//                 wgpu::VertexAttribute {
//                     offset: 0,
//                     shader_location: 2,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
//                     shader_location: 3,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
//                     shader_location: 4,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
//                     shader_location: 5,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
//                     shader_location: 6,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 20]>() as wgpu::BufferAddress,
//                     shader_location: 7,
//                     format: wgpu::VertexFormat::Uint32,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 24]>() as wgpu::BufferAddress,
//                     shader_location: 8,
//                     format: wgpu::VertexFormat::Float32,
//                 },
//             ],
//         }
//     }
// }

// #[repr(C)]
// #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// #[allow(dead_code)]
// pub struct InstanceRaw {
//     model: [[f32; 4]; 4],
//     normal: [[f32; 3]; 3],
// }
//
// impl Vertex for InstanceRaw {
//     fn desc() -> wgpu::VertexBufferLayout<'static> {
//         use std::mem;
//         wgpu::VertexBufferLayout {
//             array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Instance,
//             attributes: &[
//                 wgpu::VertexAttribute {
//                     offset: 0,
//                     shader_location: 5,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
//                     shader_location: 6,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
//                     shader_location: 7,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
//                     shader_location: 8,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
//                     shader_location: 9,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
//                     shader_location: 10,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 22]>() as wgpu::BufferAddress,
//                     shader_location: 11,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//             ],
//         }
//     }
// }

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct ModelVertex {
//     pub position: [f32; 3],
//     pub tex_coords: [f32; 2],
//     // The trifecta of black magic!
//     pub normal: [f32; 3],
//     pub tangent: [f32; 3],
//     pub bitangent: [f32; 3],
// }
//
// impl Vertex for ModelVertex {
//     fn desc() -> wgpu::VertexBufferLayout<'static> {
//         use std::mem;
//         wgpu::VertexBufferLayout {
//             array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Vertex,
//             attributes: &[
//                 wgpu::VertexAttribute {
//                     offset: 0,
//                     shader_location: 0,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
//                     shader_location: 1,
//                     format: wgpu::VertexFormat::Float32x2,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
//                     shader_location: 2,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
//                     shader_location: 3,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
//                     shader_location: 4,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//             ],
//         }
//     }
// }

// pub struct Model {
//     pub meshes: Vec<Mesh>,
//     pub materials: Vec<Material>,
// }
//
// pub struct Material {
//     pub name: String,
//     pub diffuse_texture: DiffuseTexture,
//     pub normal_texture: NormalTexture,
//     pub bind_group: wgpu::BindGroup,
// }
//
// impl Material {
//     pub fn new(
//         device: &wgpu::Device,
//         name: &str,
//         diffuse_texture: DiffuseTexture,
//         normal_texture: NormalTexture,
//         layout: &wgpu::BindGroupLayout,
//     ) -> Self {
//         let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
//             layout,
//             entries: &[
//                 wgpu::BindGroupEntry {
//                     binding: 0,
//                     resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 1,
//                     resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
//                 },
//                 // NEW!
//                 wgpu::BindGroupEntry {
//                     binding: 2,
//                     resource: wgpu::BindingResource::TextureView(&normal_texture.view),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 3,
//                     resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
//                 },
//             ],
//             label: Some(name),
//         });
//
//         Self {
//             name: String::from(name),
//             diffuse_texture,
//             normal_texture,
//             bind_group,
//         }
//     }
// }

// pub struct Mesh {
//     pub name: String,
//     pub vertex_buffer: wgpu::Buffer,
//     pub index_buffer: wgpu::Buffer,
//     pub num_elements: u32,
//     pub material: usize,
// }

// #[repr(C)]
// #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct PointLightUniform {
//     pub position: [f32; 3],
//     pub _padding: u32,
//     pub color: [f32; 3],
//     pub _padding2: u32,
// }
//
// impl PointLightUniform {
//     pub fn new(position: [f32; 3], color: [f32; 3]) -> Self {
//         Self {
//             position,
//             color,
//             _padding: 0,
//             _padding2: 0,
//         }
//     }
// }
//
// #[repr(C)]
// #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct PointLightStorage {
//     pub point_count: u32,
//     pub points: [PointLightUniform; 2],
// }

pub async fn load_binary(path: PathBuf) -> anyhow::Result<Vec<u8>> {
    let data = std::fs::read(path.as_os_str())?;

    Ok(data)
}

pub async fn load_string(path: &str) -> anyhow::Result<String> {
    let mut new_path = PathBuf::new();
    new_path.push("res/");
    new_path.push(path);
    let txt = std::fs::read_to_string(new_path.as_os_str())?;

    Ok(txt)
}

pub async fn load_texture(
    file_name: PathBuf,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<DiffuseTexture> {
    info!("Loading texture: {:?}", file_name);
    let data = load_binary(file_name).await?;
    DiffuseTexture::from_bytes(&data, device, queue)
}

pub async fn load_normal(
    file_name: PathBuf,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<NormalTexture> {
    info!("Loading texture: {:?}", file_name);
    let data = load_binary(file_name).await?;
    NormalTexture::from_bytes(&data, device, queue)
}

// pub async fn load_model(
//     path: &str,
//     device: &wgpu::Device,
//     queue: &wgpu::Queue,
//     layout: &wgpu::BindGroupLayout,
// ) -> Result<Model> {
//     info!("Loading model: {}", path);
//     let obj_text = load_string(path).await?;
//     let obj_cursor = Cursor::new(obj_text);
//     let mut obj_reader = BufReader::new(obj_cursor);
//
//     let (models, obj_materials) = tobj::load_obj_buf_async(
//         &mut obj_reader,
//         &tobj::LoadOptions {
//             triangulate: true,
//             single_index: true,
//             ..Default::default()
//         },
//         |p| async move {
//             let mat_text = load_string(&p).await.unwrap();
//             tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
//         },
//     )
//     .await?;
//
//     let mut materials = Vec::new();
//     for m in obj_materials? {
//         let diffuse_texture = load_texture(&m.diffuse_texture, device, queue).await?;
//         let normal_texture = load_normal(&m.normal_texture, device, queue).await?;
//
//         let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
//             layout,
//             entries: &[
//                 wgpu::BindGroupEntry {
//                     binding: 0,
//                     resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 1,
//                     resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 2,
//                     resource: wgpu::BindingResource::TextureView(&normal_texture.view),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 3,
//                     resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
//                 },
//             ],
//             label: None,
//         });
//
//         materials.push(Material {
//             name: m.name,
//             diffuse_texture,
//             normal_texture,
//             bind_group,
//         })
//     }
//
//     let meshes = models
//         .into_iter()
//         .map(|m| {
//             let mut vertices = (0..m.mesh.positions.len() / 3)
//                 .map(|i| ModelVertex {
//                     position: [
//                         m.mesh.positions[i * 3],
//                         m.mesh.positions[i * 3 + 1],
//                         m.mesh.positions[i * 3 + 2],
//                     ],
//                     tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
//                     normal: [
//                         m.mesh.normals[i * 3],
//                         m.mesh.normals[i * 3 + 1],
//                         m.mesh.normals[i * 3 + 2],
//                     ],
//                     // We'll calculate these later
//                     tangent: [0.0; 3],
//                     bitangent: [0.0; 3],
//                 })
//                 .collect::<Vec<_>>();
//
//             let indices = &m.mesh.indices;
//             let mut triangles_included = vec![0; vertices.len()];
//
//             // Calculate tangents and bitangets. We're going to
//             // use the triangles, so we need to loop through the
//             // indices in chunks of 3
//             for c in indices.chunks(3) {
//                 let v0 = vertices[c[0] as usize];
//                 let v1 = vertices[c[1] as usize];
//                 let v2 = vertices[c[2] as usize];
//
//                 let pos0: cgmath::Vector3<_> = v0.position.into();
//                 let pos1: cgmath::Vector3<_> = v1.position.into();
//                 let pos2: cgmath::Vector3<_> = v2.position.into();
//
//                 let uv0: cgmath::Vec2f<_> = v0.tex_coords.into();
//                 let uv1: cgmath::Vec2f<_> = v1.tex_coords.into();
//                 let uv2: cgmath::Vec2f<_> = v2.tex_coords.into();
//
//                 // Calculate the edges of the triangle
//                 let delta_pos1 = pos1 - pos0;
//                 let delta_pos2 = pos2 - pos0;
//
//                 // This will give us a direction to calculate the
//                 // tangent and bitangent
//                 let delta_uv1 = uv1 - uv0;
//                 let delta_uv2 = uv2 - uv0;
//
//                 // Solving the following system of equations will
//                 // give us the tangent and bitangent.
//                 //     delta_pos1 = delta_uv1.x * T + delta_u.y * B
//                 //     delta_pos2 = delta_uv2.x * T + delta_uv2.y * B
//                 // Luckily, the place I found this equation provided
//                 // the solution!
//                 let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
//                 let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
//                 // We flip the bitangent to enable right-handed normal
//                 // maps with wgpu texture coordinate system
//                 let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * -r;
//
//                 // We'll use the same tangent/bitangent for each vertex in the triangle
//                 vertices[c[0] as usize].tangent =
//                     (tangent + cgmath::Vector3::from(vertices[c[0] as usize].tangent)).into();
//                 vertices[c[1] as usize].tangent =
//                     (tangent + cgmath::Vector3::from(vertices[c[1] as usize].tangent)).into();
//                 vertices[c[2] as usize].tangent =
//                     (tangent + cgmath::Vector3::from(vertices[c[2] as usize].tangent)).into();
//                 vertices[c[0] as usize].bitangent =
//                     (bitangent + cgmath::Vector3::from(vertices[c[0] as usize].bitangent)).into();
//                 vertices[c[1] as usize].bitangent =
//                     (bitangent + cgmath::Vector3::from(vertices[c[1] as usize].bitangent)).into();
//                 vertices[c[2] as usize].bitangent =
//                     (bitangent + cgmath::Vector3::from(vertices[c[2] as usize].bitangent)).into();
//
//                 // Used to average the tangents/bitangents
//                 triangles_included[c[0] as usize] += 1;
//                 triangles_included[c[1] as usize] += 1;
//                 triangles_included[c[2] as usize] += 1;
//             }
//
//             // Average the tangents/bitangents
//             for (i, n) in triangles_included.into_iter().enumerate() {
//                 let denom = 1.0 / n as f32;
//                 let mut v = &mut vertices[i];
//                 v.tangent = (cgmath::Vector3::from(v.tangent) * denom).into();
//                 v.bitangent = (cgmath::Vector3::from(v.bitangent) * denom).into();
//             }
//
//             let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//                 label: Some(&format!("{:?} Vertex Buffer", path)),
//                 contents: bytemuck::cast_slice(&vertices),
//                 usage: wgpu::BufferUsages::VERTEX,
//             });
//             let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//                 label: Some(&format!("{:?} Index Buffer", path)),
//                 contents: bytemuck::cast_slice(&m.mesh.indices),
//                 usage: wgpu::BufferUsages::INDEX,
//             });
//
//             Mesh {
//                 name: path.to_string(),
//                 vertex_buffer,
//                 index_buffer,
//                 num_elements: m.mesh.indices.len() as u32,
//                 material: m.mesh.material_id.unwrap_or(0),
//             }
//         })
//         .collect::<Vec<_>>();
//
//     Ok(Model { meshes, materials })
// }
//
// // model.rs
// pub trait DrawModel<'a> {
//     fn draw_mesh(
//         &mut self,
//         mesh: &'a Mesh,
//         material: &'a Material,
//         camera_bind_group: &'a wgpu::BindGroup,
//         light_bind_group: &'a wgpu::BindGroup,
//     );
//     fn draw_mesh_instanced(
//         &mut self,
//         mesh: &'a Mesh,
//         material: &'a Material,
//         instances: Range<u32>,
//         camera_bind_group: &'a wgpu::BindGroup,
//         light_bind_group: &'a wgpu::BindGroup,
//     );
//
//     fn draw_model(
//         &mut self,
//         model: &'a Model,
//         camera_bind_group: &'a wgpu::BindGroup,
//         light_bind_group: &'a wgpu::BindGroup,
//     );
//     fn draw_model_instanced(
//         &mut self,
//         model: &'a Model,
//         instances: Range<u32>,
//         camera_bind_group: &'a wgpu::BindGroup,
//         light_bind_group: &'a wgpu::BindGroup,
//     );
//     fn draw_model_instanced_with_material(
//         &mut self,
//         model: &'a Model,
//         material: &'a Material,
//         instances: Range<u32>,
//         camera_bind_group: &'a wgpu::BindGroup,
//         light_bind_group: &'a wgpu::BindGroup,
//     );
// }
//
// impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
// where
//     'b: 'a,
// {
//     fn draw_mesh(
//         &mut self,
//         mesh: &'b Mesh,
//         material: &'b Material,
//         camera_bind_group: &'b wgpu::BindGroup,
//         light_bind_group: &'b wgpu::BindGroup,
//     ) {
//         self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group, light_bind_group);
//     }
//
//     fn draw_mesh_instanced(
//         &mut self,
//         mesh: &'b Mesh,
//         material: &'b Material,
//         instances: Range<u32>,
//         camera_bind_group: &'b wgpu::BindGroup,
//         light_bind_group: &'b wgpu::BindGroup,
//     ) {
//         self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
//         self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
//         self.set_bind_group(0, &material.bind_group, &[]);
//         self.set_bind_group(1, camera_bind_group, &[]);
//         self.set_bind_group(2, light_bind_group, &[]);
//         self.draw_indexed(0..mesh.num_elements, 0, instances);
//     }
//
//     fn draw_model(
//         &mut self,
//         model: &'b Model,
//         camera_bind_group: &'b wgpu::BindGroup,
//         light_bind_group: &'b wgpu::BindGroup,
//     ) {
//         self.draw_model_instanced(model, 0..1, camera_bind_group, light_bind_group);
//     }
//
//     fn draw_model_instanced(
//         &mut self,
//         model: &'b Model,
//         instances: Range<u32>,
//         camera_bind_group: &'b wgpu::BindGroup,
//         light_bind_group: &'b wgpu::BindGroup,
//     ) {
//         for mesh in &model.meshes {
//             let material = &model.materials[mesh.material];
//             self.draw_mesh_instanced(
//                 mesh,
//                 material,
//                 instances.clone(),
//                 camera_bind_group,
//                 light_bind_group,
//             );
//         }
//     }
//
//     fn draw_model_instanced_with_material(
//         &mut self,
//         model: &'b Model,
//         material: &'b Material,
//         instances: Range<u32>,
//         camera_bind_group: &'b wgpu::BindGroup,
//         light_bind_group: &'b wgpu::BindGroup,
//     ) {
//         for mesh in &model.meshes {
//             self.draw_mesh_instanced(
//                 mesh,
//                 material,
//                 instances.clone(),
//                 camera_bind_group,
//                 light_bind_group,
//             );
//         }
//     }
// }
//
// // model.rs
// pub trait DrawLight<'a> {
//     fn draw_light_mesh(
//         &mut self,
//         mesh: &'a Mesh,
//         camera_bind_group: &'a wgpu::BindGroup,
//         light_bind_group: &'a wgpu::BindGroup,
//     );
//     fn draw_light_mesh_instanced(
//         &mut self,
//         mesh: &'a Mesh,
//         instances: Range<u32>,
//         camera_bind_group: &'a wgpu::BindGroup,
//         light_bind_group: &'a wgpu::BindGroup,
//     );
//
//     fn draw_light_model(
//         &mut self,
//         model: &'a Model,
//         camera_bind_group: &'a wgpu::BindGroup,
//         light_bind_group: &'a wgpu::BindGroup,
//     );
//     fn draw_light_model_instanced(
//         &mut self,
//         model: &'a Model,
//         instances: Range<u32>,
//         camera_bind_group: &'a wgpu::BindGroup,
//         light_bind_group: &'a wgpu::BindGroup,
//     );
// }
//
// impl<'a, 'b> DrawLight<'b> for wgpu::RenderPass<'a>
// where
//     'b: 'a,
// {
//     fn draw_light_mesh(
//         &mut self,
//         mesh: &'b Mesh,
//         camera_bind_group: &'b wgpu::BindGroup,
//         light_bind_group: &'b wgpu::BindGroup,
//     ) {
//         self.draw_light_mesh_instanced(mesh, 0..1, camera_bind_group, light_bind_group);
//     }
//
//     fn draw_light_mesh_instanced(
//         &mut self,
//         mesh: &'b Mesh,
//         instances: Range<u32>,
//         camera_bind_group: &'b wgpu::BindGroup,
//         light_bind_group: &'b wgpu::BindGroup,
//     ) {
//         self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
//         self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
//         self.set_bind_group(0, camera_bind_group, &[]);
//         self.set_bind_group(1, light_bind_group, &[]);
//         self.draw_indexed(0..mesh.num_elements, 0, instances);
//     }
//
//     fn draw_light_model(
//         &mut self,
//         model: &'b Model,
//         camera_bind_group: &'b wgpu::BindGroup,
//         light_bind_group: &'b wgpu::BindGroup,
//     ) {
//         self.draw_light_model_instanced(model, 0..1, camera_bind_group, light_bind_group);
//     }
//     fn draw_light_model_instanced(
//         &mut self,
//         model: &'b Model,
//         instances: Range<u32>,
//         camera_bind_group: &'b wgpu::BindGroup,
//         light_bind_group: &'b wgpu::BindGroup,
//     ) {
//         for mesh in &model.meshes {
//             self.draw_light_mesh_instanced(
//                 mesh,
//                 instances.clone(),
//                 camera_bind_group,
//                 light_bind_group,
//             );
//         }
//     }
// }