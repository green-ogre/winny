use crate::create_read_only_storage_bind_group;
use crate::render_pipeline::bind_group::{AsBindGroup, FragTexture};
use crate::render_pipeline::vertex::{VertexLayout, VertexUv, FULLSCREEN_QUAD_VERTEX_UV};
use crate::texture::{SamplerFilterType, TextureAtlas};
use crate::texture::{Texture, TextureDimensions};
use crate::transform::Transform;
use app::app::{AppSchedule, Schedule};
use app::plugins::Plugin;
use app::time::DeltaTime;
use app::window::{ViewPort, Window};
use asset::AssetId;
use asset::{Assets, Handle};
use cgmath::{Quaternion, Rad, Rotation3};
use ecs::prelude::*;
use ecs::SparseArrayIndex;
use ecs::SparseSet;
use ecs::{WinnyBundle, WinnyComponent, WinnyResource};
use fxhash::FxHashMap;
use render::{
    BindGroupHandle, BindGroups, RenderBindGroup, RenderConfig, RenderContext, RenderDevice,
    RenderEncoder, RenderView,
};
use std::ops::Range;
use wgpu::util::DeviceExt;
use winny_math::angle::{Degrees, Radf};
use winny_math::matrix::{scale_matrix4x4f, Matrix4x4f};
use winny_math::vector::{Vec2f, Vec3f};

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
            .insert_resource(TextureAtlasBindGroups::default())
            .add_systems(Schedule::StartUp, startup)
            .add_systems(
                Schedule::PostUpdate,
                (
                    bind_new_sprite_bundles,
                    bind_new_animated_sprite_bundles,
                    update_sprite_atlas_bind_groups,
                ),
            )
            .add_systems(
                AppSchedule::Render,
                (prepare_for_render_pass, render_sprites),
            );
    }
}

fn startup(mut commands: Commands, context: Res<RenderContext>) {
    let sprite_renderer = SpriteRenderer::new(&context);
    commands.insert_resource(sprite_renderer);
}

