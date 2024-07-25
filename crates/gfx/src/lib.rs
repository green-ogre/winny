use ecs::WinnyComponent;
use render::RenderDevice;
use winny_math::{
    matrix::Matrix4x4f,
    quaternion::Quaternion,
    vector::{Vec2f, Vec3f},
};

pub mod camera;
#[cfg(feature = "egui")]
pub mod editor;
#[cfg(feature = "egui")]
pub mod gui;
pub mod model;
pub mod prelude;
pub mod sprite;
#[cfg(feature = "text")]
pub mod text;
pub mod texture;
pub mod viewport;

pub extern crate bytemuck;
pub extern crate cgmath;
pub extern crate wgpu;
#[cfg(feature = "text")]
pub extern crate wgpu_text;

#[derive(WinnyComponent, Debug)]
pub struct Transform {
    pub translation: Vec3f,
    pub rotation: Quaternion,
    pub scale: Vec3f,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3f::zero(),
            rotation: Quaternion::zero(),
            scale: Vec3f::one(),
        }
    }
}

impl Transform {
    pub fn transformation_matrix(&self) -> Matrix4x4f {
        let rotation_matrix = self.rotation.rotation_matrix();
        let scale_matrix = self.scale.scale_matrix();
        let scaled_rotation_matrix = rotation_matrix * scale_matrix;
        let translation_matrix = self.translation.translation_matrix();

        translation_matrix * scaled_rotation_matrix
    }
}

pub fn create_texture_bind_group(
    label: Option<&str>,
    device: &RenderDevice,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
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
    });

    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });

    (layout, bg)
}

pub fn create_uniform_bind_group(
    label: Option<&str>,
    device: &RenderDevice,
    buffer: &wgpu::Buffer,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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

pub fn create_read_only_storage_bind_group(
    label: Option<&str>,
    device: &RenderDevice,
    buffer: &wgpu::Buffer,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });

    (layout, bg)
}

pub trait VertexLayout {
    fn layout() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 4],
}

impl VertexLayout for Vertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x4,
            }],
        }
    }
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z, 1.0],
        }
    }
}

pub const FULLSCREEN_QUAD_VERTEX: [Vertex; 6] = [
    Vertex {
        position: [-1.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [-1.0, -1.0, 0.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0, 0.0, 1.0],
    },
    Vertex {
        position: [-1.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0, 0.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0, 0.0, 1.0],
    },
];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexUv {
    pub position: [f32; 4],
    pub uv: [f32; 2],
    pub _padding: [f32; 2],
}

impl std::ops::Mul<Vec2f> for VertexUv {
    type Output = Self;

    fn mul(mut self, rhs: Vec2f) -> Self::Output {
        self.position[0] *= rhs.x;
        self.position[1] *= rhs.y;
        self
    }
}

impl VertexLayout for VertexUv {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<VertexUv>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

impl VertexUv {
    pub fn new(x: f32, y: f32, z: f32, u: f32, v: f32) -> Self {
        Self {
            position: [x, y, z, 1.0],
            uv: [u, v],
            _padding: [0.0, 0.0],
        }
    }

    pub fn new_2d(position: Vec2f, uv: Vec2f) -> Self {
        Self {
            position: [position.x, position.y, 0.0, 0.0],
            uv: [uv.x, uv.y],
            _padding: [0.0, 0.0],
        }
    }
}

pub const FULLSCREEN_QUAD_VERTEX_UV: [VertexUv; 6] = [
    VertexUv {
        position: [-1.0, 1.0, 0.0, 1.0],
        uv: [0.0, 0.0],
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: [-1.0, -1.0, 0.0, 1.0],
        uv: [0.0, 1.0],
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: [1.0, -1.0, 0.0, 1.0],
        uv: [1.0, 1.0],
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: [-1.0, 1.0, 0.0, 1.0],
        uv: [0.0, 0.0],
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: [1.0, -1.0, 0.0, 1.0],
        uv: [1.0, 1.0],
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: [1.0, 1.0, 0.0, 1.0],
        uv: [1.0, 0.0],
        _padding: [0.0, 0.0],
    },
];

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
