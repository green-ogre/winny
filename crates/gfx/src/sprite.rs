use app::plugins::Plugin;
use ecs::Commands;
use render::{RenderLayer, RenderPass};

pub struct SpritePlugin;

impl Plugin for SpritePlugin {
    fn build(&mut self, app: &mut app::app::App) {
        app.add_systems(ecs::Schedule::StartUp, insert_render_pass);
    }
}

fn insert_render_pass(mut commands: Commands) {
    // commands.spawn(RenderPassBundle {
    //     layer: RenderLayer(0),
    //     pass: RenderPass::new(sprite_render_pass),
    // });
}

fn sprite_render_pass() {}

// use app::plugins::Plugin;
// use asset::{Asset, AssetApp, AssetLoader, Assets, Handle};
// use ecs::{Commands, Entity, IntoSystemStorage, Query, Res, ResMut, WinnyResource, With};
// use wgpu::util::DeviceExt;
//
// use render::{RenderConfig, RenderContext, Renderer};
// use winny_math::matrix::Matrix2x2f;
//
// use self::texture::Texture;
//
// use super::*;
//
// #[derive(Debug, Clone, Copy)]
// pub struct RGBA {
//     pub r: f32,
//     pub g: f32,
//     pub b: f32,
//     pub a: f32,
// }
//
// impl RGBA {
//     pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
//         Self { r, g, b, a }
//     }
//
//     pub fn clear() -> Self {
//         Self {
//             r: 1.0,
//             g: 1.0,
//             b: 1.0,
//             a: 0.0,
//         }
//     }
//
//     pub fn white() -> Self {
//         Self {
//             r: 1.0,
//             g: 1.0,
//             b: 1.0,
//             a: 1.0,
//         }
//     }
// }
//
// fn create_sprite_render_pipeline(
//     device: &wgpu::Device,
//     config: &wgpu::SurfaceConfiguration,
// ) -> wgpu::RenderPipeline {
//     let sprite_bind_group_layout =
//         device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//             entries: &[
//                 wgpu::BindGroupLayoutEntry {
//                     binding: 0,
//                     visibility: wgpu::ShaderStages::FRAGMENT,
//                     ty: wgpu::BindingType::Texture {
//                         multisampled: false,
//                         view_dimension: wgpu::TextureViewDimension::D2,
//                         sample_type: wgpu::TextureSampleType::Float { filterable: true },
//                     },
//                     count: None,
//                 },
//                 wgpu::BindGroupLayoutEntry {
//                     binding: 1,
//                     visibility: wgpu::ShaderStages::FRAGMENT,
//                     ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
//                     count: None,
//                 },
//             ],
//             label: Some("bind group layout for sprite"),
//         });
//
//     let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
//         label: Some("Render Pipeline Layout"),
//         bind_group_layouts: &[&sprite_bind_group_layout],
//         push_constant_ranges: &[],
//     });
//
//     let shader = wgpu::ShaderModuleDescriptor {
//         label: Some("Shader"),
//         source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite_shader.wgsl").into()),
//     };
//
//     crate::create_render_pipeline(
//         Some("sprites"),
//         &device,
//         &render_pipeline_layout,
//         config.format,
//         None,
//         &[SpriteVertex::desc(), SpriteInstance::desc()],
//         shader,
//         true,
//     )
// }
//
// const VERTICES: u32 = 3;
//
// #[derive(Debug, WinnyResource)]
// pub struct SpriteRenderer {
//     vertex_buffer: wgpu::Buffer,
//     sprite_buffer: wgpu::Buffer,
//     render_pipeline: wgpu::RenderPipeline,
//     num_sprites: u32,
// }
//
// impl SpriteRenderer {
//     pub fn new(device: &RenderDevice, config: &RenderConfig) -> Self {
//         let pipeline = create_sprite_render_pipeline(device, config);
//
//         let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("sprite vertexes"),
//             contents: bytemuck::cast_slice::<SpriteVertex, u8>(&[]),
//             usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
//         });
//
//         let sprite_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("sprite instances"),
//             contents: &[],
//             usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
//         });
//
//         (vertex_buffer, sprite_buffer, pipeline)
//     }
// }
//
// pub fn update_sprite_data(
//     sprites: Query<(Sprite, Handle<SpriteData>), With<SpriteIsBinded>>,
//     renderer: Option<ResMut<Renderer>>,
//     mut sprite_renderer: ResMut<SpriteRenderer>,
// ) {
//     let Some(renderer) = renderer else {
//         return;
//     };
//
//     let vertex_data: Vec<_> = sprites
//         .iter()
//         .map(|(s, _)| s.to_vertices())
//         .flatten()
//         .collect();
//     sprite_renderer.num_sprites = vertex_data.len() as u32 / VERTICES;
//     let vertex_data = bytemuck::cast_slice(&vertex_data);
//
//     let (vertex_buffer, sprite_buffer, _) = sprite_renderer.get_or_initialize(&renderer);
//
//     if vertex_data.len() == vertex_buffer.size() as usize {
//         renderer.queue.write_buffer(&vertex_buffer, 0, vertex_data);
//     } else {
//         *vertex_buffer = renderer
//             .device
//             .create_buffer_init(&wgpu::util::BufferInitDescriptor {
//                 label: Some("sprite vertex"),
//                 contents: vertex_data,
//                 usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
//             });
//     }
//
//     let sprite_data = sprites
//         .iter()
//         .map(|(s, _)| s.to_raw(&renderer))
//         .collect::<Vec<_>>();
//     let sprite_data = bytemuck::cast_slice(&sprite_data);
//
//     if sprite_data.len() == sprite_buffer.size() as usize {
//         renderer.queue.write_buffer(&sprite_buffer, 0, sprite_data);
//     } else {
//         *sprite_buffer = renderer
//             .device
//             .create_buffer_init(&wgpu::util::BufferInitDescriptor {
//                 label: Some("sprite instance"),
//                 contents: sprite_data,
//                 usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
//             });
//     }
// }
//
// fn render_sprites(
//     mut renderer: ResMut<Renderer>,
//     mut context: ResMut<RenderContext>,
//     mut sprite_renderer: ResMut<SpriteRenderer>,
//     textures: Res<Sprites>,
// ) {
//     let (vertex_buffer, sprite_buffer, pipeline) = sprite_renderer.get_or_initialize(&renderer);
//
//     let view = renderer.view();
//     let mut render_pass = context
//         .encoder()
//         .begin_render_pass(&wgpu::RenderPassDescriptor {
//             label: Some("sprites"),
//             color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//                 view,
//                 resolve_target: None,
//                 ops: wgpu::Operations {
//                     load: wgpu::LoadOp::Clear(wgpu::Color {
//                         r: 0.1,
//                         g: 0.1,
//                         b: 0.1,
//                         a: 1.0,
//                     }),
//                     store: wgpu::StoreOp::Store,
//                 },
//             })],
//             depth_stencil_attachment: None,
//             occlusion_query_set: None,
//             timestamp_writes: None,
//         });
//
//     let mut offset = 0;
//     for binding in textures.iter_bindings() {
//         render_pass.set_pipeline(pipeline);
//         render_pass.set_bind_group(0, &binding.bind_group, &[]);
//         render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
//         render_pass.set_vertex_buffer(1, sprite_buffer.slice(..));
//         render_pass.draw(
//             offset * VERTICES..offset * VERTICES + VERTICES,
//             offset..offset + 1,
//         );
//         offset += 1;
//     }
//
//     drop(render_pass);
//     context.finish_encoder();
// }
//
// #[derive(Debug, Clone, WinnyBundle)]
// pub struct SpriteBundle {
//     pub sprite: Sprite,
//     pub handle: Handle<SpriteData>,
// }
//
// pub fn bind_new_sprite_bundles(
//     sprites: Query<(Entity, Handle<SpriteData>), With<Sprite>>,
//     assets: ResMut<Assets<SpriteData>>,
//     mut textures: ResMut<Sprites>,
//     renderer: Option<ResMut<Renderer>>,
//     mut commands: Commands,
// ) {
//     let Some(renderer) = renderer else {
//         return;
//     };
//
//     for (entity, handle) in sprites.iter_mut() {
//         if !textures.contains_key(&handle.id()) {
//             if let Some(asset) = assets.get(&handle) {
//                 let texture = Texture::from_bytes(
//                     &asset.bytes,
//                     asset.dimensions,
//                     &renderer.device,
//                     &renderer.queue,
//                 );
//                 let binding = SpriteBinding::from_texture(&texture, &renderer);
//                 textures.insert(&handle, texture, binding);
//             }
//         }
//
//         commands.get_entity(entity).insert(SpriteIsBinded);
//     }
// }
//
// #[derive(Debug, WinnyComponent, Clone, Copy)]
// pub struct Sprite {
//     pub scale: f32,
//     pub rotation: f32,
//     pub position: Vec2f,
//     pub mask: RGBA,
//     pub offset: Vec2f,
//     pub v_flip: bool,
//     pub z: f32,
// }
//
// impl Asset for Sprite {}
//
// impl Default for Sprite {
//     fn default() -> Self {
//         Self {
//             scale: 1.0,
//             rotation: 0.0,
//             position: Vec2f::new(0.0, 0.0),
//             mask: RGBA::clear(),
//             offset: Vec2f::zero(),
//             v_flip: false,
//             z: 0.0,
//         }
//     }
// }
//
// impl Sprite {
//     pub fn to_raw(&self, config: &RenderConfig) -> SpriteInstance {
//         SpriteInstance {
//             position: [
//                 self.position.x / config.virtual_size[0] as f32,
//                 self.position.y / config.virtual_size[0] as f32,
//                 self.z,
//                 0.0,
//             ],
//             mask: [self.mask.r, self.mask.g, self.mask.b, self.mask.a],
//         }
//     }
//
//     pub fn to_vertices(&self) -> [VertexUv; 3] {
//         let x = self.offset.x * self.scale;
//         let y = self.offset.y * self.scale;
//
//         [
//             VertexUv::new_2d(
//                 Matrix2x2f::rotation_2d(Vec2f::new(-x, -y), self.rotation),
//                 Vec2f::zero(),
//             ),
//             VertexUv::new_2d(
//                 Matrix2x2f::rotation_2d(Vec2f::new(-x, 2.0 * self.scale - y), self.rotation),
//                 Vec2f::new(0.0, 2.0),
//             ),
//             VertexUv::new_2d(
//                 Matrix2x2f::rotation_2d(Vec2f::new(2.0 * self.scale - x, -y), self.rotation),
//                 Vec2f::new(2.0, 0.0),
//             ),
//         ]
//     }
// }
//
// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct SpriteVertex {
//     pub position: [f32; 4],
//     pub tex_coord: [f32; 2],
//     pub _padding: [f32; 2],
// }
//
// impl SpriteVertex {
//     pub fn new(position: Vec2f, tex_coord: Vec2f) -> Self {
//         Self {
//             position: [position.x, position.y, 0.0, 0.0],
//             tex_coord: [tex_coord.x, tex_coord.y],
//             _padding: [0.0, 0.0],
//         }
//     }
// }
//
// impl VertexLayout for SpriteVertex {
//     fn layout() -> wgpu::VertexBufferLayout<'static> {
//         use std::mem;
//         wgpu::VertexBufferLayout {
//             array_stride: mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Vertex,
//             attributes: &[
//                 wgpu::VertexAttribute {
//                     offset: 0,
//                     shader_location: 0,
//                     format: wgpu::VertexFormat::Float32x4,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
//                     shader_location: 1,
//                     format: wgpu::VertexFormat::Float32x2,
//                 },
//             ],
//         }
//     }
// }
//
// #[derive(Debug, WinnyComponent)]
// pub struct SpriteBinding {
//     pub bind_group: wgpu::BindGroup,
// }
//
// impl SpriteBinding {
//     pub fn from_texture(texture: &Texture, device: &RenderDevice) -> Self {
//         let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//             entries: &[
//                 wgpu::BindGroupLayoutEntry {
//                     binding: 0,
//                     visibility: wgpu::ShaderStages::FRAGMENT,
//                     ty: wgpu::BindingType::Texture {
//                         multisampled: false,
//                         view_dimension: wgpu::TextureViewDimension::D2,
//                         sample_type: wgpu::TextureSampleType::Float { filterable: true },
//                     },
//                     count: None,
//                 },
//                 wgpu::BindGroupLayoutEntry {
//                     binding: 1,
//                     visibility: wgpu::ShaderStages::FRAGMENT,
//                     ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
//                     count: None,
//                 },
//             ],
//             label: Some("bind group layout for sprite"),
//         });
//
//         let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
//             layout: &layout,
//             entries: &[
//                 wgpu::BindGroupEntry {
//                     binding: 0,
//                     resource: wgpu::BindingResource::TextureView(&texture.view),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 1,
//                     resource: wgpu::BindingResource::Sampler(&texture.sampler),
//                 },
//             ],
//             label: Some("bind group for sprite"),
//         });
//
//         Self { bind_group }
//     }
// }
//
// #[repr(C)]
// #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct SpriteInstance {
//     position: [f32; 4],
//     mask: [f32; 4],
// }
//
// impl VertexLayout for SpriteInstance {
//     fn layout() -> wgpu::VertexBufferLayout<'static> {
//         use std::mem;
//         wgpu::VertexBufferLayout {
//             array_stride: mem::size_of::<SpriteInstance>() as wgpu::BufferAddress,
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
//             ],
//         }
//     }
// }
//
// #[derive(Clone)]
// pub struct SpritePlugin;
//
// impl Plugin for SpritePlugin {
//     fn build(&mut self, app: &mut app::app::App) {
//         app.add_systems(ecs::Schedule::StartUp, startup)
//             .add_systems(
//                 ecs::Schedule::PostUpdate,
//                 (bind_new_sprite_bundles, update_sprite_data),
//             )
//             .add_systems(ecs::Schedule::Render, render_sprites);
//     }
// }
//
// fn startup(mut commands: Commands, device: Res<RenderDevice>, config: Res<RenderConfig>) {
//     let sprite_renderer = SpriteRenderer::new(&device, &config);
//     commands.insert_resource(sprite_renderer);
// }
