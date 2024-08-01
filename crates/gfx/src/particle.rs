use crate::{
    create_render_pipeline,
    render_pipeline::{
        bind_group::{self, AsBindGroup, AsWgpuResources, FragTexture, WgpuResource},
        buffer::AsGpuBuffer,
        material::{self, Material, Material2d},
        vertex::{VertexUv, FULLSCREEN_QUAD_VERTEX_UV},
        vertex_buffer::{InstanceIndex, VertexBuffer},
    },
    texture::{SamplerFilterType, Texture, TextureDimensions},
    transform::Transform,
};
use app::{
    app::{AppSchedule, Schedule},
    plugins::Plugin,
    time::DeltaTime,
};
use asset::{Assets, Handle};
use cgmath::{Quaternion, Rad, Rotation3};
use ecs::{prelude::*, WinnyBundle, WinnyComponent};
use rand::Rng;
use render::prelude::*;
use std::ops::Range;
use winny_math::{
    angle::Radf,
    matrix::{world_to_screen_space_matrix4x4f, Matrix4x4f},
    vector::{Vec2f, Vec3f, Vec4f},
};

// WARN: Particles and Sprites exist within different contexts, therefore they're z position has no
// relationship to each other, and one will always draw over the other
pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&mut self, app: &mut app::app::App) {
        app.add_systems(
            Schedule::PostUpdate,
            bind_new_particle_bundles::<Material2d>,
        )
        .add_systems(
            AppSchedule::PreRender,
            update_emitter_uniforms::<Material2d>,
        )
        .add_systems(
            AppSchedule::Render,
            (
                compute_emitters::<Material2d>,
                render_emitters::<Material2d>,
            ),
        );
    }
}

#[derive(WinnyBundle)]
pub struct ParticleBundle<M: Material = Material2d> {
    pub emitter: ParticleEmitter,
    pub material: M,
    pub handle: Handle<Texture>,
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
        texture: &TextureDimensions,
    ) -> Matrix4x4f {
        let angle: Radf = self.particle_rotation.into();
        let texture_scale = Vec2f::new(
            texture.width() / config.width() as f32,
            texture.height() / config.height() as f32,
        );
        let local_transformation = Transform {
            translation: Vec3f::zero(),
            rotation: Quaternion::from_angle_z(Rad(angle.0)),
            scale: self.particle_scale * texture_scale,
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
        texture_dimensions: &TextureDimensions,
    ) -> Self {
        Self {
            emitter_transform: emitter.particle_transformation_matrix(
                &context.config,
                emitter_transform,
                texture_dimensions,
            ),
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
            width: emitter.width / context.config.width() as f32 * emitter_transform.scale.v[0],
            height: emitter.height / context.config.height() as f32 * emitter_transform.scale.v[1],
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
    material: T,
    material_resources: Vec<WgpuResource>,
    material_binding: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,
    particle_vertex_buffer: wgpu::Buffer,
    vertex_emitter_buffer: wgpu::Buffer,
    vertex_emitter_binding: wgpu::BindGroup,
    vertex_particle_binding: wgpu::BindGroup,
    vertex_particle_buffer: wgpu::Buffer,
    compute_emitter_buffer: wgpu::Buffer,
    compute_emitter_binding: wgpu::BindGroup,
    compute_particle_binding: wgpu::BindGroup,
    compute_particle_buffer: wgpu::Buffer,
    alive_index_buffer: wgpu::Buffer,
    buffer_len: u32,
}

// TODO: does not share texture bindings
impl<M: Material> ParticlePipeline<M> {
    pub fn new<'s>(
        material: M,
        emitter: &ParticleEmitter,
        buffer_len: u32,
        context: &RenderContext,
        texture: &Texture,
        emitter_transform: &Transform,
        delta: &DeltaTime,
    ) -> Self {
        let (material_resources, material_layout, material_binding) =
            <M as AsBindGroup>::as_entire_binding(
                context,
                material.clone(),
                &material.resource_state(texture),
            );

        let vertices = FULLSCREEN_QUAD_VERTEX_UV;
        let (_, particle_vertex_buffer, particle_vertex_layout) =
            <VertexUv as VertexBuffer<0, VertexUv, VertexUv>>::as_entire_buffer(
                &context,
                &vertices,
                &(),
                wgpu::BufferUsages::VERTEX,
            );

        let particles = generate_particles_with_conditions(emitter, delta, &context.config);

        // ) -> (Vec<WgpuResource>, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let (vertex_particle_buffer, vertex_particle_layout, vertex_particle_binding) =
            <&[ReadOnlyRawParticle] as AsBindGroup>::as_entire_binding_single_buffer(
                context,
                &particles.iter().map(|p| p.into()).collect::<Vec<_>>(),
                &(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST),
            );

        let (compute_particle_buffer, compute_particle_layout, compute_particle_binding) =
            <&[RawParticle] as AsBindGroup>::as_entire_binding_single_buffer(
                context,
                &particles,
                &(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC),
            );

        let alive_indexes = (0..buffer_len)
            .map(|i| InstanceIndex(i))
            .collect::<Vec<_>>();
        let (_, alive_index_buffer, alive_index_layout) =
            <InstanceIndex as VertexBuffer<2, InstanceIndex, u32>>::as_entire_buffer(
                &context,
                &alive_indexes,
                &(),
                wgpu::BufferUsages::VERTEX,
            );

        let vertex_emitter_uniform = VertexEmitterUniform::new(
            emitter,
            context,
            emitter_transform,
            &TextureDimensions::from_texture(texture),
        );
        let (vertex_emitter_buffer, vertex_emitter_layout, vertex_emitter_binding) =
            <&[VertexEmitterUniform] as AsBindGroup>::as_entire_binding_single_buffer(
                context,
                &[vertex_emitter_uniform],
                &(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST),
            );

        let render_pipeline = create_particle_render_pipeline(
            &context.device,
            &context.config,
            &[
                &vertex_emitter_layout,
                &vertex_particle_layout,
                &material_layout,
            ],
            &[particle_vertex_layout, alive_index_layout],
            material.fragment_shader(),
        );

        let compute_emitter_uniform =
            ComputeEmitterUniform::new(emitter, context, emitter_transform, delta);
        let (compute_emitter_buffer, compute_emitter_layout, compute_emitter_binding) =
            <&[ComputeEmitterUniform] as AsBindGroup>::as_entire_binding_single_buffer(
                context,
                &[compute_emitter_uniform],
                &(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST),
            );

        let compute_shader = wgpu::ShaderModuleDescriptor {
            label: Some("particle compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/particles_compute.wgsl").into()),
        };
        let compute_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("particle compute"),
                    bind_group_layouts: &[&compute_particle_layout, &compute_emitter_layout],
                    push_constant_ranges: &[],
                });
        let compute_shader = context.device.create_shader_module(compute_shader);

        let compute_pipeline =
            context
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("particle compute"),
                    layout: Some(&compute_layout),
                    module: &compute_shader,
                    entry_point: "main",
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                });

        Self {
            material,
            material_resources,
            material_binding,
            render_pipeline,
            compute_pipeline,
            particle_vertex_buffer,
            vertex_emitter_buffer,
            vertex_emitter_binding,
            vertex_particle_binding,
            vertex_particle_buffer,
            compute_emitter_buffer,
            compute_emitter_binding,
            compute_particle_buffer,
            compute_particle_binding,
            alive_index_buffer,
            buffer_len,
        }
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

