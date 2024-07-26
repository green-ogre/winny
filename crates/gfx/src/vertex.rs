use winny_math::vector::{Vec2f, Vec4f};

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
    pub position: Vec4f,
    pub uv: Vec2f,
    pub _padding: [f32; 2],
}

impl std::ops::Mul<Vec2f> for VertexUv {
    type Output = Self;

    fn mul(mut self, rhs: Vec2f) -> Self::Output {
        self.position.v[0] *= rhs.v[0];
        self.position.v[1] *= rhs.v[1];
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
            position: Vec4f::new(x, y, z, 1.0),
            uv: Vec2f::new(u, v),
            _padding: [0.0, 0.0],
        }
    }

    pub fn new_2d(position: Vec2f, uv: Vec2f) -> Self {
        Self {
            position: Vec4f::new(position.v[0], position.v[1], 0.0, 0.0),
            uv,
            _padding: [0.0, 0.0],
        }
    }
}

pub const FULLSCREEN_QUAD_VERTEX_UV: [VertexUv; 6] = [
    VertexUv {
        position: Vec4f {
            v: [-1.0, 1.0, 0.0, 1.0],
        },
        uv: Vec2f { v: [0.0, 0.0] },
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: Vec4f {
            v: [-1.0, -1.0, 0.0, 1.0],
        },
        uv: Vec2f { v: [0.0, 1.0] },
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: Vec4f {
            v: [1.0, -1.0, 0.0, 1.0],
        },
        uv: Vec2f { v: [1.0, 1.0] },
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: Vec4f {
            v: [-1.0, 1.0, 0.0, 1.0],
        },
        uv: Vec2f { v: [0.0, 0.0] },
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: Vec4f {
            v: [1.0, -1.0, 0.0, 1.0],
        },
        uv: Vec2f { v: [1.0, 1.0] },
        _padding: [0.0, 0.0],
    },
    VertexUv {
        position: Vec4f {
            v: [1.0, 1.0, 0.0, 1.0],
        },
        uv: Vec2f { v: [1.0, 0.0] },
        _padding: [0.0, 0.0],
    },
];
