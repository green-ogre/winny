use crate::camera::{Camera, CameraUniform};
use crate::render::{RenderEncoder, RenderView};
use crate::render_pipeline::bind_group::{
    self, AsBindGroup, AssetBindGroups, BindGroup, BindGroupHandle, RenderBindGroup,
};
use crate::render_pipeline::buffer::AsGpuBuffer;
use crate::render_pipeline::material::{Material, Material2d};
use crate::render_pipeline::pipeline::{FragmentType, RenderPipeline2d};
use crate::render_pipeline::render_assets::{RenderAsset, RenderAssets};
use crate::render_pipeline::shader::{FragmentShaderSource, VertexShader, VertexShaderSource};
use crate::render_pipeline::vertex::{VertexLayout, VertexUv, FULLSCREEN_QUAD_VERTEX_UV};
use crate::render_pipeline::vertex_buffer::{AsVertexBuffer, VertexBuffer};
use crate::texture::{Image, TextureAtlas};
use crate::texture::{Texture, TextureDimensions};
use crate::transform::Transform;
use app::prelude::*;
use asset::server::AssetServer;
use asset::*;
use cgmath::{Quaternion, Rad, Rotation3};
use ecs::system_param::SystemParam;
use ecs::*;
use ecs::{WinnyBundle, WinnyComponent, WinnyResource};
use math::angle::{Degrees, Radf};
use math::matrix::{scale_matrix4x4f, Matrix4x4f};
use math::vector::{Vec2f, Vec3f};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Range;

#[derive(Debug)]
pub struct SpritePlugin;

impl Plugin for SpritePlugin {
    fn build(&mut self, app: &mut App) {
        app.register_resource::<SpriteVertShader>()
            .register_resource::<SpriteBuffers>()
            .add_systems(Schedule::StartUp, startup)
            .add_systems(AppSchedule::Render, render_sprites);
    }
}

fn startup(mut commands: Commands, context: Res<RenderContext>) {
    commands.insert_resource(SpriteBuffers::new(&context));
}

#[derive(Default)]
pub struct SpriteMaterialPlugin<M: Material>(PhantomData<M>);

impl<M: Material> Debug for SpriteMaterialPlugin<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SpriteMaterialPlugin")
    }
}

impl<M: Material> Plugin for SpriteMaterialPlugin<M> {
    fn build(&mut self, app: &mut App) {
        app.add_systems(
            AppSchedule::PreRender,
            (
                bind_new_sprite_bundles::<M>,
                bind_updated_texture_handles::<M>,
                prepare_for_render_pass::<M>,
            ),
        );
    }
}

impl<M: Material> SpriteMaterialPlugin<M> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

