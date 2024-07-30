use crate::{
    create_buffer_bind_group, create_compute_pipeline, create_render_pipeline,
    create_texture_bind_group,
    noise::NoisePlugin,
    texture::Texture,
    transform::{new_transform_bind_group, new_transform_bind_group_layout, Transform},
    vertex::{VertexLayout, VertexUv, FULLSCREEN_QUAD_VERTEX_UV},
};
use app::{
    app::{AppSchedule, Schedule},
    plugins::Plugin,
    time::DeltaTime,
};
use asset::{Assets, Handle};
use cgmath::Matrix4;
use ecs::{prelude::*, WinnyBundle, WinnyComponent, WinnyResource};
use rand::Rng;
use render::prelude::*;
use std::ops::Range;
use wgpu::util::DeviceExt;
use winny_math::{
    angle::Radf,
    matrix::{
        rotation_2d_matrix4x4f, scale_matrix4x4f, translation_matrix4x4f,
        world_to_screen_space_matrix4x4f, Matrix4x4f,
    },
    vector::{Vec2f, Vec3f, Vec4f},
};

// WARN: Particles and Sprites exist within different contexts, therefore they're z position has no
// relationship to each other, and one will always draw over the other
pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&mut self, app: &mut app::app::App) {
        app.add_plugins(NoisePlugin::new("noise/particles.png"))
            .add_systems(Schedule::PostUpdate, bind_new_particle_bundles)
            .add_systems(AppSchedule::PreRender, update_emitter_transforms)
            .add_systems(AppSchedule::Render, (compute_emitters, render_emitters));
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct VertexEmitterUniform {
    particle_transform: Matrix4x4f,
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

impl ComputeEmitterUniform {
    pub fn new(config: &RenderConfig, emitter: &ParticleEmitter) -> Self {
        Self {
            initial_velocity: Vec4f::to_homogenous(emitter.initial_velocity),
            acceleration: Vec4f::to_homogenous(emitter.acceleration),
            time_delta: 0.,
            time_elapsed: 0.,
            width: emitter.width / config.width(),
            height: emitter.height / config.height(),
            min_lifetime: emitter.lifetime.start,
            max_lifetime: emitter.lifetime.end,
            screen_width: config.width(),
            screen_height: config.height(),
        }
    }
}

#[derive(WinnyComponent)]
#[allow(dead_code)]
pub struct ParticlePipeline {
    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,

    vertex_emitter_uniform: VertexEmitterUniform,
    vertex_emitter_buffer: wgpu::Buffer,
    vertex_emitter_bind_group: wgpu::BindGroup,
    vertex_particle_binding: wgpu::BindGroup,
    vertex_particle_buffer: wgpu::Buffer,

    compute_emitter_uniform: ComputeEmitterUniform,
    compute_emitter_buffer: wgpu::Buffer,
    compute_emitter_bind_group: wgpu::BindGroup,
    compute_particle_binding: wgpu::BindGroup,
    compute_particle_buffer: wgpu::Buffer,

    alive_index_buffer: wgpu::Buffer,
    // Must maintain alignment with buffer
    alive_indexes: Vec<u32>,
    // dead_index_buffer: wgpu::Buffer,
    dead_indexes: Vec<u32>,

    texture_binding: wgpu::BindGroup,
    vertex_buffer: ParticleVertexBuffer,
}

// TODO: does not share texture bindings
impl ParticlePipeline {
    pub fn new(
        emitter: &ParticleEmitter,
        device: &RenderDevice,
        config: &RenderConfig,
        transform: &Transform,
        texture: &Texture,
        buffer_len: usize,
    ) -> Self {
        let (texture_layout, texture_binding) =
            create_texture_bind_group(None, &device, &texture.view, &texture.sampler);

        let vertex_particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particles"),
            size: (buffer_len * std::mem::size_of::<RawParticle>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let compute_particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particles"),
            size: (buffer_len * std::mem::size_of::<RawParticle>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let alive_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("alive particles"),
            size: (buffer_len * 4) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let alive_indexes = Vec::with_capacity(buffer_len);

        // let dead_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        //     label: Some("dead particles"),
        //     size: (buffer_len * 4) as u64,
        //     usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        //     mapped_at_creation: false,
        // });
        let dead_indexes = Vec::with_capacity(buffer_len);

        let binding = 0;
        let (vertex_particle_layout, vertex_particle_binding) = create_buffer_bind_group(
            None,
            &device,
            &vertex_particle_buffer,
            wgpu::BufferBindingType::Storage { read_only: true },
            wgpu::ShaderStages::VERTEX,
            binding,
        );

        let binding = 0;
        let (compute_particle_layout, compute_particle_binding) = create_buffer_bind_group(
            None,
            &device,
            &compute_particle_buffer,
            wgpu::BufferBindingType::Storage { read_only: false },
            wgpu::ShaderStages::COMPUTE,
            binding,
        );

        let vertex_emitter_uniform = VertexEmitterUniform {
            particle_transform: emitter.particle_transformation_matrix(&config, transform),
        };
        let vertex_emitter_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particles"),
            contents: bytemuck::cast_slice(&[vertex_emitter_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let binding = 0;
        let vertex_emitter_layout =
            new_transform_bind_group_layout(&device, binding, wgpu::ShaderStages::VERTEX);
        let vertex_emitter_bind_group = new_transform_bind_group(
            &device,
            &vertex_emitter_layout,
            &vertex_emitter_buffer,
            binding,
        );
        let render_pipeline = create_particle_render_pipeline(
            &device,
            &config,
            &[
                &texture_layout,
                &vertex_particle_layout,
                &vertex_emitter_layout,
            ],
        );

        let compute_emitter_uniform = ComputeEmitterUniform::new(&config, &emitter);
        let compute_emitter_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particles"),
            contents: bytemuck::cast_slice(&[compute_emitter_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let binding = 0;
        let compute_emitter_layout =
            new_transform_bind_group_layout(&device, binding, wgpu::ShaderStages::COMPUTE);
        let compute_emitter_bind_group = new_transform_bind_group(
            &device,
            &compute_emitter_layout,
            &compute_emitter_buffer,
            binding,
        );
        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("particle compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/particles_compute.wgsl").into()),
        };
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle compute"),
            bind_group_layouts: &[&compute_particle_layout, &compute_emitter_layout],
            push_constant_ranges: &[],
        });
        let compute_pipeline =
            create_compute_pipeline("particle compute", &device, &layout, shader, "main");

        let vertex_buffer = ParticleVertexBuffer::new(&texture, &device, &config);

        Self {
            render_pipeline,
            compute_pipeline,

            vertex_emitter_uniform,
            vertex_emitter_buffer,
            vertex_emitter_bind_group,
            vertex_particle_binding,
            vertex_particle_buffer,

            compute_emitter_uniform,
            compute_emitter_buffer,
            compute_emitter_bind_group,
            compute_particle_buffer,
            compute_particle_binding,

            alive_index_buffer,
            alive_indexes,
            // dead_index_buffer,
            dead_indexes,

            texture_binding,
            vertex_buffer,
        }
    }
}

fn bind_new_particle_bundles(
    mut commands: Commands,
    device: Res<RenderDevice>,
    config: Res<RenderConfig>,
    queue: Res<RenderQueue>,
    bundles: Query<
        (Entity, Handle<Texture>, Transform, ParticleEmitter),
        Without<ParticlePipeline>,
    >,
    textures: Res<Assets<Texture>>,
    delta: Res<DeltaTime>,
) {
    for (entity, handle, transform, emitter) in bundles.iter() {
        if let Some(texture) = textures.get(handle) {
            let mut particle_render_pipeline = ParticlePipeline::new(
                emitter,
                &device,
                &config,
                &transform,
                &texture,
                emitter.num_particles,
            );
            init_emitter(
                emitter,
                &mut particle_render_pipeline,
                &queue,
                &delta,
                &config,
            );

            commands.get_entity(entity).insert(particle_render_pipeline);
        }
    }
}

#[derive(WinnyBundle)]
pub struct ParticleBundle {
    pub emitter: ParticleEmitter,
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
    ) -> Matrix4x4f {
        let scale = scale_matrix4x4f(self.particle_scale);
        let t_scale = scale_matrix4x4f(emitter_transform.scale);
        let rotation = rotation_2d_matrix4x4f(self.particle_rotation);
        let t_rotation = Matrix4::from(emitter_transform.rotation);
        let t_rotation = Matrix4x4f {
            m: t_rotation.into(),
        };

        let world_to_screen_space =
            world_to_screen_space_matrix4x4f(config.width(), config.height(), config.max_z);
        let t_translation = translation_matrix4x4f(
            world_to_screen_space * Vec4f::to_homogenous(emitter_transform.translation),
        );

        // Apply entity's Transform first, then apply local transformations. Allows for rotation
        // about the translation of the Transform.
        t_translation * t_scale * t_rotation * scale * rotation
    }
}

/// Defines the ParticleInstance stored within the GPU particle buffer. The acceleration and
/// velocity are in world space.
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

fn init_emitter(
    emitter: &ParticleEmitter,
    pipeline: &mut ParticlePipeline,
    queue: &RenderQueue,
    delta: &DeltaTime,
    config: &RenderConfig,
) {
    let particles = generate_particles_with_conditions(emitter, delta, config);
    queue.write_buffer(
        &pipeline.compute_particle_buffer,
        0,
        bytemuck::cast_slice(&particles),
    );

    pipeline.alive_indexes = (0..emitter.num_particles as u32).collect();
    queue.write_buffer(
        &pipeline.alive_index_buffer,
        0,
        bytemuck::cast_slice(&pipeline.alive_indexes),
    );
}

fn generate_particles_with_conditions(
    emitter: &ParticleEmitter,
    delta: &DeltaTime,
    config: &RenderConfig,
) -> Vec<RawParticle> {
    let mut rng = rand::thread_rng();
    let mut particles = Vec::with_capacity(emitter.num_particles);
    let world_to_screen_space =
        world_to_screen_space_matrix4x4f(config.width(), config.height(), 1000.);
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

fn create_particle_render_pipeline(
    device: &RenderDevice,
    config: &RenderConfig,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("particles"),
        bind_group_layouts,
        push_constant_ranges: &[],
    });

    let shader = wgpu::ShaderModuleDescriptor {
        label: Some("particles"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/particles.wgsl").into()),
    };

    create_render_pipeline(
        "particles",
        &device,
        &layout,
        config.format,
        None,
        &[VertexUv::layout(), alive_index_layout()],
        shader,
        true,
    )
}

fn alive_index_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<u32>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[wgpu::VertexAttribute {
            offset: 0,
            shader_location: 2,
            format: wgpu::VertexFormat::Uint32,
        }],
    }
}

#[derive(WinnyResource)]
struct ParticleVertexBuffer {
    buffer: wgpu::Buffer,
}

// TODO: reduce vertices to 3, then draw_indexed to greatly limit vertex count
impl ParticleVertexBuffer {
    // scales a particle to it's original physical size
    pub fn new(texture: &Texture, device: &RenderDevice, config: &RenderConfig) -> Self {
        let mut vertices = FULLSCREEN_QUAD_VERTEX_UV.to_vec();
        let normalized_scale = Vec2f::new(
            texture.tex.width() as f32 / config.width() as f32,
            texture.tex.height() as f32 / config.height() as f32,
        );
        let image_scale = scale_matrix4x4f(normalized_scale);
        for vert in vertices.iter_mut() {
            vert.position = image_scale * vert.position;
        }

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particle vertices"),
            usage: wgpu::BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(&vertices),
        });

        Self { buffer }
    }
}