fn create_sprite_render_pipeline(
    context: &RenderContext,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let sprite_bind_group_layout = FragTexture::layout(&context);

    let render_pipeline_layout =
        context
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[layout, &sprite_bind_group_layout],
                push_constant_ranges: &[],
            });

    let shader = wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite_shader.wgsl").into()),
    };

    crate::create_render_pipeline(
        "sprites",
        &context.device,
        &render_pipeline_layout,
        context.config.format(),
        None,
        &[
            <VertexUv as VertexLayout<0>>::layout(),
            <SpriteInstance as VertexLayout<2>>::layout(),
            <Matrix4x4f as VertexLayout<4>>::layout(),
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
    atlas_uniforms: wgpu::Buffer,
    atlas_uniform_bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}

impl SpriteRenderer {
    pub fn new(context: &RenderContext) -> Self {
        let atlas_uniforms = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atlas uniforms"),
            size: 12,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (layout, atlas_uniform_bind_group) = create_read_only_storage_bind_group(
            Some("atlas uniforms"),
            &context.device,
            &atlas_uniforms,
            wgpu::ShaderStages::VERTEX,
            0,
        );

        let pipeline = create_sprite_render_pipeline(context, &layout);

        let vertex_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite vertexes"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sprite_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite instances"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let transform_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite transform"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            atlas_uniforms,
            atlas_uniform_bind_group,
            vertex_buffer,
            sprite_buffer,
            transform_buffer,
            pipeline,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct AtlasUniform {
    width: u32,
    height: u32,
    index: u32,
}

pub fn prepare_for_render_pass(
    mut sprite_renderer: ResMut<SpriteRenderer>,
    context: Res<RenderContext>,
    sprites: Query<(
        Sprite,
        Transform,
        BindGroupHandle,
        TextureDimensions,
        Option<AnimatedSprite>,
    )>,
    // TODO: change to camera
    window: Res<Window>,
) {
    let viewport = &window.viewport;

    // TODO: decide on whether to sort by bind group handle or z
    let mut sprites = sprites.iter().collect::<Vec<_>>();
    sprites.sort_by(|(s1, _, _, _, _), (s2, _, _, _, _)| s1.z.cmp(&s2.z));

    let atlas_data: Vec<_> = sprites
        .iter()
        .map(|(_, _, _, _, a_s)| {
            if let Some(a_s) = a_s {
                AtlasUniform {
                    width: a_s.width as u32,
                    height: a_s.height as u32,
                    index: a_s.index,
                }
            } else {
                AtlasUniform {
                    width: 1,
                    height: 1,
                    index: 0,
                }
            }
        })
        .collect();
    let atlas_data = bytemuck::cast_slice(&atlas_data);
    if atlas_data.len() <= sprite_renderer.atlas_uniforms.size() as usize {
        context
            .queue
            .write_buffer(&sprite_renderer.atlas_uniforms, 0, atlas_data);
    } else {
        util::tracing::trace!(
            "allocating larger sprite atlas uniform buffer. current size: {}, new size: {}",
            sprite_renderer.atlas_uniforms.size(),
            atlas_data.len(),
        );

        sprite_renderer.atlas_uniforms =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("sprite atlas uiform"),
                    contents: atlas_data,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let (_, atlas_uniform_bind_group) = create_read_only_storage_bind_group(
            Some("atlas uniforms"),
            &context.device,
            &sprite_renderer.atlas_uniforms,
            wgpu::ShaderStages::VERTEX,
            0,
        );
        sprite_renderer.atlas_uniform_bind_group = atlas_uniform_bind_group;
    }

    let vertex_data: Vec<_> = sprites
        .iter()
        .map(|(s, _, _, d, a)| s.to_vertices(&viewport, d, *a))
        .flatten()
        .collect();
    let vertex_data = bytemuck::cast_slice(&vertex_data);

    if vertex_data.len() <= sprite_renderer.vertex_buffer.size() as usize {
        context
            .queue
            .write_buffer(&sprite_renderer.vertex_buffer, 0, vertex_data);
    } else {
        util::tracing::trace!(
            "allocating larger sprite vertex buffer. current size: {}, new size: {}",
            sprite_renderer.vertex_buffer.size(),
            vertex_data.len(),
        );

        sprite_renderer.vertex_buffer =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("sprite vertex"),
                    contents: vertex_data,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
    }

    let sprite_data = sprites
        .iter()
        .map(|(s, _, _, _, _)| s.to_raw())
        .collect::<Vec<_>>();
    let sprite_data = bytemuck::cast_slice(&sprite_data);

    if sprite_data.len() <= sprite_renderer.sprite_buffer.size() as usize {
        context
            .queue
            .write_buffer(&sprite_renderer.sprite_buffer, 0, sprite_data);
    } else {
        util::tracing::trace!(
            "allocating larger sprite instance buffer. current size: {}, new size: {}",
            sprite_renderer.sprite_buffer.size(),
            sprite_data.len()
        );

        sprite_renderer.sprite_buffer =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("sprite instance"),
                    contents: sprite_data,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
    }

    let transform_data = sprites
        .iter()
        .map(|(s, t, _, _, _)| s.transformation_matrix(t))
        .collect::<Vec<_>>();
    let transform_data = bytemuck::cast_slice(&transform_data);

    if transform_data.len() <= sprite_renderer.transform_buffer.size() as usize {
        context
            .queue
            .write_buffer(&sprite_renderer.transform_buffer, 0, transform_data);
    } else {
        util::tracing::trace!(
            "allocating larger sprite transform buffer. current size: {}, new size: {}",
            sprite_renderer.transform_buffer.size(),
            transform_data.len()
        );

        sprite_renderer.transform_buffer =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("sprite transform"),
                    contents: transform_data,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
    }
}

#[derive(WinnyResource, Default)]
pub struct TextureAtlasBindGroups {
    bindings: SparseSet<BindGroupHandle, RenderBindGroup>,
    stored_bindings: FxHashMap<AssetId, BindGroupHandle>,
}

impl TextureAtlasBindGroups {
    pub fn get(&self, handle: BindGroupHandle) -> Option<&RenderBindGroup> {
        self.bindings.get(&handle)
    }

    pub fn get_handle_or_insert_with(
        &mut self,
        asset_id: AssetId,
        bind_group: impl FnOnce() -> RenderBindGroup,
    ) -> BindGroupHandle {
        if let Some(handle) = self.stored_bindings.get(&asset_id) {
            *handle
        } else {
            let index = self.bindings.insert_in_first_empty(bind_group());
            let handle = BindGroupHandle(index);
            self.stored_bindings.insert(asset_id, handle);

            handle
        }
    }
}

// TODO: Event driven updates
pub fn update_sprite_atlas_bind_groups(
    context: Res<RenderContext>,
    mut sprites: Query<
        (
            Handle<TextureAtlas>,
            Mut<BindGroupHandle>,
            Mut<TextureDimensions>,
        ),
        With<(AnimatedSprite, Sprite)>,
    >,
    texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut bind_groups: ResMut<TextureAtlasBindGroups>,
) {
    for (atlas, bind_handle, dimensions) in sprites.iter_mut() {
        if let Some(asset) = texture_atlases.get(&atlas) {
            let new_dimensions = TextureDimensions::from_texture_atlas(&asset.asset);
            let new_handle = bind_groups.get_handle_or_insert_with(atlas.id(), || {
                let (_, _, binding) = FragTexture::as_entire_binding(
                    &context,
                    FragTexture(&asset.asset.texture),
                    &SamplerFilterType::Nearest,
                );
                RenderBindGroup(binding)
            });
            *bind_handle = new_handle;
            *dimensions = new_dimensions;
        }
    }
}

fn render_sprites(
    mut encoder: ResMut<RenderEncoder>,
    sprite_renderer: Res<SpriteRenderer>,
    view: Res<RenderView>,
    sprites: Query<(
        Sprite,
        Transform,
        BindGroupHandle,
        TextureDimensions,
        Option<AnimatedSprite>,
    )>,
    bind_groups: Res<BindGroups>,
    atlas_bind_groups: Res<TextureAtlasBindGroups>,
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
    sprites.sort_by(|(s1, _, _, _, _), (s2, _, _, _, _)| s1.z.cmp(&s2.z));

    render_pass.set_pipeline(&sprite_renderer.pipeline);
    // sorted by bind group handle
    render_pass.set_vertex_buffer(0, sprite_renderer.vertex_buffer.slice(..));
    // sorted by bind group handle
    render_pass.set_vertex_buffer(1, sprite_renderer.sprite_buffer.slice(..));
    // sorted by bind group handle
    render_pass.set_vertex_buffer(2, sprite_renderer.transform_buffer.slice(..));
    // sorted by bind group handle
    render_pass.set_bind_group(0, &sprite_renderer.atlas_uniform_bind_group, &[]);

    let mut offset = 0;
    let previous_bind_index = usize::MAX;
    for (_, _, handle, _, anim) in sprites.iter() {
        if (**handle).index() != previous_bind_index {
            let binding = if anim.is_some() {
                atlas_bind_groups.get(**handle).unwrap()
            } else {
                bind_groups.get(**handle).unwrap()
            };

            render_pass.set_bind_group(1, binding, &[]);
        }

        render_pass.draw(
            offset * VERTICES..offset * VERTICES + VERTICES,
            offset..offset + 1,
        );
        offset += 1;
    }
}

#[derive(Debug, WinnyBundle)]
pub struct SpriteBundle {
    pub sprite: Sprite,
    pub handle: Handle<Texture>,
}

#[derive(Debug, WinnyBundle)]
pub struct AnimatedSpriteBundle {
    pub sprite: Sprite,
    pub animated_sprite: AnimatedSprite,
    pub handle: Handle<TextureAtlas>,
}

pub fn bind_new_animated_sprite_bundles(
    mut commands: Commands,
    context: Res<RenderContext>,
    sprites: Query<
        (Entity, Handle<TextureAtlas>),
        (
            With<AnimatedSprite>,
            Without<(BindGroupHandle, TextureDimensions)>,
        ),
    >,
    texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut bind_groups: ResMut<TextureAtlasBindGroups>,
) {
    for (entity, handle) in sprites.iter() {
        if let Some(asset) = texture_atlases.get(&handle) {
            let new_dimensions = TextureDimensions::from_texture_atlas(&asset.asset);
            let handle = bind_groups.get_handle_or_insert_with(handle.id(), || {
                let (_, _, binding) = FragTexture::as_entire_binding(
                    &context,
                    FragTexture(&asset.asset.texture),
                    &SamplerFilterType::Nearest,
                );
                RenderBindGroup(binding)
            });
            commands.get_entity(entity).insert((handle, new_dimensions));
        }
    }
}

pub fn bind_new_sprite_bundles(
    mut commands: Commands,
    context: Res<RenderContext>,
    sprites: Query<
        (Entity, Handle<Texture>),
        (With<Sprite>, Without<(BindGroupHandle, TextureDimensions)>),
    >,
    textures: ResMut<Assets<Texture>>,
    mut bind_groups: ResMut<BindGroups>,
) {
    for (entity, handle) in sprites.iter() {
        if let Some(asset) = textures.get(&handle) {
            let dimensions = TextureDimensions::from_texture(asset);
            let handle = bind_groups.get_handle_or_insert_with(&asset.path, || {
                let (_, _, binding) = FragTexture::as_entire_binding(
                    &context,
                    FragTexture(&asset.asset),
                    &SamplerFilterType::Nearest,
                );
                RenderBindGroup(binding)
            });
            commands.get_entity(entity).insert((handle, dimensions));
        }
    }
}

/// Describes local transformations in relation to the entity [`Transform`].
#[derive(WinnyComponent, Debug, Clone, Copy)]
pub struct Sprite {
    pub position: Vec3f,
    pub scale: Vec2f,
    pub rotation: Degrees,
    pub z: i32,
    pub v_flip: bool,
    pub h_flip: bool,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            scale: Vec2f::new(1., 1.),
            position: Vec3f::new(0.0, 0.0, 0.0),
            rotation: Degrees(0.0),
            z: 0,
            v_flip: false,
            h_flip: false,
        }
    }
}

impl Sprite {
    pub(crate) fn to_raw(&self) -> SpriteInstance {
        let flip_h = if self.h_flip { 0. } else { 1. };
        let flip_v = if self.v_flip { 0. } else { 1. };

        SpriteInstance { flip_v, flip_h }
    }

    /// Combines the entity transformation with the local transformation.
    pub(crate) fn transformation_matrix(&self, transform: &Transform) -> Matrix4x4f {
        let angle: Radf = self.rotation.into();
        let local_transformation = Transform {
            translation: self.position,
            rotation: Quaternion::from_angle_z(Rad(angle.0)),
            scale: self.scale,
        };

        transform.as_matrix() * local_transformation.as_matrix()
    }

    /// Creates a fullscreen quad, then scales about the origin for a one-to-one pixel size related
    /// to the [`ViewPort`].
    pub fn to_vertices(
        &self,
        viewport: &ViewPort,
        texture_dimension: &TextureDimensions,
        animation: Option<&AnimatedSprite>,
    ) -> [VertexUv; 6] {
        let mut vertices = FULLSCREEN_QUAD_VERTEX_UV;
        let mut atlas_scaling = (1.0, 1.0);
        if let Some(animation) = animation {
            atlas_scaling.0 /= animation.width as f32;
            atlas_scaling.1 /= animation.height as f32;
        }
        let normalized_scale = Vec2f::new(
            atlas_scaling.0 * texture_dimension.width() / viewport.width(),
            atlas_scaling.1 * texture_dimension.height() / viewport.height(),
        );
        let image_scale = scale_matrix4x4f(normalized_scale);
        for vert in vertices.iter_mut() {
            vert.position = image_scale * vert.position;
        }

        vertices
    }
}

/// Length of an [`AnimatedSprite`]'s frame.
#[derive(WinnyComponent)]
pub struct AnimationDuration(f32);

impl From<&AnimatedSprite> for AnimationDuration {
    fn from(value: &AnimatedSprite) -> Self {
        Self(value.frame_delta)
    }
}

/// Manages state of an Entity's animation.
#[derive(WinnyComponent, Debug, Clone)]
pub struct AnimatedSprite {
    /// Width of [`TextureAtlas`]
    width: u32,
    /// Height of [`TextureAtlas`]
    height: u32,
    index: u32,
    frame_range: Range<u32>,
    frame_delta: f32,
}

impl Default for AnimatedSprite {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            index: 0,
            frame_range: 0..1,
            frame_delta: 0.1,
        }
    }
}

