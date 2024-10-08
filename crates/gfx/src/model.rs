// use std::{
//     future::Future,
//     io::{BufReader, Cursor},
//     path::Path,
//     sync::Arc,
// };
//
// use std::ops::Range;
//
// use app::chrono::Offset;
// use asset::{load_binary, load_string, reader::ByteReader, AssetApp, AssetLoaderError};
// use image::GenericImageView;
// use render::{RenderContext, RenderDevice, RenderQueue};
// use wgpu::util::DeviceExt;
//
// use util::tracing::trace;
//
// use crate::{render_pipeline::vertex::VertexLayout, texture::Texture};
//
// #[derive(Debug, Clone, Copy)]
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
//
// #[repr(C)]
// #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// #[allow(dead_code)]
// pub struct InstanceRaw {
//     model: [[f32; 4]; 4],
//     normal: [[f32; 3]; 3],
// }
//
// impl<const Offset: u32> VertexLayout for InstanceRaw {
//     fn layout() -> wgpu::VertexBufferLayout<'static> {
//         use std::mem;
//         wgpu::VertexBufferLayout {
//             array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Instance,
//             attributes: &[
//                 wgpu::VertexAttribute {
//                     offset: 0,
//                     shader_location: Offset,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
//                     shader_location: Offset + 1,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
//                     shader_location: Offset + 2,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
//                     shader_location: Offset + 3,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
//                     shader_location: Offset + 4,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
//                     shader_location: Offset + 5,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 22]>() as wgpu::BufferAddress,
//                     shader_location: Offset + 6,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//             ],
//         }
//     }
// }
//
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
// impl<const Offset: u32> VertexLayout<Offset> for ModelVertex {
//     fn layout() -> wgpu::VertexBufferLayout<'static> {
//         use std::mem;
//         wgpu::VertexBufferLayout {
//             array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Vertex,
//             attributes: &[
//                 wgpu::VertexAttribute {
//                     offset: 0,
//                     shader_location: Offset,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
//                     shader_location: Offset + 1,
//                     format: wgpu::VertexFormat::Float32x2,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
//                     shader_location: Offset + 2,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
//                     shader_location: Offset + 3,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
//                     shader_location: Offset + 4,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//             ],
//         }
//     }
// }
//
// pub struct Model {
//     pub meshes: Vec<Mesh>,
//     pub materials: Vec<Material>,
// }
//
// #[cfg(target_arch = "wasm32")]
// unsafe impl Send for Model {}
// #[cfg(target_arch = "wasm32")]
// unsafe impl Sync for Model {}
//
// pub struct Material {
//     pub name: String,
//     pub diffuse_texture: Texture,
//     pub normal_texture: NormalTexture,
//     pub bind_group: wgpu::BindGroup,
// }
//
// impl Material {
//     pub fn new(
//         device: &wgpu::Device,
//         name: &str,
//         diffuse_texture: Texture,
//         normal_texture: NormalTexture,
//         layout: &wgpu::BindGroupLayout,
//     ) -> Self {
//         let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
//             layout,
//             entries: &[
//                 wgpu::BindGroupEntry {
//                     binding: 0,
//                     resource: wgpu::BindingResource::TextureView(diffuse_texture.view()),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 1,
//                     resource: wgpu::BindingResource::Sampler(diffuse_texture.sampler()),
//                 },
//                 // NEW!
//                 wgpu::BindGroupEntry {
//                     binding: 2,
//                     resource: wgpu::BindingResource::TextureView(normal_texture.view()),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 3,
//                     resource: wgpu::BindingResource::Sampler(normal_texture.sampler()),
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
//
// #[derive(Debug)]
// pub struct Mesh {
//     pub name: String,
//     pub vertex_buffer: wgpu::Buffer,
//     pub index_buffer: wgpu::Buffer,
//     pub num_elements: u32,
//     pub material: usize,
// }
//
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
//
// pub async fn load_normal(
//     file_name: &str,
//     device: &RenderDevice,
//     queue: &RenderQueue,
// ) -> Result<NormalTexture, ()> {
//     trace!("Loading texture: {:?}", file_name);
//     let data = load_binary(file_name).await.unwrap();
//     let img = image::load_from_memory(&data).map_err(|_| ())?;
//     Ok(NormalTexture::from_image(&img, device, queue))
// }
//
// async fn load_model(
//     path: String,
//     obj_string: Arc<String>,
//     device: &RenderDevice,
//     queue: &RenderQueue,
//     layout: &wgpu::BindGroupLayout,
// ) -> Result<Model, AssetLoaderError> {
//     trace!("Loading model: {}", path);
//     let obj_cursor = Cursor::new(obj_string.as_ref());
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
//             trace!("{:?}", &p);
//             let mat_text = load_string(&p.as_str()).await.unwrap();
//             tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
//         },
//     )
//     .await
//     .map_err(|_| AssetLoaderError::FailedToBuild)?;
//
//     let mut materials = Vec::new();
//     for m in obj_materials.unwrap() {
//         trace!("{:?}", &m);
//
//         if m.diffuse_texture.is_none() | m.normal_texture.is_none() | m.diffuse_texture.is_none() {
//             continue;
//         }
//
//         let diffuse_texture = crate::texture::Texture::load_texture(
//             Path::new::<String>(&m.diffuse_texture.unwrap())
//                 .file_name()
//                 .unwrap()
//                 .to_str()
//                 .unwrap(),
//             &device,
//             &queue,
//         )
//         .await
//         .map_err(|_| AssetLoaderError::FailedToBuild)?;
//         let normal_texture = load_normal(
//             Path::new::<String>(&m.normal_texture.unwrap())
//                 .file_name()
//                 .unwrap()
//                 .to_str()
//                 .unwrap(),
//             &device,
//             &queue,
//         )
//         .await
//         .map_err(|_| AssetLoaderError::FailedToBuild)?;
//
//         let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
//             layout: &layout,
//             entries: &[
//                 wgpu::BindGroupEntry {
//                     binding: 0,
//                     resource: wgpu::BindingResource::TextureView(diffuse_texture.view()),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 1,
//                     resource: wgpu::BindingResource::Sampler(diffuse_texture.sampler()),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 2,
//                     resource: wgpu::BindingResource::TextureView(normal_texture.view()),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 3,
//                     resource: wgpu::BindingResource::Sampler(normal_texture.sampler()),
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
//         });
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
//                 let uv0: cgmath::Vector2<f32> = v0.tex_coords.into();
//                 let uv1: cgmath::Vector2<f32> = v1.tex_coords.into();
//                 let uv2: cgmath::Vector2<f32> = v2.tex_coords.into();
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
//                 let v = &mut vertices[i];
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
//             let mesh = Mesh {
//                 name: path.to_string(),
//                 vertex_buffer,
//                 index_buffer,
//                 num_elements: m.mesh.indices.len() as u32,
//                 material: m.mesh.material_id.unwrap_or(usize::MAX),
//             };
//             trace!("{:?}", mesh);
//             mesh
//         })
//         .collect::<Vec<_>>();
//
//     Ok(Model { meshes, materials })
// }
//
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
//
// pub struct NormalTexture {
//     pub tex: wgpu::Texture,
//     pub view: wgpu::TextureView,
//     pub sampler: wgpu::Sampler,
// }
//
// impl NormalTexture {
//     pub fn from_image(
//         img: &image::DynamicImage,
//         device: &RenderDevice,
//         queue: &RenderQueue,
//     ) -> Self {
//         let rgba = img.to_rgba8();
//         let dimensions = img.dimensions();
//
//         let size = wgpu::Extent3d {
//             width: dimensions.0,
//             height: dimensions.1,
//             depth_or_array_layers: 1,
//         };
//
//         let tex = device.create_texture(&wgpu::TextureDescriptor {
//             label: None,
//             size,
//             mip_level_count: 1,
//             sample_count: 1,
//             dimension: wgpu::TextureDimension::D2,
//             // UPDATED!
//             format: wgpu::TextureFormat::Rgba8Unorm,
//             usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
//             view_formats: &[],
//         });
//
//         queue.write_texture(
//             wgpu::ImageCopyTexture {
//                 texture: &tex,
//                 mip_level: 0,
//                 origin: wgpu::Origin3d::ZERO,
//                 aspect: wgpu::TextureAspect::All,
//             },
//             // The actual pixel data
//             &rgba,
//             // The layout of the texture
//             wgpu::ImageDataLayout {
//                 offset: 0,
//                 bytes_per_row: Some(4 * dimensions.0),
//                 rows_per_image: Some(dimensions.1),
//             },
//             size,
//         );
//
//         let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
//         let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
//             address_mode_u: wgpu::AddressMode::ClampToEdge,
//             address_mode_v: wgpu::AddressMode::ClampToEdge,
//             address_mode_w: wgpu::AddressMode::ClampToEdge,
//             mag_filter: wgpu::FilterMode::Linear,
//             min_filter: wgpu::FilterMode::Nearest,
//             mipmap_filter: wgpu::FilterMode::Nearest,
//             ..Default::default()
//         });
//
//         Self { tex, view, sampler }
//     }
//
//     pub fn view(&self) -> &wgpu::TextureView {
//         &self.view
//     }
//
//     pub fn sampler(&self) -> &wgpu::Sampler {
//         &self.sampler
//     }
// }
//
// struct ModelAssetLoader;
//
// impl asset::AssetLoader for ModelAssetLoader {
//     type Asset = Model;
//
//     fn extensions(&self) -> Vec<&'static str> {
//         vec!["obj"]
//     }
//
//     fn load(
//         context: RenderContext,
//         reader: asset::reader::ByteReader<std::io::Cursor<Vec<u8>>>,
//         path: String,
//         ext: &str,
//     ) -> impl Future<Output = Result<Self::Asset, AssetLoaderError>> {
//         async move {
//             match ext {
//                 "obj" => {
//                     let source = ModelSource::new(reader)?;
//                     let layout =
//                         context
//                             .device
//                             .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//                                 entries: &[
//                                     wgpu::BindGroupLayoutEntry {
//                                         binding: 0,
//                                         visibility: wgpu::ShaderStages::FRAGMENT,
//                                         ty: wgpu::BindingType::Texture {
//                                             multisampled: false,
//                                             sample_type: wgpu::TextureSampleType::Float {
//                                                 filterable: true,
//                                             },
//                                             view_dimension: wgpu::TextureViewDimension::D2,
//                                         },
//                                         count: None,
//                                     },
//                                     wgpu::BindGroupLayoutEntry {
//                                         binding: 1,
//                                         visibility: wgpu::ShaderStages::FRAGMENT,
//                                         ty: wgpu::BindingType::Sampler(
//                                             wgpu::SamplerBindingType::Filtering,
//                                         ),
//                                         count: None,
//                                     },
//                                     // normal map
//                                     wgpu::BindGroupLayoutEntry {
//                                         binding: 2,
//                                         visibility: wgpu::ShaderStages::FRAGMENT,
//                                         ty: wgpu::BindingType::Texture {
//                                             multisampled: false,
//                                             sample_type: wgpu::TextureSampleType::Float {
//                                                 filterable: true,
//                                             },
//                                             view_dimension: wgpu::TextureViewDimension::D2,
//                                         },
//                                         count: None,
//                                     },
//                                     wgpu::BindGroupLayoutEntry {
//                                         binding: 3,
//                                         visibility: wgpu::ShaderStages::FRAGMENT,
//                                         ty: wgpu::BindingType::Sampler(
//                                             wgpu::SamplerBindingType::Filtering,
//                                         ),
//                                         count: None,
//                                     },
//                                 ],
//                                 label: Some("texture_bind_group_layout"),
//                             });
//                     load_model(
//                         path,
//                         source.string,
//                         &context.device,
//                         &context.queue,
//                         &layout,
//                     )
//                     .await
//                 }
//                 _ => Err(AssetLoaderError::UnsupportedFileExtension),
//             }
//         }
//     }
// }
//
// pub struct ModelPlugin;
//
// impl app::plugins::Plugin for ModelPlugin {
//     fn build(&mut self, app: &mut app::app::App) {
//         let loader = ModelAssetLoader {};
//         app.register_asset_loader::<Model>(loader);
//     }
// }
//
// pub struct ModelSource {
//     pub string: Arc<String>,
// }
//
// impl asset::Asset for Model {}
//
// impl ModelSource {
//     pub fn new(mut reader: ByteReader<Cursor<Vec<u8>>>) -> Result<Self, AssetLoaderError> {
//         // TODO: loading binary, then loading into string, then parsing...
//         let data = reader
//             .read_all_to_string()
//             .map_err(|_| AssetLoaderError::FailedToParse)?;
//
//         Ok(Self {
//             string: Arc::new(data),
//         })
//     }
// }