fn update_emitter_transforms(
    mut emitters: Query<(Mut<ParticlePipeline>, Transform, ParticleEmitter)>,
    queue: Res<RenderQueue>,
    dt: Res<DeltaTime>,
    config: Res<RenderConfig>,
) {
    for (pipeline, transform, emitter) in emitters.iter_mut() {
        pipeline.vertex_emitter_uniform.particle_transform =
            emitter.particle_transformation_matrix(&config, transform);

        queue.write_buffer(
            &pipeline.vertex_emitter_buffer,
            0,
            bytemuck::cast_slice(&[pipeline.vertex_emitter_uniform]),
        );

        pipeline.compute_emitter_uniform = ComputeEmitterUniform {
            initial_velocity: Vec4f::to_homogenous(emitter.initial_velocity),
            acceleration: Vec4f::to_homogenous(emitter.acceleration),
            time_delta: dt.delta,
            time_elapsed: dt.wrapping_elapsed_as_seconds(),
            width: emitter.width / config.width() * transform.scale.v[0],
            height: emitter.height / config.height() * transform.scale.v[1],
            min_lifetime: emitter.lifetime.start,
            max_lifetime: emitter.lifetime.end,
            screen_width: config.width(),
            screen_height: config.height(),
        };

        queue.write_buffer(
            &pipeline.compute_emitter_buffer,
            0,
            bytemuck::cast_slice(&[pipeline.compute_emitter_uniform]),
        );
    }
}

fn compute_emitters(
    mut encoder: ResMut<RenderEncoder>,
    emitters: Query<(ParticlePipeline, ParticleEmitter), With<Transform>>,
) {
    for (pipeline, emitter) in emitters.iter() {
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("particle compute"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&pipeline.compute_pipeline);
            compute_pass.set_bind_group(0, &pipeline.compute_particle_binding, &[]);
            compute_pass.set_bind_group(1, &pipeline.compute_emitter_bind_group, &[]);
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

fn render_emitters(
    mut encoder: ResMut<RenderEncoder>,
    view: Res<RenderView>,
    emitters: Query<(ParticlePipeline, ParticleEmitter), With<Transform>>,
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
        render_pass.set_vertex_buffer(0, pipeline.vertex_buffer.buffer.slice(..));
        render_pass.set_vertex_buffer(1, pipeline.alive_index_buffer.slice(..));
        render_pass.set_bind_group(0, &pipeline.texture_binding, &[]);
        render_pass.set_bind_group(1, &pipeline.vertex_particle_binding, &[]);
        render_pass.set_bind_group(2, &pipeline.vertex_emitter_bind_group, &[]);
        render_pass.draw(0..6, 0..pipeline.alive_indexes.len() as u32);
    }
}
