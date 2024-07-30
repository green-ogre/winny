use std::ops::Range;

use app::{
    app::{AppSchedule, Schedule},
    plugins::Plugin,
    time::DeltaTime,
    window::Window,
};
use asset::{Assets, Handle};
use ecs::{prelude::*, WinnyBundle, WinnyComponent, WinnyResource};
use rand::Rng;
use render::prelude::*;
use wgpu::util::DeviceExt;
use winny_math::{
    angle::Radf,
    matrix::{
        rotation_2d_matrix4x4f, scale_matrix4x4f, world_to_screen_space_matrix4x4f, Matrix4x4f,
    },
    vector::{Vec2f, Vec3f, Vec4f},
};

use crate::{
    create_buffer_bind_group, create_compute_pipeline, create_render_pipeline,
    create_texture_bind_group,
    noise::{NoisePlugin, NoiseTexture},
    texture::Texture,
    transform::{new_transform_bind_group, new_transform_bind_group_layout, Transform},
    vertex::{VertexLayout, VertexUv, FULLSCREEN_QUAD_VERTEX_UV},
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
    emitter_transform: Matrix4x4f,
    particle_transform: Matrix4x4f,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeEmitterUniform {
    // Seconds
    time_delta: f32,
    time_elapsed: f32,
    width: f32,
    height: f32,
    max_lifetime: f32,
    min_lifetime: f32,
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
    noise_texture_binding: wgpu::BindGroup,
    vertex_buffer: ParticleVertexBuffer,
}

// TODO: does not share texture bindings
impl ParticlePipeline {
    pub fn new(
        emitter: &ParticleEmitter,
        device: &RenderDevice,
        config: &RenderConfig,
        transform: &Transform,
        window: &Window,
        texture: &Texture,
        buffer_len: usize,
        noise: &Texture,
    ) -> Self {
        let noise_texture_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            });

        let noise_texture_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &noise_texture_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&noise.view),
            }],
        });

        let (texture_layout, texture_binding) =
            create_texture_bind_group(None, &device, &texture.view, &texture.sampler);

        let vertex_particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particles"),
            size: (buffer_len * std::mem::size_of::<Particle>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let compute_particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particles"),
            size: (buffer_len * std::mem::size_of::<Particle>()) as u64,
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
            emitter_transform: transform.transformation_matrix(&window.viewport, 1000.),
            particle_transform: emitter.particle_transformation_matrix(),
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

        let compute_emitter_uniform = ComputeEmitterUniform {
            time_delta: 0.,
            time_elapsed: 0.,
            width: emitter.width,
            height: emitter.height,
            min_lifetime: emitter.lifetime.start,
            max_lifetime: emitter.lifetime.end,
        };
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
            bind_group_layouts: &[
                &compute_particle_layout,
                &compute_emitter_layout,
                &noise_texture_layout,
            ],
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
            noise_texture_binding,
            vertex_buffer,
        }
    }
}

fn bind_new_particle_bundles(
    mut commands: Commands,
    device: Res<RenderDevice>,
    config: Res<RenderConfig>,
    queue: Res<RenderQueue>,
    window: Res<Window>,
    bundles: Query<
        (Entity, Handle<Texture>, Transform, ParticleEmitter),
        Without<ParticlePipeline>,
    >,
    textures: Res<Assets<Texture>>,
    noise: Res<NoiseTexture>,
) {
    for (entity, handle, transform, emitter) in bundles.iter() {
        if let Some(texture) = textures.get(handle) {
            // TODO: dont make new binding
            if let Some(noise) = textures.get(&noise.0) {
                let mut particle_render_pipeline = ParticlePipeline::new(
                    emitter,
                    &device,
                    &config,
                    &transform,
                    &window,
                    &texture,
                    emitter.num_particles,
                    &noise.asset,
                );
                init_emitter(emitter, &mut particle_render_pipeline, &queue, &window);

                commands.get_entity(entity).insert(particle_render_pipeline);
            }
        }
    }
}

#[derive(WinnyBundle)]
pub struct ParticleBundle {
    pub emitter: ParticleEmitter,
    pub handle: Handle<Texture>,
    pub transform: Transform,
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
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            is_emitting: true,
            num_particles: 10,
            lifetime: 0.5..1.0,
            width: 400.,
            height: 400.,
            particle_scale: Vec2f::new(1., 1.),
            particle_rotation: Radf(0.),
        }
    }
}

impl ParticleEmitter {
    pub(crate) fn particle_transformation_matrix(&self) -> Matrix4x4f {
        let scale = scale_matrix4x4f(self.particle_scale);
        let rotation = rotation_2d_matrix4x4f(self.particle_rotation);

        scale * rotation
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Particle {
    // Within screen space
    translation: Vec4f,
    scale: Vec2f,
    // Radians
    rotation: f32,
    // Seconds
    lifetime: f32,
}

impl Particle {
    pub fn new(translation: Vec4f, scale: Vec2f, rotation: f32, lifetime: f32) -> Self {
        Self {
            translation,
            scale,
            rotation,
            lifetime,
        }
    }
}

fn init_emitter(
    emitter: &ParticleEmitter,
    pipeline: &mut ParticlePipeline,
    queue: &RenderQueue,
    window: &Window,
) {
    let particles = generate_particles_with_conditions(emitter, window);
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

fn generate_particles_with_conditions(emitter: &ParticleEmitter, window: &Window) -> Vec<Particle> {
    let mut rng = rand::thread_rng();
    let mut particles = Vec::with_capacity(emitter.num_particles);
    let size = window.winit_window.inner_size();
    let world_to_screen =
        world_to_screen_space_matrix4x4f(size.width as f32, size.height as f32, 1000.);
    for _ in 0..emitter.num_particles {
        let x = rng.gen_range(-emitter.width..emitter.width);
        let y = rng.gen_range(-emitter.height..emitter.height);
        let lifetime = rng.gen_range(emitter.lifetime.clone());
        particles.push(Particle::new(
            world_to_screen * Vec4f::to_homogenous(Vec3f::new(x, y, 0.)),
            Vec2f::new(1., 1.),
            0.,
            lifetime,
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
    window: Res<Window>,
    // TODO: assumes that render happens during every update loop
    dt: Res<DeltaTime>,
    config: Res<RenderConfig>,
) {
    for (pipeline, transform, emitter) in emitters.iter_mut() {
        pipeline.vertex_emitter_uniform.emitter_transform =
            transform.transformation_matrix(&window.viewport, 1000.);
        pipeline.vertex_emitter_uniform.particle_transform =
            emitter.particle_transformation_matrix();

        queue.write_buffer(
            &pipeline.vertex_emitter_buffer,
            0,
            bytemuck::cast_slice(&[pipeline.vertex_emitter_uniform]),
        );

        pipeline.compute_emitter_uniform = ComputeEmitterUniform {
            time_delta: dt.delta,
            time_elapsed: dt.wrapping_elapsed_as_seconds(),
            // TODO: the radius needs
            width: emitter.width / config.width(),
            height: emitter.height / config.height(),
            min_lifetime: emitter.lifetime.start,
            max_lifetime: emitter.lifetime.end,
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
            compute_pass.set_bind_group(2, &pipeline.noise_texture_binding, &[]);
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
    if !emitters.iter().any(|(_, e)| e.is_emitting) {
        return;
    }

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
