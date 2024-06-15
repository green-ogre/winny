use std::{collections::HashMap, path::PathBuf};

use app::{app::App, plugins::Plugin};
use asset::{Asset, AssetApp, AssetId, Assets, Handle};
use ecs::{Mut, Query, Res, ResMut, SparseSet, WinnyResource, With};

use wgpu::{util::DeviceExt, BindGroupLayout, SurfaceTargetUnsafe};
use winit::window::Window;

use crate::{sprite::*, Vertex};

const VERTICES: u32 = 3;

pub struct RendererPlugin {
    renderer: Option<Renderer>,
}

impl RendererPlugin {
    pub fn new(window: Window, dimensions: (u32, u32), virutal_dimensions: (u32, u32)) -> Self {
        RendererPlugin {
            renderer: Some(pollster::block_on(Renderer::new(
                window,
                [dimensions.0, dimensions.1],
                [virutal_dimensions.0, virutal_dimensions.1],
            ))),
        }
    }
}

impl Plugin for RendererPlugin {
    fn build(&mut self, app: &mut App) {
        let renderer_context = RendererContext::default();
        let renderer = self.renderer.take().unwrap();

        app.insert_resource(renderer)
            .insert_resource(renderer_context);

        let loader = SpriteAssetLoader {};
        app.register_asset_loader::<SpriteData>(loader);
    }
}

#[derive(Debug, Default, WinnyResource)]
pub struct RendererContext {
    pub view: Option<wgpu::TextureView>,
    pub encoder: Option<wgpu::CommandEncoder>,
    pub output: Option<wgpu::SurfaceTexture>,
}

impl RendererContext {
    pub fn destroy(&mut self) {
        self.view = None;
        self.encoder = None;
        self.output = None;
    }
}

#[derive(Debug, WinnyResource)]
pub struct Renderer {
    pub window: Window,
    pub sprite_bindings: SparseSet<AssetId, SpriteBinding>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: [u32; 2],
    pub virtual_size: [u32; 2],
    render_pipeline: wgpu::RenderPipeline,
    surface: wgpu::Surface<'static>,
    vertex_buffer: wgpu::Buffer,
    sprite_buffer: wgpu::Buffer,
    num_sprites: u32,
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

        let render_pipeline =
            create_sprite_render_pipeline(&device, &config, &[&sprite_bind_group_layout]);

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
            sprite_bindings: SparseSet::new(),
            sprite_buffer,
            surface,
            device,
            queue,
            config,
            size,
            virtual_size,
            render_pipeline,
            vertex_buffer,
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

pub fn create_context(renderer: Res<Renderer>, mut renderer_context: ResMut<RendererContext>) {
    let output = renderer.surface.get_current_texture().unwrap();
    let view = output
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    let encoder = renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

    renderer_context.encoder = Some(encoder);
    renderer_context.view = Some(view);
    renderer_context.output = Some(output);
}

pub fn render(
    renderer: Res<Renderer>,
    mut renderer_context: ResMut<RendererContext>,
    handles: Query<Handle<Sprite>, With<Sprite>>,
) {
    let mut encoder = renderer_context.encoder.take().unwrap();
    let view = renderer_context.view.take().unwrap();
    let output = renderer_context.output.take().unwrap();

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
    for handle in handles.iter() {
        render_pass.set_pipeline(&renderer.render_pipeline);
        render_pass.set_bind_group(
            0,
            &renderer
                .sprite_bindings
                .get(&handle.id())
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

    renderer.queue.submit(std::iter::once(encoder.finish()));
    output.present();
}

pub fn update_sprite_data(
    sprites: Query<(Sprite, Handle<SpriteData>)>,
    assets: ResMut<Assets<SpriteData>>,
    mut renderer: ResMut<Renderer>,
) {
    // TODO: Event system to make this do no unnecessary checks
    for (_, handle) in sprites.iter_mut() {
        if !renderer.sprite_bindings.contains_key(&handle.id()) {
            if let Some(loaded_sprite) = assets.get(&handle) {
                let binding = SpriteBinding::from_data(&loaded_sprite.asset, &renderer);
                renderer.sprite_bindings.insert(handle.id(), binding);
            }
        }
    }

    let sprite_data = sprites
        .iter()
        .map(|(s, _)| s.to_raw(&renderer))
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
        .map(|(s, _)| s.to_vertices())
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
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite_shader.wgsl").into()),
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
