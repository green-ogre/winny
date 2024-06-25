use ecs::{WinnyBundle, WinnyComponent};

use winny_math::{matrix::Matrix2x2f, vector::Vec2f};

use self::texture::Texture;

#[cfg(feature = "egui")]
pub mod gui;
pub mod png;
pub mod prelude;
pub mod renderer;
pub mod sprite;
pub mod texture;

#[derive(Debug, WinnyComponent)]
pub struct Transform2D {
    t: Vec2f,
}

impl Transform2D {
    pub fn zero() -> Self {
        Self { t: Vec2f::zero() }
    }

    pub fn new(x: f32, y: f32) -> Self {
        Self {
            t: Vec2f::new(x, y),
        }
    }

    pub fn as_matrix(&self) -> [f32; 2] {
        self.t.as_matrix()
    }
}

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}
