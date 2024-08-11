// use crate::{
//     camera::Camera,
//     mesh2d::{Mesh2d, Points},
//     render_pipeline::{bind_group, buffer::AsGpuBuffer},
//     AsBindGroup, AsVertexBuffer, BindGroup, FragTexture, FragmentShader, Modulation, RenderEncoder,
//     RenderPipeline2d, RenderView, SamplerFilterType, Texture, Transform, VertTexture, Vertex,
//     VertexBuffer, VertexLayout, VertexShader,
// };
// use app::{
//     core::{AppSchedule, Schedule},
//     plugins::Plugin,
//     render_util::RenderContext,
//     window::ViewPort,
// };
// use ecs::*;
// use ecs::{WinnyAsEgui, WinnyResource};
// use math::vector::{Vec2f, Vec3f, Vec4f};
// use wgpu::core::pipeline;
//
// #[derive(Debug)]
// pub struct Lighting2dPlugin;
//
// impl Plugin for Lighting2dPlugin {
//     fn build(&mut self, app: &mut app::prelude::App) {
//         app.egui_resource::<AmbientLight>()
//             .register_resource::<Lighting2dPipeline>()
//             .register_resource::<AmbientLight>()
//             .add_systems(Schedule::StartUp, startup)
//             .add_systems(AppSchedule::PreRender, prepare_render_pass)
//             .add_systems(AppSchedule::RenderLighting, render_pass);
//     }
// }
//
// fn startup(mut commands: Commands, context: Res<RenderContext>) {
//     let ambient = AmbientLight {
//         color: Modulation(Vec4f::new(0.0, 0.15, 0.15, 0.11)),
//     };
//     commands.insert_resource(ambient);
//     commands.run_system_once_when(build_pipeline, can_build_pipeline);
// }
//
// fn build_pipeline(
//     mut commands: Commands,
//     context: Res<RenderContext>,
//     camera: Query<Camera>,
//     ambient: Res<AmbientLight>,
// ) {
//     let camera = camera.get_single().unwrap();
//
//     commands.insert_resource(Lighting2dPipeline::new(
//         &context,
//         &ambient,
//         camera.viewport_or_window(&context),
//     ));
// }
//
// fn can_build_pipeline(camera: Query<Camera>) -> bool {
//     camera.get_single().is_ok()
// }
//
// #[cfg(feature = "widgets")]
// fn register_resources(mut registry: ResMut<ecs::egui_widget::EguiRegistery>) {
//     registry.register_resource::<AmbientLight>();
// }
//
// #[derive(WinnyResource)]
// pub struct Lighting2dPipeline {
//     occlusion: RenderPipeline2d,
//     copy_to_view: RenderPipeline2d,
//     occluders: VertexBuffer,
//     lights: BindGroup,
//     ambient: BindGroup,
//     light_texture: BindGroup,
//     num_lights: u32,
// }
//
// impl Lighting2dPipeline {
//     pub fn new(context: &RenderContext, ambient: &AmbientLight, viewport: ViewPort) -> Self {
//         const MAX_LIGHTS: u64 = 64;
//         const MAX_OCCLUDERS: u64 = 128;
//
//         let lights = <&[Light] as AsBindGroup>::as_entire_binding_empty(
//             context,
//             &[],
//             std::mem::size_of::<Light>() as u64 * MAX_LIGHTS,
//             wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
//         );
//
//         let occluders = <Vertex as AsVertexBuffer<0>>::as_entire_buffer_empty(
//             context,
//             48,
//             wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
//         );
//
//         let ambient = <&[AmbientLight] as AsBindGroup>::as_entire_binding(
//             context,
//             &[*ambient],
//             wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
//         );
//
//         let (vert, frag) = occlude_shaders(context);
//
//         let occlusion = RenderPipeline2d::new(
//             "occlusion",
//             context,
//             &[ambient.layout(), lights.layout()],
//             &[occluders.layout()],
//             &vert,
//             &frag,
//             wgpu::BlendState::ALPHA_BLENDING,
//             None,
//         );
//
//         let _light_texture = Texture::empty(
//             viewport.dimensions_u32(),
//             context,
//             wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
//             context.config.format(),
//         );
//         let light_texture = FragTexture(&_light_texture);
//         let light_texture = <FragTexture as AsBindGroup>::as_entire_binding(
//             context,
//             light_texture,
//             SamplerFilterType::Linear,
//         );
//
//         let (vert, frag) = copy_shaders(context);
//
//         let copy_to_view = RenderPipeline2d::new(
//             "copy to view",
//             context,
//             &[light_texture.layout()],
//             &[],
//             &vert,
//             &frag,
//             wgpu::BlendState::ALPHA_BLENDING,
//             None,
//         );
//
//         Lighting2dPipeline {
//             occlusion,
//             copy_to_view,
//             occluders,
//             lights,
//             ambient,
//             light_texture,
//             num_lights: 1,
//         }
//     }
// }
//
// fn occlude_shaders(context: &RenderContext) -> (VertexShader, FragmentShader) {
//     (
//         VertexShader({
//             let shader = wgpu::ShaderModuleDescriptor {
//                 label: None,
//                 source: wgpu::ShaderSource::Wgsl(
//                     include_str!("../../../res/shaders/lighting2d.wgsl").into(),
//                 ),
//             };
//             context.device.create_shader_module(shader)
//         }),
//         FragmentShader({
//             let shader = wgpu::ShaderModuleDescriptor {
//                 label: None,
//                 source: wgpu::ShaderSource::Wgsl(
//                     include_str!("../../../res/shaders/lighting2d.wgsl").into(),
//                 ),
//             };
//             context.device.create_shader_module(shader)
//         }),
//     )
// }
//
// fn copy_shaders(context: &RenderContext) -> (VertexShader, FragmentShader) {
//     (
//         VertexShader({
//             let shader = wgpu::ShaderModuleDescriptor {
//                 label: None,
//                 source: wgpu::ShaderSource::Wgsl(
//                     include_str!("../../../res/shaders/copy_texture.wgsl").into(),
//                 ),
//             };
//             context.device.create_shader_module(shader)
//         }),
//         FragmentShader({
//             let shader = wgpu::ShaderModuleDescriptor {
//                 label: None,
//                 source: wgpu::ShaderSource::Wgsl(
//                     include_str!("../../../res/shaders/copy_texture.wgsl").into(),
//                 ),
//             };
//             context.device.create_shader_module(shader)
//         }),
//     )
// }
//
// #[repr(C)]
// #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct Light {
//     position: Vec3f,
//     /// In clip space.
//     radius: f32,
//     color: Vec4f,
// }
//
// unsafe impl AsGpuBuffer for Light {}
//
// impl AsBindGroup for &[Light] {
//     const LABEL: &'static str = "lights";
//     const BINDING_TYPES: &'static [wgpu::BindingType] = &[bind_group::UNIFORM];
//     const VISIBILITY: &'static [wgpu::ShaderStages] = &[wgpu::ShaderStages::VERTEX];
// }
//
// #[repr(C)]
// #[derive(WinnyResource, WinnyAsEgui, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct AmbientLight {
//     color: Modulation,
// }
//
// unsafe impl AsGpuBuffer for AmbientLight {}
//
// impl AsBindGroup for &[AmbientLight] {
//     const LABEL: &'static str = "ambient light";
//     const BINDING_TYPES: &'static [wgpu::BindingType] = &[bind_group::UNIFORM];
//     const VISIBILITY: &'static [wgpu::ShaderStages] = &[wgpu::ShaderStages::VERTEX];
// }
//
// #[derive(WinnyComponent, WinnyAsEgui, Debug, Clone)]
// pub struct Occluder {
//     pub mesh: Mesh2d,
// }
//
// fn prepare_render_pass(
//     pipeline: Res<Lighting2dPipeline>,
//     context: Res<RenderContext>,
//     ambient: Res<AmbientLight>,
//     occluders: Query<(Occluder, Transform)>,
// ) {
//     Light::write_buffer(
//         &context,
//         pipeline.lights.single_buffer(),
//         &[Light {
//             position: Vec3f::zero(),
//             radius: 0.5,
//             color: Vec4f::new(1., 0., 1., 0.1),
//         }],
//     );
//
//     // let occluders = occluders
//     //     .iter()
//     //     .map(|(o, t)| {
//     //         let mut o = o.clone();
//     //         for point in o.quad.p.iter_mut() {
//     //             let mut p = Vec4f::new(point.x, point.y, 0.0, 1.0);
//     //             p = t.as_matrix() * p;
//     //             point.x = p.x;
//     //             point.y = p.y;
//     //         }
//     //
//     //         o
//     //     })
//     //     .collect::<Vec<_>>();
//
//     let mut points = Points::default();
//     points.add(Vec2f::new(0., 0.));
//     points.add(Vec2f::new(1., 0.));
//     points.add(Vec2f::new(0., 1.));
//     points.add(Vec2f::new(0.5, 0.5));
//     // let verts = Mesh2d::from_points(points).unwrap().as_verts();
//     // Vertex::write_buffer(&context, pipeline.occluders.buffer(), &verts);
//
//     AmbientLight::write_buffer(&context, pipeline.ambient.single_buffer(), &[*ambient]);
// }
//
// fn render_pass(
//     mut encoder: ResMut<RenderEncoder>,
//     view: Res<RenderView>,
//     pipeline: Res<Lighting2dPipeline>,
// ) {
//     {
//         let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//             label: Some("clear"),
//             color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//                 view: pipeline.light_texture.single_texture_view(),
//                 resolve_target: None,
//                 ops: wgpu::Operations {
//                     load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
//                     store: wgpu::StoreOp::Store,
//                 },
//             })],
//             depth_stencil_attachment: None,
//             occlusion_query_set: None,
//             timestamp_writes: None,
//         });
//     }
//
//     {
//         let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//             label: Some("occluders"),
//             color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//                 view: pipeline.light_texture.single_texture_view(),
//                 resolve_target: None,
//                 ops: wgpu::Operations {
//                     load: wgpu::LoadOp::Load,
//                     store: wgpu::StoreOp::Store,
//                 },
//             })],
//             depth_stencil_attachment: None,
//             occlusion_query_set: None,
//             timestamp_writes: None,
//         });
//
//         render_pass.set_pipeline(&pipeline.occlusion.0);
//         render_pass.set_vertex_buffer(0, pipeline.occluders.buffer().slice(..));
//         render_pass.set_bind_group(0, &pipeline.ambient.binding(), &[]);
//         render_pass.set_bind_group(1, &pipeline.lights.binding(), &[]);
//         render_pass.draw(0..3, 0..1);
//     }
//
//     {
//         let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//             label: Some("draw to output"),
//             color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//                 view: &view,
//                 resolve_target: None,
//                 ops: wgpu::Operations {
//                     load: wgpu::LoadOp::Load,
//                     store: wgpu::StoreOp::Store,
//                 },
//             })],
//             depth_stencil_attachment: None,
//             occlusion_query_set: None,
//             timestamp_writes: None,
//         });
//
//         render_pass.set_pipeline(&pipeline.copy_to_view.0);
//         render_pass.set_bind_group(0, &pipeline.light_texture.binding(), &[]);
//         render_pass.draw(0..3, 0..1);
//     }
// }
