use std::{collections::HashMap, path::PathBuf};

use ecs::{Mut, Query, Res, ResMut, WinnyResource};

use wgpu::{util::DeviceExt, BindGroupLayout, SurfaceTargetUnsafe};
use winit::window::Window;

use crate::{camera::CameraUniform, gui::EguiRenderer, sprite::*, Vertex};

const VERTICES: u32 = 3;

#[derive(Debug, WinnyResource)]
pub struct Renderer {
    pub window: Window,
    pub sprite_bindings: HashMap<PathBuf, SpriteBindingRaw>,
    render_pipeline: wgpu::RenderPipeline,
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: [u32; 2],
    pub virtual_size: [u32; 2],
    vertex_buffer: wgpu::Buffer,
    sprite_buffer: wgpu::Buffer,
    num_sprites: u32,
    camera_bind_group: wgpu::BindGroup,
}

impl Renderer {
    pub async fn new(window: Window, size: [u32; 2], virtual_size: [u32; 2]) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = unsafe {
            instance
                .create_surface_unsafe(SurfaceTargetUnsafe::from_window(&window).unwrap())
                .unwrap()
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            desired_maximum_frame_latency: 3,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size[0],
            height: size[1],
            present_mode: surface_caps.present_modes[1],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let camera_uniform = CameraUniform::new();

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

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

        let render_pipeline = create_sprite_render_pipeline(
            &device,
            &config,
            &[&camera_bind_group_layout, &sprite_bind_group_layout],
        );

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite vertex buffer"),
            contents: &[],
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let sprite_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Buffer"),
            contents: bytemuck::cast_slice::<SpriteInstance, u8>(&[]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Renderer {
            window,
            num_sprites: 0,
            sprite_bindings: HashMap::new(),
            sprite_buffer,
            surface,
            device,
            queue,
            config,
            size,
            virtual_size,
            render_pipeline,
            vertex_buffer,
            camera_bind_group,
        }
    }

    pub fn resize(&mut self, new_size: [u32; 2]) {
        if new_size[0] > 0 && new_size[1] > 0 {
            self.size = new_size;
            self.config.width = new_size[0];
            self.config.height = new_size[1];
            self.surface.configure(&self.device, &self.config);
        }
    }
}

pub fn render(
    renderer: Res<Renderer>,
    sprites: Query<SpriteBinding>,
    mut egui_renderer: ResMut<EguiRenderer>,
) {
    let output = renderer.surface.get_current_texture().unwrap();
    let mut view = output
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
    });

    let mut offset = 0;
    for sprite in sprites.iter() {
        render_pass.set_pipeline(&renderer.render_pipeline);
        render_pass.set_bind_group(0, &renderer.camera_bind_group, &[]);
        render_pass.set_bind_group(
            1,
            &renderer
                .sprite_bindings
                .get(&sprite.path)
                .expect("sprite binding added before render pass")
                .bind_group,
            &[],
        );
        render_pass.set_vertex_buffer(0, renderer.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, renderer.sprite_buffer.slice(..));
        render_pass.draw(
            offset * VERTICES..offset * VERTICES + VERTICES,
            offset..offset + 1,
        );
        offset += 1;
    }
    drop(render_pass);

    egui_renderer.end_frame(
        &renderer.device,
        &renderer.queue,
        &mut encoder,
        &renderer.window,
        &mut view,
        egui_wgpu::ScreenDescriptor {
            size_in_pixels: [
                renderer.window.inner_size().width,
                renderer.window.inner_size().height,
            ],
            pixels_per_point: renderer.window.scale_factor() as f32,
        },
    );

    renderer.queue.submit(std::iter::once(encoder.finish()));
    output.present();
}

pub fn update_sprite_data(
    sprites: Query<Sprite>,
    bindings: Query<Mut<SpriteBinding>>,
    mut renderer: ResMut<Renderer>,
) {
    for b in bindings.iter_mut() {
        if !renderer.sprite_bindings.contains_key(&b.path) {
            let bg = SpriteBindingRaw::initialize(&b.path, &renderer).unwrap();
            renderer.sprite_bindings.insert(b.path.clone(), bg);
        }
    }

    let sprite_data = sprites
        .iter()
        .map(|s| s.to_raw(&renderer))
        .collect::<Vec<_>>();

    renderer.sprite_buffer =
        renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sprite Buffer"),
                contents: bytemuck::cast_slice(&sprite_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

    let vertex_data: Vec<_> = sprites
        .iter()
        .map(|sprite| sprite.to_vertices())
        .flatten()
        .collect();

    renderer.num_sprites = vertex_data.len() as u32 / VERTICES;

    renderer.vertex_buffer =
        renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sprite vertex buffer"),
                contents: bytemuck::cast_slice(&vertex_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
}

fn create_sprite_render_pipeline(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    bind_group_layouts: &[&BindGroupLayout],
) -> wgpu::RenderPipeline {
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts,
        push_constant_ranges: &[],
    });

    let shader = wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("sprite_shader.wgsl").into()),
    };

    create_render_pipeline(
        &device,
        &render_pipeline_layout,
        config.format,
        // Some(wgpu::TextureFormat::Depth32Float),
        &[SpriteVertex::desc(), SpriteInstance::desc()],
        shader,
    )
}

fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    // depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModuleDescriptor,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(shader);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: vertex_layouts,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState {
                    alpha: wgpu::BlendComponent::OVER,
                    color: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Cw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: true,
        },
        multiview: None,
    })
}
