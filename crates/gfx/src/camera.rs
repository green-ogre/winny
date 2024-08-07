use crate::render_pipeline::bind_group::{self, AsBindGroup};
use crate::{render_pipeline::buffer::AsGpuBuffer, transform::Transform};
use app::render_util::Dimensions;
use app::window::ViewPort;
use app::window::Window;
use ecs::{WinnyBundle, WinnyComponent, WinnyResource};
use math::matrix::Matrix4x4f;

#[derive(WinnyBundle, Default)]
pub struct Camera2dBundle {
    camera: Camera,
    transform: Transform,
}

/// Defines what [`ViewPort`] the world should be drawn to.
///
/// At the moment, only _one_ camera may exist at a time.
#[derive(WinnyComponent, Default)]
pub struct Camera {
    // Window viewport if None.
    pub viewport: Option<ViewPort>,
}

impl AsBindGroup for &[CameraUniform] {
    const LABEL: &'static str = "camera";
    const VISIBILITY: &'static [wgpu::ShaderStages] = &[wgpu::ShaderStages::VERTEX];
    const BINDING_TYPES: &'static [wgpu::BindingType] = &[bind_group::UNIFORM];
}

#[repr(C)]
#[derive(WinnyResource, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct CameraUniform {
    transform: Matrix4x4f,
    viewport_dimensions: Dimensions<f32>,
    window_dimensions: Dimensions<f32>,
}

impl CameraUniform {
    pub fn from_camera(camera: &Camera, transform: &Transform, window: &Window) -> Self {
        let viewport = camera.viewport.unwrap_or_else(|| window.viewport);
        let mut transform = transform.as_matrix();

        let viewport_center = viewport.center();
        let screen_center = window.viewport.center();
        let offset = screen_center - viewport_center;
        transform.m[0][3] += offset.x;
        transform.m[1][3] -= offset.y;

        // The y will already be flipped
        transform.m[0][3] *= -1.0;

        Self {
            transform,
            viewport_dimensions: Dimensions::new(viewport.width(), viewport.height()),
            window_dimensions: Dimensions::new(window.viewport.width(), window.viewport.height()),
        }
    }
}

unsafe impl AsGpuBuffer for Dimensions<f32> {}
unsafe impl AsGpuBuffer for Dimensions<u32> {}
unsafe impl AsGpuBuffer for CameraUniform {}
