use app::plugins::Plugin;
use ecs::{Commands, Component, Query, Res, ResMut, WinnyResource};
use render::{BindGroupHandle, BindGroups, RenderConfig, RenderDevice, RenderEncoder, RenderView};
use winny_math::matrix::Matrix4x4f;

use crate::{
    create_read_only_storage_bind_group,
    sprite::{AnimatedSprite, Sprite, TextureAtlasBindGroups},
    transform::Transform,
    vertex::{VertexLayout, VertexUv},
};

pub struct PrimitivesPlugin;

impl Plugin for PrimitivesPlugin {
    fn build(&mut self, app: &mut app::app::App) {
        app.register_resource::<PrimitiveRenderer>()
            .add_systems(ecs::Schedule::StartUp, startup);
    }
}

fn startup(mut commands: Commands, device: Res<RenderDevice>, config: Res<RenderConfig>) {
    commands.insert_resource(PrimitiveRenderer::new(&device, &config));
}

#[derive(WinnyComponent)]
pub enum Primitive {
    Rect(RectCollider),
    Circle(CircleCollider),
}

impl Collider {
    pub fn absolute(&self, position: &Vec3f) -> AbsoluteCollider {
        match self {
            Self::Rect(rect) => {
                let mut abs = *rect;
                abs.tl += *position;
                AbsoluteCollider::Rect(abs)
            }
            Self::Circle(circle) => {
                let mut abs = *circle;
                abs.position += *position;
                AbsoluteCollider::Circle(abs)
            }
        }
    }
}

pub enum AbsoluteCollider {
    Rect(RectCollider),
    Circle(CircleCollider),
}

impl CollidesWith<Self> for AbsoluteCollider {
    fn collides_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Rect(s), Self::Rect(o)) => s.collides_with(o),
            (Self::Rect(s), Self::Circle(o)) => s.collides_with(o),
            (Self::Circle(s), Self::Rect(o)) => s.collides_with(o),
            (Self::Circle(s), Self::Circle(o)) => s.collides_with(o),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Component)]
pub struct RectCollider {
    pub tl: Vec3f,
    pub size: Vec3f,
}

impl RectCollider {
    pub fn br(&self) -> Vec3f {
        self.tl + self.size
    }
}

impl CollidesWith<Self> for RectCollider {
    fn collides_with(&self, other: &Self) -> bool {
        let not_collided = other.tl.y > self.br().y
            || other.tl.x > self.br().x
            || other.br().y < self.tl.y
            || other.br().x < self.tl.x;

        !not_collided
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Component)]
pub struct CircleCollider {
    pub position: Vec3f,
    pub radius: f32,
}

impl CollidesWith<Self> for CircleCollider {
    fn collides_with(&self, other: &Self) -> bool {
        let distance = self.position.dist2(&other.position);
        let combined_radii = self.radius.powi(2) + other.radius.powi(2);

        distance <= combined_radii
    }
}

impl CollidesWith<RectCollider> for CircleCollider {
    fn collides_with(&self, other: &RectCollider) -> bool {
        let dist_x = (self.position.x - (other.tl.x - other.size.x * 0.5)).abs();
        let dist_y = (self.position.y - (other.tl.y - other.size.y * 0.5)).abs();

        if dist_x > other.size.x * 0.5 + self.radius {
            return false;
        }

        if dist_y > other.size.y * 0.5 + self.radius {
            return false;
        }

        if dist_x <= other.size.x * 0.5 {
            return true;
        }

        if dist_y <= other.size.y * 0.5 {
            return true;
        }

        let corner_dist =
            (dist_x - other.size.x * 0.5).powi(2) + (dist_y - other.size.y * 0.5).powi(2);

        corner_dist <= self.radius.powi(2)
    }
}

impl CollidesWith<CircleCollider> for RectCollider {
    fn collides_with(&self, other: &CircleCollider) -> bool {
        other.collides_with(self)
    }
}

#[derive(WinnyResource)]
pub struct PrimitiveRenderer {
    vertex_buffer: wgpu::Buffer,
    transform_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
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
            label: Some("sprite transform"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            transform_buffer,
            pipeline,
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
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite_shader.wgsl").into()),
    };

    crate::create_render_pipeline(
        "sprites",
        &device,
        &render_pipeline_layout,
        config.format(),
        None,
        &[VertexUv::layout(), Matrix4x4f::layout()],
        shader,
        true,
    )
}

fn render_primitives(
    mut encoder: ResMut<RenderEncoder>,
    primitive_renderer: Res<PrimitiveRenderer>,
    view: Res<RenderView>,
    primitives: Query<(Primitive, Transform)>,
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

    // TODO: decide on whether to sort by bind group handle or z
    let mut sprites = sprites.iter().collect::<Vec<_>>();
    sprites.sort_by(|(_, s1, _), (_, s2, _)| s1.z.cmp(&s2.z));

    render_pass.set_pipeline(&sprite_renderer.pipeline);
    // sorted by bind group handle
    render_pass.set_vertex_buffer(0, sprite_renderer.vertex_buffer.slice(..));
    // sorted by bind group handle
    render_pass.set_vertex_buffer(1, sprite_renderer.sprite_buffer.slice(..));
    // sorted by bind group handle
    render_pass.set_vertex_buffer(2, sprite_renderer.transform_buffer.slice(..));
    // sorted by bind group handle
    render_pass.set_bind_group(1, &sprite_renderer.atlas_uniform_bind_group, &[]);

    let mut offset = 0;
    let previous_bind_index = usize::MAX;
    for (handle, _, anim) in sprites.iter() {
        if (**handle).index() != previous_bind_index {
            let binding = if anim.is_some() {
                atlas_bind_groups.get(**handle).unwrap()
            } else {
                bind_groups.get(**handle).unwrap()
            };

            render_pass.set_bind_group(0, binding, &[]);
        }

        render_pass.draw(
            offset * VERTICES..offset * VERTICES + VERTICES,
            offset..offset + 1,
        );
        offset += 1;
    }
}
