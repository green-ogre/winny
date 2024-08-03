use super::{
    buffer::AsGpuBuffer,
    vertex::{Vertex, VertexLayout, VertexUv},
};
use app::render::RenderContext;
use winny_math::matrix::Matrix4x4f;

/// Handle to a GPU buffer with layout information. Obtained from [`AsVertexBuffer::as_entire_buffer`].
pub struct VertexBuffer {
    buffer: wgpu::Buffer,
    usage: wgpu::BufferUsages,
    layout: wgpu::VertexBufferLayout<'static>,
}

impl VertexBuffer {
    pub fn new(
        buffer: wgpu::Buffer,
        usage: wgpu::BufferUsages,
        layout: wgpu::VertexBufferLayout<'static>,
    ) -> Self {
        Self {
            buffer,
            usage,
            layout,
        }
    }

    pub fn layout(&self) -> wgpu::VertexBufferLayout<'static> {
        self.layout.clone()
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

/// Creates a [`VertexBuffer`] from a slice of `vertices`, which implement [`AsGpuBuffer`] and
/// [`VertexLayout`].
pub trait AsVertexBuffer<const Offset: u32>: AsGpuBuffer + VertexLayout<Offset> {
    const LABEL: &'static str;

    fn as_entire_buffer<'s>(
        context: &RenderContext,
        vertices: &[Self],
        usage: wgpu::BufferUsages,
    ) -> VertexBuffer {
        if vertices.len() == 0 {
            panic!("`contents` must contain atleast one instance to call `as_entire_buffer`: [AsVertexBuffer: {:?}]", std::any::type_name::<Self>());
        }

        let buffer = Self::create_buffer_init(Some(Self::LABEL), context, &vertices, usage);
        let layout = Self::vertex_layout();

        VertexBuffer::new(buffer, usage, layout)
    }

    fn as_entire_buffer_empty<'s>(
        context: &RenderContext,
        size: u64,
        usage: wgpu::BufferUsages,
    ) -> VertexBuffer {
        let buffer = Self::create_buffer(Some(Self::LABEL), context, size, usage);
        let layout = Self::vertex_layout();

        VertexBuffer::new(buffer, usage, layout)
    }

    fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        Self::layout()
    }

    fn write_buffer_resize<T: AsGpuBuffer>(
        context: &RenderContext,
        vertex_buffer: &mut VertexBuffer,
        contents: &[T],
    ) {
        if contents.len() * std::mem::size_of::<T>() <= vertex_buffer.buffer.size() as usize {
            context
                .queue
                .write_buffer(&vertex_buffer.buffer, 0, bytemuck::cast_slice(contents));
        } else {
            vertex_buffer.buffer = <T as AsGpuBuffer>::create_buffer_init(
                Some(Self::LABEL),
                context,
                contents,
                vertex_buffer.usage,
            );
        }
    }
}

unsafe impl AsGpuBuffer for Vertex {}

impl<const Offset: u32> AsVertexBuffer<Offset> for Vertex {
    const LABEL: &'static str = "vertex uv";
}

unsafe impl AsGpuBuffer for VertexUv {}

impl<const Offset: u32> AsVertexBuffer<Offset> for VertexUv {
    const LABEL: &'static str = "vertex uv";
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceIndex(pub u32);

unsafe impl AsGpuBuffer for InstanceIndex {}

impl<const Offset: u32> VertexLayout<Offset> for InstanceIndex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<u32>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: Offset,
                format: wgpu::VertexFormat::Uint32,
            }],
        }
    }
}

impl<const Offset: u32> AsVertexBuffer<Offset> for InstanceIndex {
    const LABEL: &'static str = "instance index";
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexIndex(pub u32);

unsafe impl AsGpuBuffer for VertexIndex {}

impl<const Offset: u32> VertexLayout<Offset> for VertexIndex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<u32>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: Offset,
                format: wgpu::VertexFormat::Uint32,
            }],
        }
    }
}

impl<const Offset: u32> AsVertexBuffer<Offset> for VertexIndex {
    const LABEL: &'static str = "instance index";
}

impl<const Offset: u32> AsVertexBuffer<Offset> for Matrix4x4f {
    const LABEL: &'static str = "matrix";
}
