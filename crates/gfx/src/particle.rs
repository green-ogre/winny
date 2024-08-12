use crate::{
    camera::{Camera, CameraUniform},
    render::{RenderEncoder, RenderView},
    render_pipeline::{
        bind_group::{self, AsBindGroup, BindGroup},
        buffer::AsGpuBuffer,
        material::{Material, Material2d},
        pipeline::{FragmentType, RenderPipeline2d},
        render_assets::{RenderAsset, RenderAssets},
        shader::{FragmentShaderSource, VertexShader, VertexShaderSource},
        vertex::{VertexUv, FULLSCREEN_QUAD_VERTEX_UV},
        vertex_buffer::{AsVertexBuffer, InstanceIndex, VertexBuffer},
    },
    texture::{Image, Texture, TextureDimensions},
    transform::Transform,
    AsWgpuResources, FragmentShader,
};
use app::prelude::*;
use app::{
    render_util::{RenderConfig, RenderContext},
    window::Window,
};
use asset::{server::AssetServer, *};
use cgmath::{Quaternion, Rad, Rotation3};
use ecs::{system_param::SystemParam, WinnyBundle, WinnyComponent, WinnyResource, *};
use math::{
    angle::Radf,
    matrix::{scale_matrix4x4f, world_to_screen_space_matrix4x4f, Matrix4x4f},
    vector::{Vec2f, Vec3f, Vec4f},
};
use rand::Rng;
use std::{fmt::Debug, marker::PhantomData, ops::Range};
use util::info;

// WARN: Particles and Sprites exist within different contexts, therefore they're z position has no
// relationship to each other, and one will always draw over the other
pub struct ParticlePlugin<M: Material>(PhantomData<M>);

impl<M: Material> Debug for ParticlePlugin<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ParticlePlugin")
    }
}

impl<M: Material> Plugin for ParticlePlugin<M> {
    fn build(&mut self, app: &mut App) {
        app.register_resource::<ParticleVertShaderHandle>()
            .add_systems(Schedule::PostUpdate, bind_new_particle_bundles::<M>)
            .add_systems(AppSchedule::PreRender, update_uniforms::<M>)
            .add_systems(
                AppSchedule::Render,
                (compute_emitters::<M>, render_emitters::<M>),
            );
    }
}

impl<M: Material> ParticlePlugin<M> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

/// For the time being, WebGPU will not run on wasm, therefore the compute shader and storage
/// buffers are not supported for the [`ParticlePlugin`].
pub struct CpuParticlePlugin<M: Material>(PhantomData<M>);

impl<M: Material> Debug for CpuParticlePlugin<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CpuParticlePlugin")
    }
}

impl<M: Material> Plugin for CpuParticlePlugin<M> {
    fn build(&mut self, app: &mut App) {
        app.register_resource::<ParticleVertShaderHandle>()
            .add_systems(Schedule::PostUpdate, bind_new_cpu_particle_bundles::<M>)
            .add_systems(AppSchedule::PreRender, update_cpu_uniforms::<M>)
            .add_systems(AppSchedule::Render, render_cpu_emitters::<M>);
    }
}

impl<M: Material> CpuParticlePlugin<M> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

#[derive(WinnyBundle)]
pub struct ParticleBundle<M: Material = Material2d> {
    pub emitter: ParticleEmitter,
    pub material: M,
    pub handle: Handle<Image>,
}

#[derive(WinnyComponent, Clone)]
pub struct ParticleEmitter {
    pub is_emitting: bool,
    pub num_particles: usize,
    pub lifetime: Range<f32>,
    pub width: f32,
    pub height: f32,
    pub particle_scale: Vec2f,
    pub particle_rotation: Radf,
    pub initial_velocity: Vec3f,
    pub acceleration: Vec3f,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            is_emitting: true,
            num_particles: 10,
            lifetime: 0.5..1.5,
            width: 400.,
            height: 400.,
            particle_scale: Vec2f::new(1., 1.),
            particle_rotation: Radf(0.),
            initial_velocity: Vec3f::zero(),
            acceleration: Vec3f::new(0., -200., 0.),
        }
    }
}