#[derive(Debug, WinnyBundle)]
pub struct SpriteBundle<M: Material = Material2d> {
    pub sprite: Sprite,
    pub material: M,
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
    pub(crate) fn to_raw(&self, anim_sprite: Option<&AnimatedSprite>) -> SpriteInstance {
        let flip_h = if self.h_flip { 0. } else { 1. };
        let flip_v = if self.v_flip { 0. } else { 1. };
        if let Some(anim_sprite) = anim_sprite {
            SpriteInstance {
                flip_v,
                flip_h,
                width: anim_sprite.width as u32,
                height: anim_sprite.height as u32,
                index: anim_sprite.index,
                _padding: 0.,
            }
        } else {
            SpriteInstance {
                flip_v,
                flip_h,
                width: 1,
                height: 1,
                index: 0,
                _padding: 0.,
            }
        }
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
    /// to the window's [`ViewPort`].
    pub(crate) fn to_vertices(
        &self,
        window: &Window,
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
            atlas_scaling.0 * texture_dimension.width() / window.viewport.width(),
            atlas_scaling.1 * texture_dimension.height() / window.viewport.height(),
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

#[derive(Debug, WinnyBundle)]
pub struct AnimatedSpriteBundle<M: Material = Material2d> {
    pub sprite: Sprite,
    pub animated_sprite: AnimatedSprite,
    pub material: M,
    pub handle: Handle<TextureAtlas>,
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
            width: atlas.dimensions.width(),
            height: atlas.dimensions.height(),
            index: 0,
            frame_range: 0..(atlas.dimensions.width() * atlas.dimensions.height()),
            frame_delta: 0.1,
        }
    }

    pub fn set_dimensions(&mut self, dimensions: &Dimensions<u32>) {
        self.width = dimensions.width();
        self.height = dimensions.height();
        if self.index >= self.width * self.height {
            self.index = 0;
        }
    }

    pub fn with_dimensions(mut self, dimensions: Dimensions<u32>) -> Self {
        self.width = dimensions.width();
        self.height = dimensions.height();
        self
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

    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn advance(&mut self, delta_time: &DeltaTime, duration: &mut AnimationDuration) {
        duration.0 -= delta_time.delta;
        if duration.0 <= 0.0 {
            duration.0 = self.frame_delta;
            self.index += 1;
            if self.index >= self.frame_range.end || self.index >= self.width * self.height {
                self.index = self.frame_range.start;
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        self.index == self.frame_range.end - 1
    }
}

#[derive(WinnyComponent, PartialEq, Eq, Hash, Clone, Copy)]
pub struct SpritePipelineEntity(Entity);

#[derive(WinnyResource)]
pub struct SpriteVertShader(Handle<VertexShaderSource>);

fn bind_new_sprite_bundles<M: Material>(
    mut commands: Commands,
    mut server: ResMut<AssetServer>,
    mut bind_groups: ResMut<AssetBindGroups>,
    buffers: Res<SpriteBuffers>,
    context: Res<RenderContext>,
    texture_params: <Texture as RenderAsset>::Params<'_>,
    pipelines: Query<(Entity, SpritePipeline, MaterialMarker<M>)>,
    bundles: Query<
        (Entity, Handle<Image>, M),
        (
            Without<(SpritePipelineEntity, TextureDimensions)>,
            With<(Sprite, Transform)>,
        ),
    >,
    mut textures: ResMut<RenderAssets<Texture>>,
    images: Res<Assets<Image>>,
    sprite_vert_shader: Option<Res<SpriteVertShader>>,
    mut vert_shaders: ResMut<Assets<VertexShaderSource>>,
    mut frag_shaders: ResMut<Assets<FragmentShaderSource>>,
) {
    let mut pipeline_entity = None;
    for (entity, image_handle, material) in bundles.iter() {
        if let Some(vert_shader_handle) = &sprite_vert_shader {
            if let Some(vert_shader) = vert_shaders.get_mut(&vert_shader_handle.0) {
                if let Some(image) = images.get(image_handle) {
                    let texture = textures
                        .entry(image_handle.clone())
                        .or_insert_with(|| Texture::prepare_asset(image, &texture_params));
                    let dimensions = TextureDimensions::from_texture(&texture);

                    let binding = if !bind_groups.contains(image_handle.id()) {
                        let binding = RenderBindGroup(<M as AsBindGroup>::as_entire_binding(
                            &context,
                            material.clone(),
                            material
                                .resource_state(&mut textures, &images, &context)
                                .expect("material is initialized"),
                        ));
                        bind_groups.insert(image_handle.clone(), binding)
                    } else {
                        bind_groups.get_handle(image_handle).unwrap()
                    };
                    commands.get_entity(entity).insert((binding, dimensions));
                } else {
                    return;
                }

                if let Ok((pipeline, _, _)) = pipelines.get_single() {
                    commands
                        .get_entity(entity)
                        .insert(SpritePipelineEntity(pipeline));
                } else {
                    if let Some(pipeline_entity) = pipeline_entity {
                        commands.get_entity(entity).insert(pipeline_entity);
                    } else {
                        let sprite_render_pipeline = SpritePipeline::new(
                            material.clone(),
                            &context,
                            &mut server,
                            &mut frag_shaders,
                            vert_shader.shader(&context),
                            &buffers,
                        );

                        let pipeline = SpritePipelineEntity(
                            commands
                                .spawn((sprite_render_pipeline, MaterialMarker::<M>(PhantomData)))
                                .entity(),
                        );

                        commands.get_entity(entity).insert(pipeline);
                        pipeline_entity = Some(pipeline);
                    }
                }
            }
        } else {
            // This will only run when the first sprite pipeline is built, so we compile the
            // shader
            let vert_shader = wgpu::ShaderModuleDescriptor {
                label: Some("particles vert"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../res/shaders/sprite_vert.wgsl").into(),
                ),
            };
            let vert_shader = VertexShader(context.device.create_shader_module(vert_shader));
            let handle = vert_shaders.add(VertexShaderSource(
                include_str!("../../../res/shaders/sprite_vert.wgsl").into(),
                Some(vert_shader),
            ));
            commands.insert_resource(SpriteVertShader(handle));
        }
    }
}

fn bind_updated_texture_handles<M: Material>(
    mut bind_groups: ResMut<AssetBindGroups>,
    mut sprites: Query<(
        Mut<Handle<Image>>,
        Mut<BindGroupHandle>,
        Mut<TextureDimensions>,
        M,
    )>,
    context: Res<RenderContext>,
    images: Res<Assets<Image>>,
    mut textures: ResMut<RenderAssets<Texture>>,
    exture_params: <Texture as RenderAsset>::Params<'_>,
    // params: <M as Material>::BindingState<'_>,
) {
    // for (texture_handle, bind_group_handle, texture_dimensions, material) in sprites.iter_mut() {
    //     if texture_handle.is_changed() {
    //         if let Some(image) = images.get(texture_handle) {
    //             let texture = textures
    //                 .entry(texture_handle.clone())
    //                 .or_insert_with(|| Texture::prepare_asset(image, &texture_params));
    //             let dimensions = TextureDimensions::from_texture(texture);
    //             if !bind_groups.contains(texture_handle.id()) {
    //                 let binding = RenderBindGroup(<M as AsBindGroup>::as_entire_binding(
    //                     &context,
    //                     material.clone(),
    //                     material
    //                         .resource_state(&params)
    //                         .expect("material is not initialized"),
    //                 ));
    //                 bind_groups.insert(texture_handle.clone(), binding);
    //             }
    //
    //             let binding = bind_groups.get_from_id(texture_handle.id()).unwrap();
    //             *bind_group_handle = binding;
    //             *texture_dimensions = dimensions;
    //         } else {
    //             // Image is not yet created, so we mark the Handle<Image> as `changed` to repeat
    //             // this operation.
    //             texture_handle.mark_changed();
    //         }
    //     }
    // }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    flip_v: f32,
    flip_h: f32,
    width: u32,
    height: u32,
    index: u32,
    _padding: f32,
}

unsafe impl AsGpuBuffer for SpriteInstance {}

impl<const OFFSET: u32> AsVertexBuffer<OFFSET> for SpriteInstance {
    const LABEL: &'static str = "sprite instance";
}

impl<const OFFSET: u32> VertexLayout<OFFSET> for SpriteInstance {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SpriteInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: OFFSET,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<f32>() as wgpu::BufferAddress,
                    shader_location: OFFSET + 1,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: OFFSET + 2,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: OFFSET + 3,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: OFFSET + 4,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

// Marks the kind of Material that a SpritePipeline is binded to
#[derive(WinnyComponent)]
pub struct MaterialMarker<M: Material>(PhantomData<M>);

#[derive(WinnyResource)]
pub struct SpriteBuffers {
    vertex_buffer: VertexBuffer,
    sprite_buffer: VertexBuffer,
    transform_buffer: VertexBuffer,
    sprites: Vec<(Sprite, Transform, TextureDimensions, Option<AnimatedSprite>)>,
}

impl SpriteBuffers {
    pub fn new(context: &RenderContext) -> Self {
        let init_buffer_size = 12;

        let vertex_buffer = <VertexUv as AsVertexBuffer<0>>::as_entire_buffer_empty(
            context,
            init_buffer_size,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        let sprite_buffer = <SpriteInstance as AsVertexBuffer<2>>::as_entire_buffer_empty(
            context,
            init_buffer_size,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        let transform_buffer = <Matrix4x4f as AsVertexBuffer<7>>::as_entire_buffer_empty(
            context,
            init_buffer_size,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );

        Self {
            vertex_buffer,
            sprite_buffer,
            transform_buffer,
            sprites: Vec::new(),
        }
    }

    pub fn append_sprites(
        &mut self,
        mut sprites: Vec<(Sprite, Transform, TextureDimensions, Option<AnimatedSprite>)>,
    ) {
        self.sprites.append(&mut sprites);
    }

    pub fn write_buffers(&mut self, context: &RenderContext, window: &Window) {
        self.sprites
            .sort_by(|(s1, _, _, _), (s2, _, _, _)| s1.z.cmp(&s2.z));

        let vertex_data: Vec<_> = self
            .sprites
            .iter()
            .map(|(s, _, d, a)| s.to_vertices(&window, d, a.as_ref()))
            .flatten()
            .collect();
        let sprite_data = self
            .sprites
            .iter()
            .map(|(s, _, _, a)| s.to_raw(a.as_ref()))
            .collect::<Vec<_>>();
        let transform_data = self
            .sprites
            .iter()
            .map(|(s, t, _, _)| s.transformation_matrix(t))
            .collect::<Vec<_>>();

        <VertexUv as AsVertexBuffer<0>>::write_buffer_resize::<VertexUv>(
            &context,
            &mut self.vertex_buffer,
            &vertex_data,
        );

        <SpriteInstance as AsVertexBuffer<2>>::write_buffer_resize::<SpriteInstance>(
            &context,
            &mut self.sprite_buffer,
            &sprite_data,
        );

        <Matrix4x4f as AsVertexBuffer<7>>::write_buffer_resize::<Matrix4x4f>(
            &context,
            &mut self.transform_buffer,
            &transform_data,
        );

        self.sprites.clear();
    }
}

#[derive(WinnyComponent)]
pub struct SpritePipeline {
    pipeline: RenderPipeline2d,
    camera_binding: BindGroup,
}

impl SpritePipeline {
    pub fn new<M: Material>(
        material: M,
        context: &RenderContext,
        server: &mut AssetServer,
        frag_shaders: &mut Assets<FragmentShaderSource>,
        vert_shader: &VertexShader,
        sprite_buffers: &SpriteBuffers,
    ) -> Self {
        let material_binding_layout = M::layout(context);

        let camera_binding = <&[CameraUniform] as AsBindGroup>::as_entire_binding_empty(
            context,
            &[],
            std::mem::size_of::<CameraUniform>() as u64,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        let pipeline = RenderPipeline2d::from_material_layout(
            format!("sprites: {}", M::LABEL).as_str(),
            FragmentType::Sprite,
            context,
            server,
            &[&camera_binding],
            &material_binding_layout,
            &[
                &sprite_buffers.vertex_buffer,
                &sprite_buffers.sprite_buffer,
                &sprite_buffers.transform_buffer,
            ],
            vert_shader,
            frag_shaders,
            material,
        );

        Self {
            pipeline,
            camera_binding,
        }
    }
}

fn prepare_for_render_pass<M: Material>(
    mut buffers: ResMut<SpriteBuffers>,
    sprite_pipeline: Query<SpritePipeline, With<MaterialMarker<M>>>,
    sprites: Query<
        (Sprite, Transform, TextureDimensions, Option<AnimatedSprite>),
        With<(M, SpritePipelineEntity, BindGroupHandle)>,
    >,
    context: Res<RenderContext>,
    camera: Query<(Camera, Transform)>,
    window: Res<Window>,
) {
    let Ok(pipeline) = sprite_pipeline.get_single() else {
        return;
    };

    let Ok((camera, transform)) = camera.get_single() else {
        return;
    };

    CameraUniform::write_buffer(
        &context,
        pipeline.camera_binding.single_buffer(),
        &[CameraUniform::from_camera(camera, transform, &window)],
    );

    buffers.append_sprites(
        sprites
            .iter()
            .map(|(s, t, d, a)| (s.clone(), t.clone(), d.clone(), a.cloned()))
            .collect::<Vec<_>>(),
    );
}

fn render_sprites(
    mut buffers: ResMut<SpriteBuffers>,
    mut encoder: ResMut<RenderEncoder>,
    context: Res<RenderContext>,
    sprite_pipelines: Query<(Entity, SpritePipeline)>,
    sprites: Query<
        (SpritePipelineEntity, Sprite, BindGroupHandle),
        With<(Transform, TextureDimensions)>,
    >,
    bind_groups: Res<AssetBindGroups>,
    view: Res<RenderView>,
    window: Res<Window>,
) {
    let num_sprites_in_buffer = buffers.sprites.len();
    buffers.write_buffers(&context, &window);

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

    let mut sprites = sprites.iter().collect::<Vec<_>>();
    sprites.sort_by(|(_, s1, _), (_, s2, _)| s1.z.cmp(&s2.z));

    render_pass.set_vertex_buffer(0, buffers.vertex_buffer.buffer().slice(..));
    render_pass.set_vertex_buffer(1, buffers.sprite_buffer.buffer().slice(..));
    render_pass.set_vertex_buffer(2, buffers.transform_buffer.buffer().slice(..));

    if sprites.len() != num_sprites_in_buffer {
        // println!("{}, {}", sprites.len(), num_sprites_in_buffer);
        return;
    }

    let mut last_pipeline_entity = None;
    let mut last_material_id = None;
    let mut offset = 0;
    for (pipeline_entity, _, material_binding) in sprites.iter() {
        let (_, pipeline) = sprite_pipelines.get(pipeline_entity.0).unwrap();

        if last_pipeline_entity != Some(pipeline_entity) {
            render_pass.set_pipeline(&pipeline.pipeline.0);
            last_pipeline_entity = Some(pipeline_entity);
        }

        if last_material_id != Some(material_binding.id()) {
            let material = bind_groups.get_from_id(material_binding.id()).unwrap();
            render_pass.set_bind_group(0, pipeline.camera_binding.binding(), &[]);
            render_pass.set_bind_group(1, &material.0.binding(), &[]);
            last_material_id = Some(material_binding.id());
        }

        render_pass.draw(offset * 6..offset * 6 + 6, offset..offset + 1);
        offset += 1;
    }
}
