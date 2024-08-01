use super::{
    buffer::AsGpuBuffer,
    vertex::{Vertex, VertexLayout, VertexUv},
};
use render::RenderContext;

pub trait VertexBuffer<const Offset: u32, T, V: AsGpuBuffer + VertexLayout<Offset>> {
    const LABEL: &'static str;
    type State<'s>;

    fn as_verts<'s>(contents: &[T], state: &Self::State<'s>) -> Vec<V>;

    fn as_entire_buffer<'s>(
        context: &RenderContext,
        contents: &[T],
        state: &Self::State<'s>,
        usage: wgpu::BufferUsages,
    ) -> (Vec<V>, wgpu::Buffer, wgpu::VertexBufferLayout<'static>) {
        if contents.len() == 0 {
            panic!("`contents` must contain atleast one instance to call `as_entire_buffer`: [AsBindGroup<{:?}, {:?}: NoUninit]", std::any::type_name::<T>(), std::any::type_name::<V>());
        }

        let raw_contents = Self::as_verts(contents, state);
        let buffer = V::create_buffer_init(Some(Self::LABEL), context, &raw_contents, &usage);
        let layout = Self::vertex_layout();

        (raw_contents, buffer, layout)
    }

    fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        V::layout()
    }
}

unsafe impl AsGpuBuffer for Vertex {}

impl<const Offset: u32> VertexBuffer<Offset, Vertex, Vertex> for Vertex {
    const LABEL: &'static str = "vertex uv";
    type State<'s> = ();

    fn as_verts<'s>(contents: &[Vertex], _state: &Self::State<'s>) -> Vec<Vertex> {
        contents.to_vec()
    }
}

unsafe impl AsGpuBuffer for VertexUv {}

impl<const Offset: u32> VertexBuffer<Offset, VertexUv, VertexUv> for VertexUv {
    const LABEL: &'static str = "vertex uv";
    type State<'s> = ();

    fn as_verts<'s>(contents: &[VertexUv], _state: &Self::State<'s>) -> Vec<VertexUv> {
        contents.to_vec()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceIndex(pub u32);

unsafe impl AsGpuBuffer for InstanceIndex {}

impl<const Offset: u32> VertexBuffer<Offset, InstanceIndex, u32> for InstanceIndex {
    const LABEL: &'static str = "instance index";
    type State<'s> = ();

    fn as_verts<'s>(contents: &[InstanceIndex], _state: &Self::State<'s>) -> Vec<u32> {
        contents.iter().map(|i| i.0).collect()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexIndex(pub u32);

unsafe impl AsGpuBuffer for VertexIndex {}

impl<const Offset: u32> VertexBuffer<Offset, VertexIndex, u32> for VertexIndex {
    const LABEL: &'static str = "instance index";
    type State<'s> = ();

    fn as_verts<'s>(contents: &[VertexIndex], _state: &Self::State<'s>) -> Vec<u32> {
        contents.iter().map(|i| i.0).collect()
    }
}
