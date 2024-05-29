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
//
// pub async fn load_normal(
//     file_name: PathBuf,
//     device: &wgpu::Device,
//     queue: &wgpu::Queue,
// ) -> Result<NormalTexture> {
//     info!("Loading texture: {:?}", file_name);
//     let data = load_binary(file_name).await?;
//     NormalTexture::from_bytes(&data, device, queue)
// }

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
//
// pub struct NormalTexture {
//     pub tex: wgpu::Texture,
//     pub view: wgpu::TextureView,
//     pub sampler: wgpu::Sampler,
// }
//
// impl NormalTexture {
//     pub fn from_image_bytes(
//         img_bytes: &[u8],
//         device: &wgpu::Device,
//         queue: &wgpu::Queue,
//     ) -> Result<Self, ()> {
//         let img = image::load_from_memory(img_bytes).map_err(|err| {
//             logger::error!("Could not read image from bytes: {}", err);
//             ()
//         })?;
//         Ok(Self::from_image(&img, device, queue))
//     }
//
//     pub fn from_image(
//         img: &image::DynamicImage,
//         device: &wgpu::Device,
//         queue: &wgpu::Queue,
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
// }
//
// pub struct DepthTexture {
//     pub tex: wgpu::Texture,
//     pub view: wgpu::TextureView,
//     pub sampler: wgpu::Sampler,
// }
//
// impl DepthTexture {
//     pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, label: &str) -> Self {
//         let size = wgpu::Extent3d {
//             width: config.width,
//             height: config.height,
//             depth_or_array_layers: 1,
//         };
//         let desc = wgpu::TextureDescriptor {
//             label: Some(label),
//             size,
//             mip_level_count: 1,
//             sample_count: 1,
//             dimension: wgpu::TextureDimension::D2,
//             format: wgpu::TextureFormat::Depth32Float,
//             usage: wgpu::TextureUsages::RENDER_ATTACHMENT // 3.
//                 | wgpu::TextureUsages::TEXTURE_BINDING,
//             view_formats: &[],
//         };
//         let tex = device.create_texture(&desc);
//
//         let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
//         let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
//             // 4.
//             address_mode_u: wgpu::AddressMode::ClampToEdge,
//             address_mode_v: wgpu::AddressMode::ClampToEdge,
//             address_mode_w: wgpu::AddressMode::ClampToEdge,
//             mag_filter: wgpu::FilterMode::Nearest,
//             min_filter: wgpu::FilterMode::Nearest,
//             mipmap_filter: wgpu::FilterMode::Nearest,
//             compare: Some(wgpu::CompareFunction::LessEqual), // 5.
//             lod_min_clamp: 0.0,
//             lod_max_clamp: 100.0,
//             ..Default::default()
//         });
//
//         Self { tex, view, sampler }
//     }
// }
//
// // Vertex shader
//
// struct Camera {
//     view_pos: vec4<f32>,
//     view_proj: mat4x4<f32>,
// }
// @group(1) @binding(0)
// var<uniform> camera: Camera;
//
// struct Light {
//     position: vec3<f32>,
//     color: vec3<f32>,
// }
// @group(2) @binding(0)
// var<uniform> light: Light;
//
// struct VertexInput {
//     @location(0) position: vec3<f32>,
//     @location(1) tex_coords: vec2<f32>,
//     @location(2) normal: vec3<f32>,
//     @location(3) tangent: vec3<f32>,
//     @location(4) bitangent: vec3<f32>,
// }
// struct InstanceInput {
//     @location(5) model_matrix_0: vec4<f32>,
//     @location(6) model_matrix_1: vec4<f32>,
//     @location(7) model_matrix_2: vec4<f32>,
//     @location(8) model_matrix_3: vec4<f32>,
//     @location(9) normal_matrix_0: vec3<f32>,
//     @location(10) normal_matrix_1: vec3<f32>,
//     @location(11) normal_matrix_2: vec3<f32>,
// }
//
// struct VertexOutput {
//     @builtin(position) clip_position: vec4<f32>,
//     @location(0) tex_coords: vec2<f32>,
//     @location(1) tangent_position: vec3<f32>,
//     @location(2) tangent_light_position: vec3<f32>,
//     @location(3) tangent_view_position: vec3<f32>,
// }
//
// @vertex
// fn vs_main(
//     model: VertexInput,
//     instance: InstanceInput,
// ) -> VertexOutput {
//     let model_matrix = mat4x4<f32>(
//         instance.model_matrix_0,
//         instance.model_matrix_1,
//         instance.model_matrix_2,
//         instance.model_matrix_3,
//     );
//     let normal_matrix = mat3x3<f32>(
//         instance.normal_matrix_0,
//         instance.normal_matrix_1,
//         instance.normal_matrix_2,
//     );
//
//     // Construct the tangent matrix
//     let world_normal = normalize(normal_matrix * model.normal);
//     let world_tangent = normalize(normal_matrix * model.tangent);
//     let world_bitangent = normalize(normal_matrix * model.bitangent);
//     let tangent_matrix = transpose(mat3x3<f32>(
//         world_tangent,
//         world_bitangent,
//         world_normal,
//     ));
//
//     let world_position = model_matrix * vec4<f32>(model.position, 1.0);
//
//     var out: VertexOutput;
//     out.clip_position = camera.view_proj * world_position;
//     out.tex_coords = model.tex_coords;
//     out.tangent_position = tangent_matrix * world_position.xyz;
//     out.tangent_view_position = tangent_matrix * camera.view_pos.xyz;
//     out.tangent_light_position = tangent_matrix * light.position;
//     return out;
// }
//
// // Fragment shader
//
// @group(0) @binding(0)
// var t_diffuse: texture_2d<f32>;
// @group(0)@binding(1)
// var s_diffuse: sampler;
// @group(0)@binding(2)
// var t_normal: texture_2d<f32>;
// @group(0) @binding(3)
// var s_normal: sampler;
//
// @fragment
// fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
//     let object_color: vec4<f32> = textureSample(t_diffuse, s_diffuse, in.tex_coords);
//     let object_normal: vec4<f32> = textureSample(t_normal, s_normal, in.tex_coords);
//
//     // We don't need (or want) much ambient light, so 0.1 is fine
//     let ambient_strength = 0.1;
//     let ambient_color = light.color * ambient_strength;
//
//     // Create the lighting vectors
//     let tangent_normal = object_normal.xyz * 2.0 - 1.0;
//     let light_dir = normalize(in.tangent_light_position - in.tangent_position);
//     let view_dir = normalize(in.tangent_view_position - in.tangent_position);
//     let half_dir = normalize(view_dir + light_dir);
//
//     let diffuse_strength = max(dot(tangent_normal, light_dir), 0.0);
//     let diffuse_color = light.color * diffuse_strength;
//
//     let specular_strength = pow(max(dot(tangent_normal, half_dir), 0.0), 32.0);
//     let specular_color = specular_strength * light.color;
//
//     let result = (ambient_color + diffuse_color + specular_color) * object_color.xyz;
//
//     return vec4<f32>(result, object_color.a);
// }
//
//
// use std::f32::consts::FRAC_PI_2;
//
// use cgmath::perspective;
// use cgmath::prelude::*;
// use cgmath::Matrix4;
// use cgmath::Point3;
// use cgmath::Rad;
// use cgmath::SquareMatrix;
// use cgmath::Vector3;
// use ecs::WinnyResource;
// use plugins::Plugin;
//
// use crate::DeltaT;
//
// pub const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;
//
// #[rustfmt::skip]
// pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
//     1.0, 0.0, 0.0, 0.0,
//     0.0, 1.0, 0.0, 0.0,
//     0.0, 0.0, 0.5, 0.5,
//     0.0, 0.0, 0.0, 1.0,
// );
//
// #[derive(Debug, WinnyResource)]
// pub struct Camera {
//     pub position: Point3<f32>,
//     yaw: Rad<f32>,
//     pitch: Rad<f32>,
//     pub projection: Projection,
// }
//
// impl Camera {
//     pub fn new<V: Into<Point3<f32>>, Y: Into<Rad<f32>>, P: Into<Rad<f32>>>(
//         position: V,
//         yaw: Y,
//         pitch: P,
//         projection: Projection,
//     ) -> Self {
//         Self {
//             position: position.into(),
//             yaw: yaw.into(),
//             pitch: pitch.into(),
//             projection,
//         }
//     }
//
//     pub fn calc_matrix(&self) -> Matrix4<f32> {
//         let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
//         let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();
//
//         Matrix4::look_to_rh(
//             self.position,
//             Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
//             Vector3::unit_y(),
//         )
//     }
// }
//
// #[derive(Debug)]
// pub struct Projection {
//     aspect: f32,
//     fovy: Rad<f32>,
//     znear: f32,
//     zfar: f32,
// }
//
// impl Projection {
//     pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
//         Self {
//             aspect: width as f32 / height as f32,
//             fovy: fovy.into(),
//             znear,
//             zfar,
//         }
//     }
//
//     pub fn resize(&mut self, width: u32, height: u32) {
//         self.aspect = width as f32 / height as f32;
//     }
//
//     pub fn calc_matrix(&self) -> Matrix4<f32> {
//         OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
//     }
// }
//
// #[derive(Debug, WinnyResource)]
// pub struct CameraController {
//     pub amount_left: f32,
//     pub amount_right: f32,
//     pub amount_forward: f32,
//     pub amount_backward: f32,
//     pub amount_up: f32,
//     pub amount_down: f32,
//     pub rotate_horizontal: f32,
//     pub rotate_vertical: f32,
//     scroll: f32,
//     speed: f32,
//     sensitivity: f32,
// }
//
// pub struct Camera2D;
//
// impl Plugin for Camera2D {
//     fn build(&self, _world: &mut ecs::World, _scheduler: &mut ecs::Scheduler) {
//         // let projection = Projection::new(
//         //     renderer.config.width,
//         //     renderer.config.height,
//         //     cgmath::Deg(45.0),
//         //     0.1,
//         //     100.0,
//         // );
//
//         // world.insert_resource(Camera::new(
//         //     (0.0, 0.0, 0.0),
//         //     cgmath::Deg(-90.0),
//         //     cgmath::Deg(-20.0),
//         //     projection,
//         // ));
//     }
// }
//
// impl CameraController {
//     pub fn new(speed: f32, sensitivity: f32) -> Self {
//         Self {
//             amount_left: 0.0,
//             amount_right: 0.0,
//             amount_forward: 0.0,
//             amount_backward: 0.0,
//             amount_up: 0.0,
//             amount_down: 0.0,
//             rotate_horizontal: 0.0,
//             rotate_vertical: 0.0,
//             scroll: 0.0,
//             speed,
//             sensitivity,
//         }
//     }
//
//     pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
//         self.rotate_horizontal = mouse_dx as f32;
//         self.rotate_vertical = mouse_dy as f32;
//     }
//
//     pub fn update_camera(&mut self, camera: &mut Camera, dt: &DeltaT) -> () {
//         let dt = dt.0 as f32;
//
//         // Move forward/backward and left/right
//         let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
//         let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
//         let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
//         camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
//         camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;
//
//         // Move in/out (aka. "zoom")
//         // Note: this isn't an actual zoom. The camera's position
//         // changes when zooming. I've added this to make it easier
//         // to get closer to an object you want to focus on.
//         let (pitch_sin, pitch_cos) = camera.pitch.0.sin_cos();
//         let scrollward =
//             Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
//         camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
//         self.scroll = 0.0;
//
//         // Move up/down. Since we don't use roll, we can just
//         // modify the y coordinate directly.
//         camera.position.y += (self.amount_up - self.amount_down) * self.speed * dt;
//
//         // Rotate
//         camera.yaw += Rad(self.rotate_horizontal) * self.sensitivity * dt;
//         camera.pitch += Rad(-self.rotate_vertical) * self.sensitivity * dt;
//
//         // If process_mouse isn't called every frame, these values
//         // will not get set to zero, and the camera will rotate
//         // when moving in a non-cardinal direction.
//         self.rotate_horizontal = 0.0;
//         self.rotate_vertical = 0.0;
//
//         // Keep the camera's angle from going too high/low.
//         if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
//             camera.pitch = -Rad(SAFE_FRAC_PI_2);
//         } else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
//             camera.pitch = Rad(SAFE_FRAC_PI_2);
//         }
//     }
// }

// #[repr(C)]
// #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct CameraUniform {
//     view_position: [f32; 4],
//     view_proj: [[f32; 4]; 4],
// }
//
// impl CameraUniform {
//     pub fn new() -> Self {
//         Self {
//             view_position: [0.0; 4],
//             view_proj: [[0.0; 4]; 4], //cgmath::Matrix4::identity().into(),
//         }
//     }
//
//     //pub fn update_view_proj(&mut self, camera: &Camera) {
//     // self.view_position = camera.position.to_homogeneous().into();
//     // self.view_proj = (camera.projection.calc_matrix() * camera.calc_matrix()).into();
//     // }
// }