impl ParticleEmitter {
    pub(crate) fn particle_transformation_matrix(
        &self,
        config: &RenderConfig,
        emitter_transform: &Transform,
    ) -> Matrix4x4f {
        let angle: Radf = self.particle_rotation.into();
        let local_transformation = Transform {
            translation: Vec3f::zero(),
            rotation: Quaternion::from_angle_z(Rad(angle.0)),
            scale: self.particle_scale,
        };

        emitter_transform.as_matrix() * local_transformation.as_matrix()
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct VertexEmitterUniform {
    emitter_transform: Matrix4x4f,
}

unsafe impl AsGpuBuffer for VertexEmitterUniform {}

impl AsBindGroup for &[VertexEmitterUniform] {
    const LABEL: &'static str = "vertex emitter uniform";
    const BINDING_TYPES: &'static [wgpu::BindingType] = &[bind_group::UNIFORM];
    const VISIBILITY: &'static [wgpu::ShaderStages] = &[wgpu::ShaderStages::VERTEX];
}

impl VertexEmitterUniform {
    pub fn new(
        emitter: &ParticleEmitter,
        context: &RenderContext,
        emitter_transform: &Transform,
    ) -> Self {
        Self {
            emitter_transform: emitter
                .particle_transformation_matrix(&context.config, emitter_transform),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeEmitterUniform {
    initial_velocity: Vec4f,
    acceleration: Vec4f,
    time_delta: f32,
    time_elapsed: f32,
    width: f32,
    height: f32,
    max_lifetime: f32,
    min_lifetime: f32,
    screen_width: f32,
    screen_height: f32,
}

unsafe impl AsGpuBuffer for ComputeEmitterUniform {}

impl AsBindGroup for &[ComputeEmitterUniform] {
    const LABEL: &'static str = "compute emitter uniform";
    const BINDING_TYPES: &'static [wgpu::BindingType] = &[bind_group::UNIFORM];
    const VISIBILITY: &'static [wgpu::ShaderStages] = &[wgpu::ShaderStages::COMPUTE];
}

impl ComputeEmitterUniform {
    pub fn new(
        emitter: &ParticleEmitter,
        context: &RenderContext,
        emitter_transform: &Transform,
        dt: &DeltaTime,
    ) -> Self {
        Self {
            initial_velocity: Vec4f::to_homogenous(emitter.initial_velocity),
            acceleration: Vec4f::to_homogenous(emitter.acceleration),
            time_delta: dt.delta,
            time_elapsed: dt.wrapping_elapsed_as_seconds(),
            width: emitter.width / context.config.width() as f32 * emitter_transform.scale.x,
            height: emitter.height / context.config.height() as f32 * emitter_transform.scale.y,
            min_lifetime: emitter.lifetime.start,
            max_lifetime: emitter.lifetime.end,
            screen_width: context.config.width() as f32,
            screen_height: context.config.height() as f32,
        }
    }
}

/// Defines the ParticleInstance stored within the GPU particle buffer. The acceleration and
/// velocity are in world space, whereas the translation is in clip space.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawParticle {
    translation: Vec4f,
    velocity: Vec4f,
    acceleration: Vec4f,
    scale: Vec2f,
    /// From [`DeltaTime`] elapsed
    creation_time: f32,
    // Seconds
    lifetime: f32,
}

unsafe impl AsGpuBuffer for RawParticle {}

impl AsBindGroup for &[RawParticle] {
    const LABEL: &'static str = "raw particle";
    const BINDING_TYPES: &'static [wgpu::BindingType] = &[bind_group::READ_WRITE_STORAGE];
    const VISIBILITY: &'static [wgpu::ShaderStages] = &[wgpu::ShaderStages::COMPUTE];
}

impl RawParticle {
    pub fn new(
        translation: Vec4f,
        velocity: Vec4f,
        acceleration: Vec4f,
        scale: Vec2f,
        lifetime: f32,
        delta: &DeltaTime,
    ) -> Self {
        Self {
            translation,
            velocity,
            acceleration,
            scale,
            lifetime,
            creation_time: delta.wrapping_elapsed_as_seconds(),
        }
    }
}

/// [`RawParticle`] type for `read_only` GPU storage.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReadOnlyRawParticle {
    translation: Vec4f,
    velocity: Vec4f,
    acceleration: Vec4f,
    scale: Vec2f,
    /// From [`DeltaTime`] elapsed
    creation_time: f32,
    // Seconds
    lifetime: f32,
}

unsafe impl AsGpuBuffer for ReadOnlyRawParticle {}

impl AsBindGroup for &[ReadOnlyRawParticle] {
    const LABEL: &'static str = "read only raw particle";
    const BINDING_TYPES: &'static [wgpu::BindingType] = &[bind_group::READ_ONLY_STORAGE];
    const VISIBILITY: &'static [wgpu::ShaderStages] = &[wgpu::ShaderStages::VERTEX];
}

impl From<&RawParticle> for ReadOnlyRawParticle {
    fn from(value: &RawParticle) -> Self {
        // Safety:
        //     RawParticle and ReadOnlyRawParticle are both repr(C) with the same fields in the
        //     same order
        unsafe { std::mem::transmute(*value) }
    }
}

#[derive(WinnyComponent)]
pub struct ParticlePipeline<T: Material> {
    camera_binding: BindGroup,
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: RenderPipeline2d,
    material_resources: BindGroup,
    vertex_emitter_resources: BindGroup,
    vertex_particle_resources: BindGroup,
    compute_emitter_resources: BindGroup,
    compute_particle_resources: BindGroup,
    particle_vertex_buffer: VertexBuffer,
    alive_index_buffer: VertexBuffer,
    buffer_len: u32,

    // This information may or may not be valuable
    _phantom: PhantomData<T>,
}

// impl<M: Material> ParticlePipeline<M> {
//     pub fn new<'s>(
//         vert_shader: &VertexShader,
//         frag_shaders: &mut Assets<FragmentShaderSource>,
//         server: &mut AssetServer,
//         material: M,
//         emitter: &ParticleEmitter,
//         buffer_len: u32,
//         context: &RenderContext,
//         textures: &RenderAssets<Texture>,
//         texture_dimensions: &TextureDimensions,
//         emitter_transform: &Transform,
//         delta: &DeltaTime,
//         window: &Window,
//     ) -> Self {
//         let material_resources = <M as AsBindGroup>::as_entire_binding(
//             context,
//             material.clone(),
//             material.resource_state(textures).unwrap(),
//         );
//
//         let camera_binding = <&[CameraUniform] as AsBindGroup>::as_entire_binding_empty(
//             context,
//             &[],
//             std::mem::size_of::<CameraUniform>() as u64,
//             wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
//         );
//
//         let mut vertices = FULLSCREEN_QUAD_VERTEX_UV;
//         let mut atlas_scaling = (1.0, 1.0);
//         let normalized_scale = Vec2f::new(
//             texture_dimensions.width() / window.viewport.width(),
//             texture_dimensions.height() / window.viewport.height(),
//         );
//         let image_scale = scale_matrix4x4f(normalized_scale);
//         for vert in vertices.iter_mut() {
//             vert.position = image_scale * vert.position;
//         }
//         let particle_vertex_buffer = <VertexUv as AsVertexBuffer<0>>::as_entire_buffer(
//             &context,
//             &vertices,
//             wgpu::BufferUsages::VERTEX,
//         );
//
//         let particles = generate_particles_with_conditions(emitter, delta, &context.config);
//
//         let vertex_particle_resources = <&[ReadOnlyRawParticle] as AsBindGroup>::as_entire_binding(
//             context,
//             &particles.iter().map(|p| p.into()).collect::<Vec<_>>(),
//             wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
//         );
//
//         let compute_particle_resources = <&[RawParticle] as AsBindGroup>::as_entire_binding(
//             context,
//             &particles,
//             wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
//         );
//
//         let alive_indexes = (0..buffer_len)
//             .map(|i| InstanceIndex(i))
//             .collect::<Vec<_>>();
//         let alive_index_buffer = <InstanceIndex as AsVertexBuffer<2>>::as_entire_buffer(
//             &context,
//             &alive_indexes,
//             wgpu::BufferUsages::VERTEX,
//         );
//
//         let vertex_emitter_uniform = VertexEmitterUniform::new(emitter, context, emitter_transform);
//         let vertex_emitter_resources = <&[VertexEmitterUniform] as AsBindGroup>::as_entire_binding(
//             context,
//             &[vertex_emitter_uniform],
//             wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
//         );
//
//         let render_pipeline = RenderPipeline2d::from_material_layout(
//             format!("particles: {}", M::LABEL).as_str(),
//             FragmentType::Particle,
//             context,
//             server,
//             &[
//                 &vertex_emitter_resources,
//                 &vertex_particle_resources,
//                 &camera_binding,
//             ],
//             material_resources.layout(),
//             &[&particle_vertex_buffer, &alive_index_buffer],
//             vert_shader,
//             frag_shaders,
//             material,
//         );
//
//         let compute_emitter_uniform =
//             ComputeEmitterUniform::new(emitter, context, emitter_transform, delta);
//         let compute_emitter_resources =
//             <&[ComputeEmitterUniform] as AsBindGroup>::as_entire_binding(
//                 context,
//                 &[compute_emitter_uniform],
//                 wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
//             );
//
//         let compute_shader = wgpu::ShaderModuleDescriptor {
//             label: Some("particle compute"),
//             source: wgpu::ShaderSource::Wgsl(
//                 include_str!("../../../res/shaders/particles_compute.wgsl").into(),
//             ),
//         };
//         let compute_layout =
//             context
//                 .device
//                 .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
//                     label: Some("particle compute"),
//                     bind_group_layouts: &[
//                         &compute_particle_resources.layout(),
//                         &compute_emitter_resources.layout(),
//                     ],
//                     push_constant_ranges: &[],
//                 });
//         let compute_shader = context.device.create_shader_module(compute_shader);
//
//         let compute_pipeline =
//             context
//                 .device
//                 .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
//                     label: Some("particle compute"),
//                     layout: Some(&compute_layout),
//                     module: &compute_shader,
//                     entry_point: "main",
//                     compilation_options: wgpu::PipelineCompilationOptions::default(),
//                 });
//
//         Self {
//             camera_binding,
//             render_pipeline,
//             compute_pipeline,
//             material_resources,
//             particle_vertex_buffer,
//             vertex_emitter_resources,
//             vertex_particle_resources,
//             compute_emitter_resources,
//             compute_particle_resources,
//             alive_index_buffer,
//             buffer_len,
//
//             _phantom: PhantomData,
//         }
//     }
// }

#[derive(WinnyComponent)]
pub struct CpuParticlePipeline<T: Material> {
    camera_binding: BindGroup,
    render_pipeline: RenderPipeline2d,
    material_resources: BindGroup,
    particle_transform_buffer: VertexBuffer,
    particle_vertex_buffer: VertexBuffer,
    particles: Vec<RawParticle>,
    emitter_uniform: ComputeEmitterUniform,

    // This information may or may not be valuable
    _phantom: PhantomData<T>,
}

impl<M: Material> CpuParticlePipeline<M> {
    pub fn new<'s>(
        vert_shader: &VertexShader,
        frag_shader: &FragmentShader,
        material: M,
        emitter: &ParticleEmitter,
        buffer_len: u32,
        context: &Res<RenderContext>,
        texture_dimensions: &TextureDimensions,
        emitter_transform: &Transform,
        delta: &DeltaTime,
        window: &Window,
        state: <M as AsWgpuResources>::State<'s>,
    ) -> Self {
        let material_resources =
            <M as AsBindGroup>::as_entire_binding(context, material.clone(), state);

        let camera_binding = <&[CameraUniform] as AsBindGroup>::as_entire_binding_empty(
            context,
            &[],
            std::mem::size_of::<CameraUniform>() as u64,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        let mut vertices = FULLSCREEN_QUAD_VERTEX_UV;
        let mut atlas_scaling = (1.0, 1.0);
        let normalized_scale = Vec2f::new(
            texture_dimensions.width() / window.viewport.width(),
            texture_dimensions.height() / window.viewport.height(),
        );
        let image_scale = scale_matrix4x4f(normalized_scale);
        for vert in vertices.iter_mut() {
            vert.position = image_scale * vert.position;
        }
        let particle_vertex_buffer = <VertexUv as AsVertexBuffer<0>>::as_entire_buffer(
            &context,
            &vertices,
            wgpu::BufferUsages::VERTEX,
        );

        let particles = generate_particles_with_conditions(emitter, delta, &context.config);

        let particle_transform_buffer = <Matrix4x4f as AsVertexBuffer<2>>::as_entire_buffer_empty(
            context,
            particles.len() as u64 * std::mem::size_of::<Matrix4x4f>() as u64,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );

        let render_pipeline = RenderPipeline2d::new(
            format!("cpu particles: {}", M::LABEL).as_str(),
            context,
            &[&camera_binding.layout()],
            &[
                particle_vertex_buffer.layout(),
                particle_transform_buffer.layout(),
            ],
            vert_shader,
            frag_shader,
            wgpu::BlendState::ALPHA_BLENDING,
            None,
        );

        let emitter_uniform =
            ComputeEmitterUniform::new(emitter, context, emitter_transform, delta);

        Self {
            camera_binding,
            render_pipeline,
            material_resources,
            particle_vertex_buffer,
            particle_transform_buffer,
            particles,
            emitter_uniform,

            _phantom: PhantomData,
        }
    }

    pub fn update_particles(&mut self, delta: &DeltaTime) {
        let mut rng = rand::thread_rng();
        for particle in self.particles.iter_mut() {
            if particle.lifetime <= 0.0 {
                particle.translation.x = rng.gen_range(0.0..self.emitter_uniform.width)
                    - 0.5 * self.emitter_uniform.width;
                particle.translation.y = rng.gen_range(0.0..self.emitter_uniform.height)
                    - 0.5 * self.emitter_uniform.height;
                particle.lifetime = rng.gen_range(
                    self.emitter_uniform.min_lifetime..self.emitter_uniform.max_lifetime,
                );
                particle.creation_time = delta.wrapping_elapsed_as_seconds();
                particle.velocity = self.emitter_uniform.initial_velocity;
                particle.acceleration = self.emitter_uniform.acceleration;
            } else {
                particle.translation.x +=
                    particle.velocity.x / self.emitter_uniform.screen_width * delta.delta;
                particle.translation.y +=
                    particle.velocity.y / self.emitter_uniform.screen_height * delta.delta;
                particle.velocity.x += particle.acceleration.x * delta.delta;
                particle.velocity.y += particle.acceleration.y * delta.delta;
                particle.lifetime -= delta.delta;
            }
        }
    }

    pub fn particle_transforms(
        &self,
        context: &RenderContext,
        emitter: &ParticleEmitter,
        emitter_transform: &Transform,
    ) -> Vec<Matrix4x4f> {
        let emitter_transform =
            emitter.particle_transformation_matrix(&context.config, emitter_transform);
        self.particles
            .iter()
            .map(|p| {
                Transform {
                    translation: p.translation.into(),
                    scale: p.scale,
                    ..Default::default()
                }
                .as_matrix()
                    * emitter_transform
            })
            .collect::<Vec<_>>()
    }
}

fn generate_particles_with_conditions(
    emitter: &ParticleEmitter,
    delta: &DeltaTime,
    config: &RenderConfig,
) -> Vec<RawParticle> {
    let mut rng = rand::thread_rng();
    let mut particles = Vec::with_capacity(emitter.num_particles);
    let world_to_screen_space = world_to_screen_space_matrix4x4f(config.widthf(), config.heightf());
    for _ in 0..emitter.num_particles {
        let x = rng.gen_range(0.0..emitter.width) - 0.5 * emitter.width;
        let y = rng.gen_range(0.0..emitter.height) - 0.5 * emitter.height;
        let lifetime = rng.gen_range(0.0..emitter.lifetime.end);
        particles.push(RawParticle::new(
            world_to_screen_space * Vec4f::to_homogenous(Vec3f::new(x, y, 0.)),
            Vec4f::to_homogenous(emitter.initial_velocity),
            Vec4f::to_homogenous(emitter.acceleration),
            Vec2f::new(1., 1.),
            lifetime,
            delta,
        ));
    }

    particles
}

#[derive(WinnyResource)]
struct ParticleVertShaderHandle(Handle<VertexShaderSource>);

fn bind_new_particle_bundles<'w, M: Material>(
    mut commands: Commands,
    mut server: ResMut<AssetServer>,
    context: Res<RenderContext>,
    bundles: Query<
        (Entity, Handle<Image>, Transform, ParticleEmitter, M),
        Without<ParticlePipeline<M>>,
    >,
    images: Res<Assets<Image>>,
    mut textures: ResMut<RenderAssets<Texture>>,
    texture_params: <Texture as RenderAsset>::Params<'_>,
    delta: Res<DeltaTime>,
    particle_vert_shader_handle: Option<Res<ParticleVertShaderHandle>>,
    mut vert_shaders: ResMut<Assets<VertexShaderSource>>,
    mut frag_shaders: ResMut<Assets<FragmentShaderSource>>,
    window: Res<Window>,
) {
    // for (entity, handle, transform, emitter, material) in bundles.iter() {
    //     if let Some(vert_shader_handle) = &particle_vert_shader_handle {
    //         if let Some(image) = images.get(handle) {
    //             let texture = textures
    //                 .entry(handle.clone())
    //                 .or_insert_with(|| Texture::prepare_asset(image, &texture_params));
    //
    //             if let Some(vert_shader) = vert_shaders.get_mut(&vert_shader_handle.0) {
    //                 if !material.is_init(&server, &frag_shaders) {
    //                     continue;
    //                 }
    //
    //                 let dimensions = TextureDimensions::from_texture(&texture);
    //
    //                 let particle_render_pipeline = ParticlePipeline::new(
    //                     vert_shader.shader(&context),
    //                     &mut frag_shaders,
    //                     &mut server,
    //                     material.clone(),
    //                     emitter,
    //                     emitter.num_particles as u32,
    //                     &context,
    //                     &textures,
    //                     &dimensions,
    //                     &transform,
    //                     &delta,
    //                     &window,
    //                 );
    //
    //                 commands
    //                     .get_entity(entity)
    //                     .insert((particle_render_pipeline, dimensions));
    //             } else {
    //                 // util::tracing::error!(
    //                 //     "Could not retrieve asset particle pipeline vertex shader"
    //                 // );
    //             }
    //         }
    //     } else {
    //         // This will only run when the first particle pipeline is built, so we compile the
    //         // shader
    //         let vert_shader = wgpu::ShaderModuleDescriptor {
    //             label: Some("particles vert"),
    //             source: wgpu::ShaderSource::Wgsl(
    //                 include_str!("../../../res/shaders/particle_vert.wgsl").into(),
    //             ),
    //         };
    //         let vert_shader = VertexShader(context.device.create_shader_module(vert_shader));
    //         let handle = vert_shaders.add(VertexShaderSource(
    //             include_str!("../../../res/shaders/particle_vert.wgsl").into(),
    //             Some(vert_shader),
    //         ));
    //         commands.insert_resource(ParticleVertShaderHandle(handle));
    //     }
    // }
}

fn bind_new_cpu_particle_bundles<M: Material>(
    mut commands: Commands,
    mut server: ResMut<AssetServer>,
    context: Res<RenderContext>,
    bundles: Query<
        (Entity, Handle<Image>, Transform, ParticleEmitter, M),
        Without<CpuParticlePipeline<M>>,
    >,
    images: Res<Assets<Image>>,
    mut textures: ResMut<RenderAssets<Texture>>,
    texture_params: <Texture as RenderAsset>::Params<'_>,
    delta: Res<DeltaTime>,
    particle_vert_shader_handle: Option<Res<ParticleVertShaderHandle>>,
    mut vert_shaders: ResMut<Assets<VertexShaderSource>>,
    mut frag_shaders: ResMut<Assets<FragmentShaderSource>>,
    window: Res<Window>,
) {
    for (entity, handle, transform, emitter, material) in bundles.iter() {
        if let Some(vert_shader_handle) = &particle_vert_shader_handle {
            if let Some(image) = images.get(handle) {
                let texture = textures
                    .entry(handle.clone())
                    .or_insert_with(|| Texture::prepare_asset(image, &texture_params));

                if let Some(vert_shader) = vert_shaders.get_mut(&vert_shader_handle.0) {
                    if let Some(frag) =
                        frag_shaders.get_mut(&material.cpu_particle_fragment_shader(&server))
                    {
                        let dimensions = TextureDimensions::from_texture(&texture);
                        if let Some(state) =
                            material.resource_state(&mut textures, &images, &context)
                        {
                            let particle_render_pipeline = CpuParticlePipeline::new(
                                vert_shader.shader(&context),
                                frag.shader(&context),
                                material.clone(),
                                emitter,
                                emitter.num_particles as u32,
                                &context,
                                &dimensions,
                                &transform,
                                &delta,
                                &window,
                                state,
                            );

                            commands
                                .get_entity(entity)
                                .insert((particle_render_pipeline, dimensions));
                        }
                    } else {
                    }
                } else {
                    // util::tracing::error!(
                    //     "Could not retrieve asset particle pipeline vertex shader"
                    // );
                }
            }
        } else {
            // This will only run when the first particle pipeline is built, so we compile the
            // shader
            let vert_shader = wgpu::ShaderModuleDescriptor {
                label: Some("particles vert"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../../res/shaders/cpu_particle_vert.wgsl").into(),
                ),
            };
            let vert_shader = VertexShader(context.device.create_shader_module(vert_shader));
            let handle = vert_shaders.add(VertexShaderSource(
                include_str!("../../../res/shaders/cpu_particle_vert.wgsl").into(),
                Some(vert_shader),
            ));
            commands.insert_resource(ParticleVertShaderHandle(handle));
        }
    }
}

fn update_uniforms<M: Material>(
    mut emitters: Query<(
        Mut<ParticlePipeline<M>>,
        Transform,
        ParticleEmitter,
        TextureDimensions,
    )>,
    dt: Res<DeltaTime>,
    context: Res<RenderContext>,
    camera: Query<(Camera, Transform)>,
    window: Res<Window>,
) {
    for (pipeline, transform, emitter, dimensions) in emitters.iter_mut() {
        let vertex_emitter = VertexEmitterUniform::new(emitter, &context, transform);
        VertexEmitterUniform::write_buffer(
            &context,
            &pipeline.vertex_emitter_resources.single_buffer(),
            &[vertex_emitter],
        );

        let compute_emitter = ComputeEmitterUniform::new(emitter, &context, transform, &dt);
        ComputeEmitterUniform::write_buffer(
            &context,
            &pipeline.compute_emitter_resources.single_buffer(),
            &[compute_emitter],
        );

        let Ok((camera, transform)) = camera.get_single() else {
            continue;
        };

        CameraUniform::write_buffer(
            &context,
            pipeline.camera_binding.single_buffer(),
            &[CameraUniform::from_camera(camera, transform, &window)],
        );
    }
}

fn update_cpu_uniforms<M: Material>(
    mut emitters: Query<(
        Mut<CpuParticlePipeline<M>>,
        Transform,
        ParticleEmitter,
        TextureDimensions,
    )>,
    dt: Res<DeltaTime>,
    context: Res<RenderContext>,
    camera: Query<(Camera, Transform)>,
    window: Res<Window>,
) {
    for (pipeline, transform, emitter, dimensions) in emitters.iter_mut() {
        pipeline.update_particles(&dt);

        Matrix4x4f::write_buffer(
            &context,
            &pipeline.particle_transform_buffer.buffer(),
            &pipeline.particle_transforms(&context, emitter, transform),
        );

        let Ok((camera, transform)) = camera.get_single() else {
            continue;
        };

        CameraUniform::write_buffer(
            &context,
            pipeline.camera_binding.single_buffer(),
            &[CameraUniform::from_camera(camera, transform, &window)],
        );
    }
}

fn compute_emitters<M: Material>(
    mut encoder: ResMut<RenderEncoder>,
    emitters: Query<(ParticlePipeline<M>, ParticleEmitter), With<Transform>>,
) {
    for (pipeline, emitter) in emitters.iter().filter(|(_, e)| e.is_emitting) {
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("particle compute"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&pipeline.compute_pipeline);
            compute_pass.set_bind_group(0, &pipeline.compute_particle_resources.binding(), &[]);
            compute_pass.set_bind_group(1, &pipeline.compute_emitter_resources.binding(), &[]);

            let mut dispatched = 0;
            while dispatched < emitter.num_particles {
                compute_pass.dispatch_workgroups(65535, 1, 1);
                dispatched += 65535
            }
        }

        encoder.copy_buffer_to_buffer(
            &pipeline.compute_particle_resources.single_buffer(),
            0,
            &pipeline.vertex_particle_resources.single_buffer(),
            0,
            pipeline.compute_particle_resources.single_buffer().size(),
        );
    }
}

fn render_emitters<M: Material>(
    mut encoder: ResMut<RenderEncoder>,
    view: Res<RenderView>,
    emitters: Query<(ParticlePipeline<M>, ParticleEmitter), With<Transform>>,
) {
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("particles"),
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

    for (pipeline, _) in emitters.iter().filter(|(_, e)| e.is_emitting) {
        render_pass.set_pipeline(&pipeline.render_pipeline.0);
        render_pass.set_vertex_buffer(0, pipeline.particle_vertex_buffer.buffer().slice(..));
        render_pass.set_vertex_buffer(1, pipeline.alive_index_buffer.buffer().slice(..));
        render_pass.set_bind_group(0, &pipeline.vertex_emitter_resources.binding(), &[]);
        render_pass.set_bind_group(1, &pipeline.vertex_particle_resources.binding(), &[]);
        render_pass.set_bind_group(2, &pipeline.camera_binding.binding(), &[]);
        render_pass.set_bind_group(3, &pipeline.material_resources.binding(), &[]);
        render_pass.draw(0..6, 0..pipeline.buffer_len);
    }
}

fn render_cpu_emitters<M: Material>(
    mut encoder: ResMut<RenderEncoder>,
    context: Res<RenderContext>,
    view: Res<RenderView>,
    emitters: Query<(CpuParticlePipeline<M>, M, ParticleEmitter), With<Transform>>,
) {
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("cpu particles"),
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

    for (pipeline, mat, _) in emitters.iter().filter(|(_, _, e)| e.is_emitting) {
        // mat.update(&context, &pipeline.material_resources);
        render_pass.set_pipeline(&pipeline.render_pipeline.0);
        render_pass.set_vertex_buffer(0, pipeline.particle_vertex_buffer.buffer().slice(..));
        render_pass.set_vertex_buffer(1, pipeline.particle_transform_buffer.buffer().slice(..));
        render_pass.set_bind_group(0, &pipeline.camera_binding.binding(), &[]);
        render_pass.set_bind_group(1, &pipeline.material_resources.binding(), &[]);
        render_pass.draw(0..6, 0..pipeline.particles.len() as u32);
    }
}
