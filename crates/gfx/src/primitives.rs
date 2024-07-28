use app::{plugins::Plugin, window::Window, winit::window};
use ecs::{Commands, Query, Res, ResMut, WinnyBundle, WinnyComponent, WinnyResource};
use render::{RenderConfig, RenderDevice, RenderEncoder, RenderQueue, RenderView};
use winny_math::{matrix::Matrix4x4f, vector::Vec3f};

use crate::{
    transform::{self, new_transform_bind_group, new_transform_bind_group_layout, Transform},
    vertex::{Vertex, VertexLayout, FULLSCREEN_QUAD_VERTEX},
};

pub struct PrimitivesPlugin;

impl Plugin for PrimitivesPlugin {
    fn build(&mut self, app: &mut app::app::App) {
        app.register_resource::<PrimitiveRenderer>()
            .add_systems(ecs::Schedule::StartUp, startup)
            .add_systems(ecs::Schedule::PreRender, prepare_buffers_for_render_pass)
            .add_systems(ecs::Schedule::Render, render_primitives);
    }
}

fn startup(mut commands: Commands, device: Res<RenderDevice>, config: Res<RenderConfig>) {
    commands.insert_resource(PrimitiveRenderer::new(&device, &config));
}

#[derive(Debug, Default, Clone, Copy, PartialEq, WinnyComponent)]
pub struct RectPrimitive {
    pub tl: Vec3f,
    pub size: Vec3f,
}

impl RectPrimitive {
    pub fn new(tl: Vec3f, size: Vec3f) -> Self {
        Self { tl, size }
    }

    pub(crate) fn as_vertices(&self) -> [Vertex; 12] {
        [
            Vertex::new(-1.0, 1.0, 0.0),
            Vertex::new(-1.0, -1.0, 0.0),
            Vertex::new(-0.98, -1.0, 0.0),
            Vertex::new(-0.98, -1.0, 0.0),
            Vertex::new(-0.98, 1.0, 0.0),
            Vertex::new(-1.0, 1.0, 0.0),
            Vertex::new(0.98, -1.0, 0.0),
            Vertex::new(1.0, -1.0, 0.0),
            Vertex::new(1.0, 1.0, 0.0),
            Vertex::new(1.0, 1.0, 0.0),
            Vertex::new(0.98, 1.0, 0.0),
            Vertex::new(0.98, -1.0, 0.0),
        ]
    }
}

#[derive(WinnyBundle)]
pub struct RectPrimitiveBundle {
    pub rect: RectPrimitive,
    pub transform: Transform,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, WinnyComponent)]
pub struct CirclePrimitive {
    pub position: Vec3f,
    pub radius: f32,
}

impl CirclePrimitive {
    pub fn new(position: Vec3f, radius: f32) -> Self {
        Self { position, radius }
    }

    pub(crate) fn as_vertices(&self, transform: &Transform) -> [Vertex; 6] {
        todo!()
    }
}

#[derive(WinnyResource)]
pub struct PrimitiveRenderer {
    transform_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    transform_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    num_verts: u32,
}

impl PrimitiveRenderer {
    pub fn new(device: &RenderDevice, config: &RenderConfig) -> Self {
        let pipeline = create_primitive_render_pipeline(device, config);

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite vertexes"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let transform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite vertexes"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let layout = new_transform_bind_group_layout(&device, 0, wgpu::ShaderStages::VERTEX);
        let transform_bind_group = new_transform_bind_group(&device, &layout, &transform_buffer, 0);

        Self {
            transform_bind_group,
            transform_buffer,
            vertex_buffer,
            pipeline,
            num_verts: 0,
        }
    }
}

fn create_primitive_render_pipeline(
    device: &RenderDevice,
    config: &RenderConfig,
) -> wgpu::RenderPipeline {
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let shader = wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/primitives.wgsl").into()),
    };

    crate::create_render_pipeline(
        "primitives",
        &device,
        &render_pipeline_layout,
        config.format(),
        None,
        &[Vertex::layout()],
        shader,
        true,
    )
}

fn prepare_buffers_for_render_pass(
    mut primitive_renderer: ResMut<PrimitiveRenderer>,
    device: Res<RenderDevice>,
    rects: Query<(RectPrimitive, Transform)>,
    window: Res<Window>,
) {
    let verts: Vec<Vertex> = rects
        .iter()
        .map(|(r, _)| r.as_vertices())
        .flatten()
        .collect();
    use wgpu::util::DeviceExt;
    primitive_renderer.num_verts = verts.len() as u32;
    primitive_renderer.vertex_buffer =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("primitives"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });

    let transforms: Vec<Matrix4x4f> = rects
        .iter()
        .map(|(_, t)| t.transformation_matrix(&window.viewport, 1000.))
        .collect();
    primitive_renderer.transform_buffer =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("primitives"),
            contents: bytemuck::cast_slice(&transforms),
            usage: wgpu::BufferUsages::VERTEX,
        });

    let layout = new_transform_bind_group_layout(&device, 0, wgpu::ShaderStages::VERTEX);
    let bg = new_transform_bind_group(&device, &layout, &primitive_renderer.transform_buffer, 0);
    primitive_renderer.transform_bind_group = bg;
}

const VERTICES: u32 = 12;

fn render_primitives(
    mut encoder: ResMut<RenderEncoder>,
    primitive_renderer: Res<PrimitiveRenderer>,
    view: Res<RenderView>,
    query: Query<(RectPrimitive, Transform)>,
) {
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("primitives"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
    });

    render_pass.set_pipeline(&primitive_renderer.pipeline);
    render_pass.set_vertex_buffer(0, primitive_renderer.vertex_buffer.slice(..));
    render_pass.set_vertex_buffer(1, primitive_renderer.transform_buffer.slice(..));
    // render_pass.draw(0..primitive_renderer.num_verts, 0..1);

    let mut offset = 0;
    for (_, _) in query.iter() {
        render_pass.draw(
            offset * VERTICES..offset * VERTICES + VERTICES,
            offset..offset + 1,
        );
        offset += 1;
    }
}
