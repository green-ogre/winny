pub mod camera;
#[cfg(feature = "egui")]
pub mod editor;
#[cfg(feature = "egui")]
pub mod gui;
pub mod model;
pub mod noise;
pub mod particle;
pub mod prelude;
// pub mod primitives;
pub mod render;
pub mod render_pipeline;
pub mod sprite;
#[cfg(feature = "text")]
pub mod text;
pub mod texture;
pub mod transform;

pub extern crate bytemuck;
pub extern crate cgmath;
#[cfg(feature = "egui")]
pub extern crate egui;
pub extern crate wgpu;
#[cfg(feature = "text")]
pub extern crate wgpu_text;

// pub fn create_compute_pipeline(
//     label: &str,
//     device: &RenderDevice,
//     layout: &wgpu::PipelineLayout,
//     shader: wgpu::ShaderModuleDescriptor,
//     entry_point: &str,
// ) -> wgpu::ComputePipeline {
//     let shader = device.create_shader_module(shader);
//
//     device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
//         label: Some(label),
//         layout: Some(layout),
//         module: &shader,
//         entry_point,
//         compilation_options: wgpu::PipelineCompilationOptions::default(),
//     })
// }
