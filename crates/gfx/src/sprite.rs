use app::plugins::Plugin;
use app::window::Window;
use asset::{Assets, Handle};
use ecs::prelude::*;
use ecs::SparseArrayIndex;
use ecs::{WinnyBundle, WinnyComponent, WinnyResource};
use render::{
    BindGroupHandle, BindGroups, RenderBindGroup, RenderConfig, RenderDevice, RenderEncoder,
    RenderQueue, RenderView,
};
use winny_math::angle::Degrees;
use winny_math::matrix::{
    rotation_2d_matrix4x4f, scale_matrix4x4f, translation_matrix4x4f,
    world_to_screen_space_matrix4x4f, Matrix4x4f,
};
use winny_math::vector::{Vec2f, Vec3f, Vec4f};

use wgpu::util::DeviceExt;

use crate::texture::{Texture, TextureDimensions};
use crate::transform::Transform;
use crate::vertex::{VertexLayout, VertexUv, FULLSCREEN_QUAD_VERTEX_UV};

#[derive(Default)]
pub struct SpritePlugin {
    pixel_perfect: bool,
}

#[derive(WinnyResource)]
#[allow(dead_code)]
pub struct GlobalSpriteSettings {
    pixel_perfect: bool,
}

impl From<&SpritePlugin> for GlobalSpriteSettings {
    fn from(value: &SpritePlugin) -> Self {
        if value.pixel_perfect {
            unimplemented!()
        }

        Self {
            pixel_perfect: value.pixel_perfect,
        }
    }
}

impl Plugin for SpritePlugin {
    fn build(&mut self, app: &mut app::app::App) {
        app.register_resource::<SpriteRenderer>()
            .insert_resource(GlobalSpriteSettings::from(&*self))
            .add_systems(ecs::Schedule::StartUp, startup)
            .add_systems(ecs::Schedule::PostUpdate, bind_new_sprite_bundles)
            .add_systems(ecs::Schedule::PreRender, prepare_for_render_pass)
            .add_systems(ecs::Schedule::Render, render_sprites);
    }
}

fn startup(mut commands: Commands, device: Res<RenderDevice>, config: Res<RenderConfig>) {
    let sprite_renderer = SpriteRenderer::new(&device, &config);
    commands.insert_resource(sprite_renderer);
}

fn create_sprite_render_pipeline(
    device: &RenderDevice,
    config: &RenderConfig,
) -> wgpu::RenderPipeline {
    let sprite_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("bind group layout for sprite"),
        });

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[&sprite_bind_group_layout],
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
        &[
            VertexUv::layout(),
            SpriteInstance::layout(),
            // Transform
            Matrix4x4f::layout(),
        ],
        shader,
        true,
    )
}

const VERTICES: u32 = 6;

#[derive(WinnyResource)]
pub struct SpriteRenderer {
    vertex_buffer: wgpu::Buffer,
    sprite_buffer: wgpu::Buffer,
    transform_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
}

impl SpriteRenderer {
    pub fn new(device: &RenderDevice, config: &RenderConfig) -> Self {
        let pipeline = create_sprite_render_pipeline(device, config);

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite vertexes"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sprite_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite instances"),
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
            sprite_buffer,
            transform_buffer,
            pipeline,
        }
    }
}

pub fn prepare_for_render_pass(
    mut sprite_renderer: ResMut<SpriteRenderer>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    config: Res<RenderConfig>,
    sprites: Query<(Sprite, Transform, BindGroupHandle, TextureDimensions)>,
    // TODO: change to camera
    window: Res<Window>,
) {
    // TODO: decide on whether to sort by bind group handle or z
    let mut sprites = sprites.iter().collect::<Vec<_>>();
    sprites.sort_by(|(s1, _, _, _), (s2, _, _, _)| s1.z.cmp(&s2.z));

    let vertex_data: Vec<_> = sprites
        .iter()
        .map(|(s, t, _, d)| s.to_vertices(&config, d, t))
        .flatten()
        .collect();
    let vertex_data = bytemuck::cast_slice(&vertex_data);

    if vertex_data.len() <= sprite_renderer.vertex_buffer.size() as usize {
        queue.write_buffer(&sprite_renderer.vertex_buffer, 0, vertex_data);
    } else {
        util::tracing::info!(
            "allocating larger sprite vertex buffer. current size: {}, new size: {}",
            sprite_renderer.vertex_buffer.size(),
            vertex_data.len(),
        );

        sprite_renderer.vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sprite vertex"),
                contents: vertex_data,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
    }

    let sprite_data = sprites
        .iter()
        .map(|(s, _, _, _)| s.to_raw())
        .collect::<Vec<_>>();
    let sprite_data = bytemuck::cast_slice(&sprite_data);

    if sprite_data.len() <= sprite_renderer.sprite_buffer.size() as usize {
        queue.write_buffer(&sprite_renderer.sprite_buffer, 0, sprite_data);
    } else {
        util::tracing::info!(
            "allocating larger sprite instance buffer. current size: {}, new size: {}",
            sprite_renderer.sprite_buffer.size(),
            sprite_data.len()
        );

        sprite_renderer.sprite_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sprite instance"),
                contents: sprite_data,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
    }

    let viewport = &window.viewport;
    let transform_data = sprites
        .iter()
        .map(|(_, t, _, _)| t.transformation_matrix(viewport, config.max_z))
        .collect::<Vec<_>>();
    let transform_data = bytemuck::cast_slice(&transform_data);

    if transform_data.len() <= sprite_renderer.transform_buffer.size() as usize {
        queue.write_buffer(&sprite_renderer.transform_buffer, 0, transform_data);
    } else {
        util::tracing::info!(
            "allocating larger sprite transform buffer. current size: {}, new size: {}",
            sprite_renderer.transform_buffer.size(),
            transform_data.len()
        );

        sprite_renderer.transform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sprite transform"),
                contents: transform_data,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
    }
}

