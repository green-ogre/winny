use crate::render_pipeline::vertex::VertexLayout;
use app::plugins::Plugin;
use cgmath::{Matrix4, Quaternion, Zero};
use ecs::{WinnyAsEgui, WinnyComponent};
use math::{
    matrix::{scale_matrix4x4f, translation_matrix4x4f, Matrix4x4f},
    vector::{Vec2f, Vec3f, Vec4f},
};

pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&mut self, app: &mut app::prelude::App) {
        app.egui_component::<Transform>();
    }
}

/// Position of an entity in world space.
#[derive(WinnyComponent, WinnyAsEgui, Debug, Clone, Copy)]
pub struct Transform {
    /// Translations are described in world space and converted to clip space on the GPU
    pub translation: Vec3f,
    pub rotation: Quaternion<f32>,
    pub scale: Vec2f,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3f::zero(),
            rotation: Quaternion::zero(),
            scale: Vec2f::one(),
        }
    }
}

impl<const OFFSET: u32> VertexLayout<OFFSET> for Matrix4x4f {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Matrix4x4f>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: OFFSET,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: OFFSET + 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: OFFSET + 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: OFFSET + 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

impl Transform {
    pub fn scale_matrix(&self) -> Matrix4x4f {
        scale_matrix4x4f(self.scale)
    }

    pub fn rotation_matrix(&self) -> Matrix4x4f {
        let rotation = Matrix4::from(self.rotation);
        Matrix4x4f { m: rotation.into() }
    }

    pub fn translation_matrix(&self) -> Matrix4x4f {
        translation_matrix4x4f(Vec4f::to_homogenous(self.translation))
    }

    pub fn as_matrix(&self) -> Matrix4x4f {
        // Apply translation first to allow local transformation about the origin of this Transform.
        self.translation_matrix() * self.scale_matrix() * self.rotation_matrix()
    }
}