impl AnimatedSprite {
    pub fn from_texture_atlas(atlas: &TextureAtlas) -> Self {
        Self {
            width: atlas.width,
            height: atlas.height,
            index: 0,
            frame_range: 0..(atlas.width * atlas.height),
            frame_delta: 0.1,
        }
    }

    pub fn total_frames(&self) -> u32 {
        self.width * self.height
    }

    pub fn with_frame_delta(mut self, frame_delta: f32) -> Self {
        self.frame_delta = frame_delta;
        self
    }

    pub fn frame_delta(&self) -> f32 {
        self.frame_delta
    }

    pub fn from_range(mut self, range: Range<u32>) -> Self {
        self.frame_range = range;
        self
    }

    pub fn range(&self) -> Range<u32> {
        self.frame_range.clone()
    }

    pub fn with_index(mut self, index: u32) -> Self {
        self.index = index;
        if self.index >= self.frame_range.end {
            self.index = self.frame_range.start;
        }
        self
    }

    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn advance(&mut self, delta_time: &DeltaTime, duration: &mut AnimationDuration) {
        duration.0 -= delta_time.delta;
        if duration.0 <= 0.0 {
            duration.0 = self.frame_delta;
            self.index += 1;
            if self.index >= self.frame_range.end {
                self.index = self.frame_range.start;
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        self.index == self.frame_range.end - 1
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    flip_v: f32,
    flip_h: f32,
}

impl<const Offset: u32> VertexLayout<Offset> for SpriteInstance {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SpriteInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: Offset,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<f32>() as wgpu::BufferAddress,
                    shader_location: Offset + 1,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}
