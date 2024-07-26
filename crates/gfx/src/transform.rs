use app::window::ViewPort;
use cgmath::{Quaternion, Zero};
use ecs::WinnyComponent;
use winny_math::{
    matrix::{
        scale_matrix4x4f, translation_matrix4x4f, world_to_screen_space_matrix4x4f, Matrix4x4f,
    },
    vector::{Vec2f, Vec3f, Vec4f},
};

use crate::vertex::VertexLayout;

#[derive(WinnyComponent, Debug, Clone, Copy)]
pub struct Transform {
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

impl VertexLayout for Matrix4x4f {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Matrix4x4f>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

impl Transform {
    pub fn transformation_matrix(&self, viewport: &ViewPort, max_z: f32) -> Matrix4x4f {
        let width = viewport.max.v[0] - viewport.min.v[0];
        let height = viewport.max.v[1] - viewport.min.v[1];

        let scale = scale_matrix4x4f(self.scale);
        let rotation = cgmath::Matrix4::from(self.rotation);
        let rotation = Matrix4x4f { m: rotation.into() };
        let world_to_screen_space = world_to_screen_space_matrix4x4f(width, height, max_z);
        let translation =
            translation_matrix4x4f(world_to_screen_space * Vec4f::to_homogenous(self.translation));
        let offset_translation = translation_matrix4x4f(
            world_to_screen_space * Vec4f::new(viewport.min.v[0], viewport.min.v[1], 0., 1.),
        );

        scale * rotation * translation * offset_translation
    }
}