fn render_sprites(
    mut encoder: ResMut<RenderEncoder>,
    sprite_renderer: Res<SpriteRenderer>,
    view: Res<RenderView>,
    sprites: Query<(BindGroupHandle, Sprite)>,
    bind_groups: Res<BindGroups>,
) {
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("sprites"),
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
    sprites.sort_by(|(_, s1), (_, s2)| s1.z.cmp(&s2.z));

    render_pass.set_pipeline(&sprite_renderer.pipeline);
    // sorted by bind group handle
    render_pass.set_vertex_buffer(0, sprite_renderer.vertex_buffer.slice(..));
    // sorted by bind group handle
    render_pass.set_vertex_buffer(1, sprite_renderer.sprite_buffer.slice(..));
    // sorted by bind group handle
    render_pass.set_vertex_buffer(2, sprite_renderer.transform_buffer.slice(..));

    let mut offset = 0;
    let previous_bind_index = usize::MAX;
    for (handle, _) in sprites.iter() {
        if (**handle).index() != previous_bind_index {
            let binding = bind_groups.get(**handle).unwrap();
            render_pass.set_bind_group(0, binding, &[]);
        }

        render_pass.draw(
            offset * VERTICES..offset * VERTICES + VERTICES,
            offset..offset + 1,
        );
        offset += 1;
    }
}

#[derive(Debug, Clone, WinnyBundle)]
pub struct SpriteBundle {
    pub sprite: Sprite,
    pub handle: Handle<Texture>,
}

pub fn bind_new_sprite_bundles(
    mut commands: Commands,
    device: Res<RenderDevice>,
    sprites: Query<(Entity, Sprite, Handle<Texture>), Without<BindGroupHandle>>,
    textures: ResMut<Assets<Texture>>,
    mut bind_groups: ResMut<BindGroups>,
) {
    for (entity, sprite, handle) in sprites.iter() {
        if let Some(asset) = textures.get(&handle) {
            util::tracing::info!("binding new sprite bundle: {entity:?}, {handle:?}, {sprite:?}");
            let texture_dimensions = TextureDimensions::from_texture(&asset.asset);
            let handle = bind_groups.get_handle_or_insert_with(&asset.path, || {
                binding_from_texture(&asset.asset, &device)
            });
            commands
                .get_entity(entity)
                .insert((handle, texture_dimensions));
        }
    }
}

#[derive(WinnyComponent, Debug, Clone, Copy)]
pub struct Sprite {
    // inherits from Transform
    pub position: Vec3f,
    // applied in addition to transform scaling
    pub scale: Vec2f,
    pub rotation: Degrees,
    // linearly mixed with the sprite sample by mask.v[3] (`a`)
    pub mask: Vec4f,
    pub z: u32,
    pub v_flip: bool,
    pub h_flip: bool,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            scale: Vec2f::new(1., 1.),
            position: Vec3f::new(0.0, 0.0, 0.0),
            rotation: Degrees(0.0),
            mask: Vec4f::zero(),
            z: 0,
            v_flip: false,
            h_flip: false,
        }
    }
}

impl Sprite {
    pub fn to_raw(&self) -> SpriteInstance {
        let flip_h = if self.h_flip { 0. } else { 1. };
        let flip_v = if self.v_flip { 0. } else { 1. };
        SpriteInstance {
            mask: self.mask,
            flip_v,
            flip_h,
            _padding: [0.; 2],
        }
    }

    pub fn to_vertices(
        &self,
        config: &RenderConfig,
        texture_dimension: &TextureDimensions,
        transform: &Transform,
    ) -> [VertexUv; 6] {
        let mut vertices = FULLSCREEN_QUAD_VERTEX_UV;

        let normalized_scale = Vec2f::new(
            texture_dimension.0 .0 as f32 / config.width() as f32,
            texture_dimension.0 .1 as f32 / config.height() as f32,
        );
        let image_scale = scale_matrix4x4f(normalized_scale);
        for vert in vertices.iter_mut() {
            vert.position = image_scale * vert.position;
        }

        let scale = scale_matrix4x4f(self.scale);
        let rotation = rotation_2d_matrix4x4f(self.rotation);
        let world_to_screen_space =
            world_to_screen_space_matrix4x4f(config.width(), config.height(), config.max_z);
        let translation = translation_matrix4x4f(
            world_to_screen_space
                * Vec4f::to_homogenous(self.position + Vec3f::new(0., 0., transform.translation.z)),
        );

        for vert in vertices.iter_mut() {
            vert.position = scale * vert.position;
            vert.position = rotation * vert.position;
            vert.position = translation * vert.position;
        }

        vertices
    }
}

pub fn binding_from_texture(texture: &Texture, device: &RenderDevice) -> RenderBindGroup {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
        label: Some("bind group layout for sprite"),
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&texture.sampler),
            },
        ],
        label: Some("bind group for sprite"),
    });

    RenderBindGroup(bind_group)
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    mask: Vec4f,
    flip_v: f32,
    flip_h: f32,
    _padding: [f32; 2],
}

impl VertexLayout for SpriteInstance {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SpriteInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}
