use cereal::{WinnyDeserialize, WinnySerialize};
use ecs::WinnyAsEgui;
use math::vector::{Vec2f, Vec3f, Vec4f};

pub const FULLSCREEN_QUAD_VERTEX_UV: [VertexUv; 6] = [
    VertexUv {
        position: Vec4f::new(-1.0, 1.0, 0.0, 1.0),
        uv: Vec2f::new(0.0, 0.0),
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: Vec4f::new(-1.0, -1.0, 0.0, 1.0),
        uv: Vec2f::new(0.0, 1.0),
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: Vec4f::new(1.0, -1.0, 0.0, 1.0),
        uv: Vec2f::new(1.0, 1.0),
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: Vec4f::new(-1.0, 1.0, 0.0, 1.0),
        uv: Vec2f::new(0.0, 0.0),
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: Vec4f::new(1.0, -1.0, 0.0, 1.0),
        uv: Vec2f::new(1.0, 1.0),
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: Vec4f::new(1.0, 1.0, 0.0, 1.0),
        uv: Vec2f::new(1.0, 0.0),
        _padding: [0.0, 0.0],
    },
];

pub trait VertexLayout<const OFFSET: u32> {
    fn layout() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(
    WinnyAsEgui,
    WinnySerialize,
    WinnyDeserialize,
    Default,
    Copy,
    Clone,
    Debug,
    bytemuck::Pod,
    bytemuck::Zeroable,
)]
pub struct Vertex {
    pub position: Vec4f,
}

impl<const OFFSET: u32> VertexLayout<OFFSET> for Vertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: OFFSET,
                format: wgpu::VertexFormat::Float32x4,
            }],
        }
    }
}

impl From<Vec2f> for Vertex {
    fn from(value: Vec2f) -> Self {
        Self {
            position: Vec4f::new(value.x, value.y, 0.0, 1.0),
        }
    }
}

impl From<Vec3f> for Vertex {
    fn from(value: Vec3f) -> Self {
        Self {
            position: Vec4f::to_homogenous(value),
        }
    }
}

impl From<Vec4f> for Vertex {
    fn from(value: Vec4f) -> Self {
        assert!(value.is_homogenous());

        Self { position: value }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct VertexUv {
    pub position: Vec4f,
    pub uv: Vec2f,
    _padding: [f32; 2],
}

impl<const OFFSET: u32> VertexLayout<OFFSET> for VertexUv {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<VertexUv>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: OFFSET,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: OFFSET + 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

impl VertexUv {
    pub fn new(position: Vec4f, uv: Vec2f) -> Self {
        Self {
            position,
            uv,
            _padding: [0.0, 0.0],
        }
    }
}
