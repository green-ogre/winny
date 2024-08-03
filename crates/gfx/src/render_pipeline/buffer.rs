use app::render::RenderContext;
use bytemuck::{NoUninit, Pod};
use wgpu::util::DeviceExt;
use winny_math::{
    matrix::Matrix4x4f,
    vector::{Vec2f, Vec4f},
};

/// Must derive [`bytemuck::Pod`], [`bytemuck::Zeroable`], and be [`repr(C)`] while maintining
/// aligment as defined by WebGPU:
///
/// https://www.w3.org/TR/WGSL/#memory-layouts
pub unsafe trait AsGpuBuffer: Sized + NoUninit + Pod {
    fn create_buffer(
        label: Option<&'static str>,
        context: &RenderContext,
        size: u64,
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        context.device.create_buffer(&wgpu::BufferDescriptor {
            label,
            size,
            usage,
            mapped_at_creation: false,
        })
    }

    fn create_buffer_init(
        label: Option<&'static str>,
        context: &RenderContext,
        contents: &[Self],
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label,
                contents: bytemuck::cast_slice(contents),
                usage,
            })
    }

    fn write_buffer(context: &RenderContext, buffer: &wgpu::Buffer, contents: &[Self]) {
        context
            .queue
            .write_buffer(buffer, 0, bytemuck::cast_slice(contents));
    }
}

unsafe impl AsGpuBuffer for Matrix4x4f {}
unsafe impl AsGpuBuffer for Vec4f {}
unsafe impl AsGpuBuffer for Vec2f {}
unsafe impl AsGpuBuffer for f32 {}
unsafe impl AsGpuBuffer for u32 {}
