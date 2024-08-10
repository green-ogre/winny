// use crate::{
//     AsBindGroup, AsWgpuResources, BindGroup, FragmentShaderSource, RenderPipeline2d, VertexShader,
// };
// use app::{core::Schedule, plugins::Plugin, render_util::RenderContext};
// use asset::server::AssetServer;
// use asset::{Assets, Handle};
// use ecs::system_param::SystemParam;
// use ecs::*;
// use std::marker::PhantomData;
//
// pub struct PostProcessingPlugin<P: PostProcessing>(PhantomData<P>);
//
// impl<P: PostProcessing> PostProcessingPlugin<P> {
//     pub fn new() -> Self {
//         Self(PhantomData)
//     }
// }
//
// impl<P: PostProcessing> Plugin for PostProcessingPlugin<P> {
//     fn build(&mut self, app: &mut app::prelude::App) {
//         app.add_systems(Schedule::StartUp, startup::<P>);
//     }
// }
//
// fn startup<P: PostProcessing>(mut commands: Commands) {
//     commands.run_system_once_when(build_post_processing_pipeline::<P>, can_build_pipeline);
// }
//
// fn build_post_processing_pipeline<P: PostProcessing>(
//     mut commands: Commands,
//     context: Res<RenderContext>,
//     binding: Res<P>,
//     state: P::BindingState,
//     shaders: Res<Assets<FragmentShaderSource>>,
//     server: Res<AssetServer>,
// ) {
//     let shader = shaders.get_mut(&binding.shader(&server)).unwrap();
//     commands.insert_resource(PostProcessingPipeline::new(
//         P::P_LABEL,
//         &context,
//         *binding,
//         &state,
//         &mut shader,
//     ));
// }
//
// fn can_build_pipeline<P: PostProcessing>(
//     p: Res<P>,
//     shaders: Res<Assets<FragmentShaderSource>>,
//     server: Res<AssetServer>,
// ) -> bool {
//     shaders.get(&p.shader(&server)).is_some()
// }
//
// pub trait PostProcessing: AsBindGroup + Resource {
//     const P_LABEL: &'static str;
//     type BindingState: SystemParam;
//
//     fn shader(&self, asset_server: &AssetServer) -> Handle<FragmentShaderSource>;
//     fn binding_state<'s>(&self, state: &Self::BindingState)
//         -> <Self as AsWgpuResources>::State<'s>;
//     fn as_binding<'s>(
//         self,
//         context: &RenderContext,
//         state: <Self as AsWgpuResources>::State<'s>,
//     ) -> BindGroup {
//         <Self as AsBindGroup>::as_entire_binding(context, self, state)
//     }
// }
//
// pub struct PostProcessingPipeline<B: PostProcessing> {
//     pipeline: RenderPipeline2d,
//     binding: BindGroup,
//     _phantom: PhantomData<B>,
// }
//
// impl<B: PostProcessing> PostProcessingPipeline<B> {
//     pub fn new(
//         label: &str,
//         context: &RenderContext,
//         binding: B,
//         state: &B::BindingState,
//         shader: &mut FragmentShaderSource,
//     ) -> Self {
//         let binding_state = binding.binding_state(state);
//         let binding = binding.as_binding(context, binding_state);
//
//         let vert_shader = VertexShader({
//             let shader = wgpu::ShaderModuleDescriptor {
//                 label: None,
//                 source: wgpu::ShaderSource::Wgsl(
//                     include_str!("../../../res/shaders/post_processing_vert.wgsl").into(),
//                 ),
//             };
//             context.device.create_shader_module(shader)
//         });
//
//         let pipeline = RenderPipeline2d::new(
//             label,
//             context,
//             &[binding.layout()],
//             &[],
//             &vert_shader,
//             &shader.shader(context),
//             wgpu::BlendState::ALPHA_BLENDING,
//             None,
//         );
//
//         Self {
//             pipeline,
//             binding,
//             _phantom: PhantomData,
//         }
//     }
// }
