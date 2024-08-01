use render::RenderDevice;

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

pub fn create_uniform_bind_group(
    label: Option<&str>,
    device: &RenderDevice,
    buffer: &wgpu::Buffer,
    visibility: wgpu::ShaderStages,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label,
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });

    (layout, bg)
}

pub fn create_buffer_bind_group(
    label: Option<&str>,
    device: &RenderDevice,
    buffer: &wgpu::Buffer,
    binding_type: wgpu::BufferBindingType,
    visibility: wgpu::ShaderStages,
    binding: u32,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: binding_type,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label,
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding,
            resource: buffer.as_entire_binding(),
        }],
    });

    (layout, bg)
}

pub fn create_read_only_storage_bind_group(
    label: Option<&str>,
    device: &RenderDevice,
    buffer: &wgpu::Buffer,
    visibility: wgpu::ShaderStages,
    binding: u32,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label,
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding,
            resource: buffer.as_entire_binding(),
        }],
    });

    (layout, bg)
}

pub fn create_render_pipeline(
    label: &str,
    device: &RenderDevice,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModuleDescriptor,
    blend_alpha: bool,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(shader);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: vertex_layouts,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: if blend_alpha {
                    Some(wgpu::BlendState::ALPHA_BLENDING)
                } else {
                    Some(wgpu::BlendState::REPLACE)
                },
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        // cache: None,
    })
}

pub fn create_compute_pipeline(
    label: &str,
    device: &RenderDevice,
    layout: &wgpu::PipelineLayout,
    shader: wgpu::ShaderModuleDescriptor,
    entry_point: &str,
) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(shader);

    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        module: &shader,
        entry_point,
        compilation_options: wgpu::PipelineCompilationOptions::default(),
    })
}