fn bind_new_particle_bundles<M: Material>(
    mut commands: Commands,
    context: Res<RenderContext>,
    bundles: Query<
        (Entity, Handle<Texture>, Transform, ParticleEmitter, M),
        Without<ParticlePipeline<M>>,
    >,
    textures: Res<Assets<Texture>>,
    delta: Res<DeltaTime>,
) {
    for (entity, handle, transform, emitter, material) in bundles.iter() {
        if let Some(texture) = textures.get(handle) {
            let particle_render_pipeline = ParticlePipeline::new(
                material.clone(),
                emitter,
                emitter.num_particles as u32,
                &context,
                &texture,
                &transform,
                &delta,
            );

            let dimensions = TextureDimensions::from_texture(&texture);
            commands
                .get_entity(entity)
                .insert((particle_render_pipeline, dimensions));
        }
    }
}

fn create_particle_render_pipeline(
    device: &RenderDevice,
    config: &RenderConfig,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    vertex_layouts: &[wgpu::VertexBufferLayout<'static>],
    frag_shader: &'static str,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("particles"),
        bind_group_layouts,
        push_constant_ranges: &[],
    });

    let vert_shader = wgpu::ShaderModuleDescriptor {
        label: Some("particles vert"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/particles.wgsl").into()),
    };

    let frag_shader = wgpu::ShaderModuleDescriptor {
        label: Some("particles frag"),
        source: wgpu::ShaderSource::Wgsl(frag_shader.into()),
    };

    let vert_shader = device.create_shader_module(vert_shader);
    let frag_shader = device.create_shader_module(frag_shader);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("particles"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &vert_shader,
            entry_point: "vs_main",
            buffers: vertex_layouts,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &frag_shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format(),
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        // cache: None,
    })
}

fn update_emitter_uniforms<M: Material>(
    mut emitters: Query<(
        Mut<ParticlePipeline<M>>,
        Transform,
        ParticleEmitter,
        TextureDimensions,
    )>,
    dt: Res<DeltaTime>,
    context: Res<RenderContext>,
) {
    for (pipeline, transform, emitter, dimensions) in emitters.iter_mut() {
        let vertex_emitter = VertexEmitterUniform::new(emitter, &context, transform, dimensions);
        VertexEmitterUniform::write_buffer(
            &context,
            &pipeline.vertex_emitter_buffer,
            &[vertex_emitter],
        );

        let compute_emitter = ComputeEmitterUniform::new(emitter, &context, transform, &dt);
        ComputeEmitterUniform::write_buffer(
            &context,
            &pipeline.compute_emitter_buffer,
            &[compute_emitter],
        );
    }
}

fn compute_emitters<M: Material>(
    mut encoder: ResMut<RenderEncoder>,
    emitters: Query<(ParticlePipeline<M>, ParticleEmitter), With<Transform>>,
) {
    for (pipeline, emitter) in emitters.iter() {
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("particle compute"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&pipeline.compute_pipeline);
            compute_pass.set_bind_group(0, &pipeline.compute_particle_binding, &[]);
            compute_pass.set_bind_group(1, &pipeline.compute_emitter_binding, &[]);
            compute_pass.dispatch_workgroups(emitter.num_particles as u32, 1, 1);
        }

        encoder.copy_buffer_to_buffer(
            &pipeline.compute_particle_buffer,
            0,
            &pipeline.vertex_particle_buffer,
            0,
            pipeline.compute_particle_buffer.size(),
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
        render_pass.set_pipeline(&pipeline.render_pipeline);
        render_pass.set_vertex_buffer(0, pipeline.particle_vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, pipeline.alive_index_buffer.slice(..));
        render_pass.set_bind_group(0, &pipeline.vertex_emitter_binding, &[]);
        render_pass.set_bind_group(1, &pipeline.vertex_particle_binding, &[]);
        render_pass.set_bind_group(2, &pipeline.material_binding, &[]);
        render_pass.draw(0..6, 0..pipeline.buffer_len);
    }
}
