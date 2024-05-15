use core::{panic, time};
use std::{
    env::{self, current_dir},
    error::Error,
    f64::NAN,
    ffi::OsString,
    io::Read,
    marker::PhantomData,
    num::NonZeroU32,
    sync::mpsc::{channel, Receiver, Sender},
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ecs::{Event, EventWriter, Res, ResMut, Resource, Scheduler, TypeGetter, World};
use egui_winit::EventResponse;
use gilrs::{EventType, Gilrs};
use image::GenericImageView;
use wgpu::{
    rwh::HasDisplayHandle,
    util::{DeviceExt, RenderEncoder},
    SurfaceTargetUnsafe,
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceEvent, ElementState, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowBuilder},
};

use logger::*;

use crate::{
    prelude::{
        load_model, load_texture,
        texture::{DepthTexture, DiffuseTexture, NormalTexture},
        Boid, BoidRaw, BoidState, BoidVertex, Camera, CameraController, DrawLight, DrawModel,
        FullscreenQuad, Instance, InstanceRaw, Material, Model, ModelVertex, PointLightUniform,
        Projection, Vertex, NUM_INSTANCES_PER_ROW,
    },
    App,
};

impl<'w> State<'w> {
    async fn new(window: Window) -> Self {
        // START

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

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            desired_maximum_frame_latency: 3,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[1],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        info!("Surface Config: {:#?}", config);
        surface.configure(&device, &config);

        // END

        let boid_sprite_bind_group_layout =
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
                label: Some("boid_sprite_bind_group_layout"),
            });

        let boid_color_bind_group_layout =
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
                label: Some("boid_color_bind_group_layout"),
            });

        let boid_rot_color_bind_group_layout =
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
                label: Some("boid_rot_color_bind_group_layout"),
            });

        // let tileset_bind_group_layout =
        //     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //         entries: &[
        //             wgpu::BindGroupLayoutEntry {
        //                 binding: 0,
        //                 visibility: wgpu::ShaderStages::FRAGMENT,
        //                 ty: wgpu::BindingType::Texture {
        //                     multisampled: false,
        //                     view_dimension: wgpu::TextureViewDimension::D2,
        //                     sample_type: wgpu::TextureSampleType::Float { filterable: true },
        //                 },
        //                 count: None,
        //             },
        //             wgpu::BindGroupLayoutEntry {
        //                 binding: 1,
        //                 visibility: wgpu::ShaderStages::FRAGMENT,
        //                 ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        //                 count: None,
        //             },
        //         ],
        //         label: Some("texture_bind_group_layout"),
        //     });

        // let tileset = load_texture("Anno_16x16.png".into(), &device, &queue)
        //     .await
        //     .unwrap();

        // let tileset_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     layout: &tileset_bind_group_layout,
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: wgpu::BindingResource::TextureView(&tileset.view),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 1,
        //             resource: wgpu::BindingResource::Sampler(&tileset.sampler),
        //         },
        //     ],
        //     label: None,
        // });

        // Camera

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

        let boid_sprite = load_texture("boid.png".into(), &device, &queue)
            .await
            .unwrap();

        let boid_sprite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &boid_sprite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&boid_sprite.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&boid_sprite.sampler),
                },
            ],
            label: None,
        });

        let boid_color = load_texture("boid_color.png".into(), &device, &queue)
            .await
            .unwrap();

        let boid_color_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &boid_color_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&boid_color.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&boid_color.sampler),
                },
            ],
            label: None,
        });

        let boid_rot_color = load_texture("boid_rot_color.png".into(), &device, &queue)
            .await
            .unwrap();

        let boid_rot_color_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &boid_color_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&boid_rot_color.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&boid_rot_color.sampler),
                },
            ],
            label: None,
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &boid_sprite_bind_group_layout,
                    // &tileset_bind_group_layout,
                    &camera_bind_group_layout,
                    &boid_color_bind_group_layout,
                    &boid_rot_color_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = {
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("gfx/shader2.wgsl").into()),
            };
            create_render_pipeline(
                &device,
                &render_pipeline_layout,
                config.format,
                // Some(wgpu::TextureFormat::Depth32Float),
                None,
                &[BoidVertex::desc(), BoidRaw::desc()],
                shader,
            )
        };

        let primitive_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &boid_sprite_bind_group_layout,
                    // &tileset_bind_group_layout,
                    &camera_bind_group_layout,
                    // &light_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let primitive_render_pipeline = {
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Primitives Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("gfx/primitives.wgsl").into()),
            };
            create_render_pipeline(
                &device,
                &primitive_render_pipeline_layout,
                config.format,
                None,
                &[],
                shader,
            )
        };

        // instances

        let boids = (0..NUM_BOIDS)
            .map(|i| {
                use rand::prelude::*;

                let mut rng = thread_rng();

                let x = rng.gen_range(0.0..100.0);
                let y = rng.gen_range(0.0..100.0);

                let vel_x = rng.gen_range(0.9..1.0);
                let vel_y = rng.gen_range(0.9..1.0);

                let position = cgmath::Vector2 { x, y };
                let velocity = cgmath::Vector2 { x: vel_x, y: vel_y };

                let state = if i == 0 {
                    crate::prelude::BoidState::Selected
                } else {
                    crate::prelude::BoidState::None
                };

                Boid {
                    position,
                    velocity,
                    state,
                    num_friends: 0,
                }
            })
            .collect::<Vec<_>>();

        let boid_data = boids.iter().map(Boid::to_raw).collect::<Vec<_>>();
        let boid_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&boid_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        const BOID_SIZE: f32 = 0.004;
        const VERTICES: [BoidVertex; 3] = [
            BoidVertex {
                position: [-BOID_SIZE, -BOID_SIZE, 0.0],
                tex_coord: [0.0, 0.0],
            },
            BoidVertex {
                position: [0.0, BOID_SIZE, 0.0],
                tex_coord: [0.5, 1.0],
            },
            BoidVertex {
                position: [BOID_SIZE, -BOID_SIZE, 0.0],
                tex_coord: [1.0, 0.0],
            },
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(
                &(0..NUM_BOIDS)
                    .map(|_| VERTICES)
                    .flatten()
                    .collect::<Vec<_>>(),
            ),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let egui_renderer = EguiRenderer::new(&device, config.format, 1, &window);

        Self {
            primitive_render_pipeline,
            window,
            egui_renderer,
            camera_buffer,
            camera_uniform,
            camera_bind_group,
            // tileset,
            // tileset_bind_group,
            boid_sprite_bind_group,
            boid_color_bind_group,
            boid_rot_color_bind_group,
            render_pipeline,
            surface,
            device,
            queue,
            config,
            size,
            vertex_buffer,
            boid_buffer,
            boids,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn update(&mut self, dt: &DeltaT, boid_params: &BoidParams) {
        for i in 0..self.boids.len() {
            let mut seperation_vec = Vector2::zero();
            let mut alignment_vec = Vector2::zero();

            let mut friends = Vec::new();

            for j in 0..self.boids.len() {
                if i == j {
                    continue;
                }

                let dst_to = self.boids[i].position.distance(self.boids[j].position);
                let dst_vec = self.boids[i].position - self.boids[j].position;

                if dst_to <= boid_params.enemy_radius {
                    seperation_vec += dst_vec;
                    if i == 0 {
                        self.boids[j].state = BoidState::Enemy;
                    }
                } else if dst_to <= boid_params.friend_radius {
                    alignment_vec += self.boids[j].velocity;
                    friends.push(self.boids[j].position);
                    if i == 0 {
                        self.boids[j].state = BoidState::Friend;
                    }
                } else {
                    if i == 0 {
                        self.boids[j].state = BoidState::None;
                    }
                }
            }

            let boid = &mut self.boids[i];

            boid.num_friends = friends.len() as u32;

            let mut cohesion_vec = Vector2::zero();
            if friends.len() != 0 {
                let avg_x = friends.iter().map(|p| p.x).sum::<f32>() / friends.len() as f32;
                let avg_y = friends.iter().map(|p| p.y).sum::<f32>() / friends.len() as f32;
                cohesion_vec = Vector2::new(avg_x, avg_y) - boid.position;

                alignment_vec /= friends.len() as f32;
                alignment_vec -= boid.position;

                if !seperation_vec.is_zero() {
                    seperation_vec = seperation_vec.normalize();
                }
                if !alignment_vec.is_zero() {
                    alignment_vec = alignment_vec.normalize();
                }
                if !cohesion_vec.is_zero() {
                    cohesion_vec = cohesion_vec.normalize();
                }
            }

            boid.velocity += seperation_vec * boid_params.seperation_force
                + alignment_vec * boid_params.alignment_force
                + cohesion_vec * boid_params.cohesion_force;

            let mut speed = boid.velocity.magnitude();
            let dir = boid.velocity / speed;
            speed = speed.clamp(boid_params.min_speed, boid_params.max_speed);

            // BOUNDS
            if boid.position.x > 90.0 {
                // let strength = 100.0 - boid.position.x;
                // dir.x -= boid_params.steering_force * strength * dt.0 as f32;
                boid.position.x = 10.0;
            }
            if boid.position.y > 90.0 {
                // let strength = 100.0 - boid.position.y;
                // dir.y -= boid_params.steering_force * strength * dt.0 as f32;
                boid.position.y = 10.0;
            }

            if boid.position.x < 10.0 {
                // let strength = 10.0 - boid.position.x;
                // dir.x += boid_params.steering_force * strength * dt.0 as f32;
                boid.position.x = 90.0;
            }
            if boid.position.y < 10.0 {
                // let strength = 10.0 - boid.position.y;
                // dir.y += boid_params.steering_force * strength * dt.0 as f32;
                boid.position.y = 90.0;
            }

            boid.velocity = speed * dir;
            boid.position += boid.velocity * dt.0 as f32;
        }

        self.queue.write_buffer(
            &self.boid_buffer,
            0,
            bytemuck::cast_slice(&self.boids.iter().map(|b| b.to_raw()).collect::<Vec<_>>()),
        );
    }

    fn render(
        &mut self,
        params: &mut BoidParams,
        param_presets: &mut BoidParamPresets,
    ) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let mut view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
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

        render_pass.set_pipeline(&self.render_pipeline);
        // render_pass.set_bind_group(0, &self.tileset_bind_group, &[]);
        render_pass.set_bind_group(0, &self.boid_sprite_bind_group, &[]);
        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(2, &self.boid_color_bind_group, &[]);
        render_pass.set_bind_group(3, &self.boid_rot_color_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.boid_buffer.slice(..));
        render_pass.draw(0..NUM_BOIDS as u32 * 3, 0..NUM_BOIDS as u32);
        drop(render_pass);

        self.egui_renderer.draw(
            &self.device,
            &self.queue,
            &mut encoder,
            &self.window,
            &mut view,
            egui_wgpu::ScreenDescriptor {
                size_in_pixels: [
                    self.window.inner_size().width,
                    self.window.inner_size().height,
                ],
                pixels_per_point: self.window.scale_factor() as f32,
            },
            |ui| {
                egui::Window::new("Boid Parameters")
                    .resizable(true)
                    .open(&mut false)
                    .show(&ui, |ui| {
                        ui.add(
                            egui::Slider::new(&mut params.seperation_force, 0.0..=2.0)
                                .text("Seperation Force"),
                        );
                        ui.add(
                            egui::Slider::new(&mut params.alignment_force, 0.0..=2.0)
                                .text("Alignment Force"),
                        );
                        ui.add(
                            egui::Slider::new(&mut params.cohesion_force, 0.0..=1.0)
                                .text("Cohesion Force"),
                        );
                        ui.add(
                            egui::Slider::new(&mut params.max_speed, 0.0..=20.0).text("Max Speed"),
                        );
                        ui.add(
                            egui::Slider::new(&mut params.min_speed, 0.0..=20.0).text("Min Speed"),
                        );

                        if params.min_speed > params.max_speed {
                            params.max_speed = params.min_speed;
                        }

                        ui.add(
                            egui::Slider::new(&mut params.friend_radius, 0.0..=20.0)
                                .text("Friend Radius"),
                        );
                        ui.add(
                            egui::Slider::new(&mut params.enemy_radius, 0.0..=20.0)
                                .text("Enemy Radius"),
                        );

                        if params.enemy_radius > params.friend_radius {
                            params.friend_radius = params.enemy_radius;
                        }

                        ui.add(
                            egui::Slider::new(&mut params.steering_force, 0.0..=20.0)
                                .text("Steering Force"),
                        );

                        ui.label("Presets");
                        for i in 0..5 {
                            if i == param_presets.index {
                                ui.add_enabled(false, egui::Button::new(format!("{}", i + 1)));
                            } else {
                                if ui.add(egui::Button::new(format!("{}", i + 1))).clicked() {
                                    *params = param_presets.presets[i];
                                    param_presets.index = i;
                                }
                            }
                        }

                        if ui.add(egui::Button::new("Set")).clicked() {
                            param_presets.presets[param_presets.index] = *params;
                        }
                    });
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[derive(Debug, Resource, TypeGetter, Clone, Copy)]
pub struct BoidParams {
    pub max_speed: f32,
    pub min_speed: f32,
    pub seperation_force: f32,
    pub alignment_force: f32,
    pub cohesion_force: f32,
    pub friend_radius: f32,
    pub enemy_radius: f32,
    pub steering_force: f32,
}

impl Default for BoidParams {
    fn default() -> Self {
        Self {
            max_speed: 15.0,
            min_speed: 12.0,
            seperation_force: 0.2,
            alignment_force: 0.03,
            cohesion_force: 0.05,
            friend_radius: 6.0,
            enemy_radius: 2.0,
            steering_force: 2.0,
        }
    }
}

fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
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

#[derive(Debug, Resource, TypeGetter)]
pub struct DeltaT(pub f64);

pub async fn game_loop(mut world: World, mut scheduler: Scheduler) {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(1800, 1800))
        .with_position(PhysicalPosition::new(100, 100))
        .build(&event_loop)
        .unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut state = State::new(window).await;

    let projection = Projection::new(
        state.config.width,
        state.config.height,
        cgmath::Deg(45.0),
        0.1,
        100.0,
    );
    let camera_controller = CameraController::new(8.0, 0.4);
    let camera = Camera::new(
        (0.0, 0.0, 0.0),
        cgmath::Deg(-90.0),
        cgmath::Deg(-20.0),
        projection,
    );

    world.insert_resource(BoidParamPresets {
        presets: vec![BoidParams::default(); 5],
        index: 0,
    });

    world.insert_resource(DeltaT(0.0));
    world.insert_resource(camera_controller);
    world.insert_resource(camera);
    world.register_event::<KeyInput>();
    world.register_event::<MouseInput>();
    world.register_event::<ControllerInput>();
    world.insert_resource(ControllerAxisState::new());

    scheduler.startup(&world);

    // let target_fps = Some(60.0);
    let target_fps: Option<f64> = None;
    let target_frame_len = target_fps.map(|target| 1.0 / target);
    let mut perf = PerfCounter::new(target_frame_len);

    let (controller_input_sender, controller_input_reciever) =
        channel::<(ControllerInput, ControllerAxisState)>();

    // handle controller input
    std::thread::spawn(move || {
        let mut gilrs = Gilrs::new().unwrap();

        // Iterate over all connected gamepads
        for (_id, gamepad) in gilrs.gamepads() {
            info!("{} is {:?}", gamepad.name(), gamepad.power_info());
        }

        let mut controller_axis_state = ControllerAxisState::new();

        // Examine new events
        loop {
            while let Some(gilrs::Event { event, .. }) = gilrs.next_event() {
                let input = ControllerInputState::from(event);

                if let Some(new_axis_state) = input.axis_state() {
                    controller_axis_state.apply_new_state(new_axis_state);
                }

                if controller_input_sender
                    .send((ControllerInput::new(input), controller_axis_state))
                    .is_err()
                {
                    error!("Error sending controller input");
                }
            }
        }
    });

    let (winit_window_event_tx, winit_window_event_rx) = channel();
    let (winit_device_event_tx, winit_device_event_rx) = channel::<DeviceEvent>();
    // This is necessary because exiting the winit event_loop will exit the program, so a message
    // is sent to the event_loop when the game_loop has finished exiting
    let (winit_exit_tx, winit_exit_rx) = channel();

    // This is the main game loop
    std::thread::spawn(move || loop {
        perf.start();

        for event in winit_window_event_rx.try_iter() {
            if let Some(response) =
                state
                    .egui_renderer
                    .handle_input(&state.window, Some(&event), None)
            {
                if response.consumed {
                    continue;
                }
            }

            match event {
                WindowEvent::Resized(new_size) => {
                    state.resize(new_size);
                }
                WindowEvent::CloseRequested => {
                    exit_game(&perf, &mut world, &mut scheduler);
                    winit_exit_tx.send(()).unwrap();
                }
                WindowEvent::KeyboardInput {
                    event: key_event, ..
                } => {
                    let mut user_input = unsafe { EventWriter::new(world.as_unsafe_world()) };

                    if let PhysicalKey::Code(key_code) = key_event.physical_key {
                        user_input.send(KeyInput::new(
                            KeyCode::new(key_code),
                            match key_event.state {
                                ElementState::Pressed => KeyState::Pressed,
                                ElementState::Released => KeyState::Released,
                            },
                        ));
                    }
                }
                _ => {}
            }
        }

        for event in winit_device_event_rx.try_iter() {
            match event {
                DeviceEvent::MouseMotion { delta } => {
                    let mut user_input = unsafe { EventWriter::new(world.as_unsafe_world()) };
                    user_input.send(MouseInput::new(delta.0, delta.1));

                    state
                        .egui_renderer
                        .handle_input(&state.window, None, Some(delta));
                }
                _ => (),
            }
        }

        if !update_and_render(
            &mut perf,
            &mut world,
            &mut scheduler,
            &mut state,
            &controller_input_reciever,
        ) {
            break;
        }
    });

    // Pipe these events into the update and render thread
    let _ = event_loop.run(move |event, elwt| match event {
        winit::event::Event::WindowEvent { event, .. } => {
            winit_window_event_tx.send(event.clone()).unwrap();

            if let Ok(_) = winit_exit_rx.try_recv() {
                elwt.exit();
            }
        }
        winit::event::Event::DeviceEvent { event, .. } => {
            winit_device_event_tx.send(event).unwrap();
        }
        _ => (),
    });
}

fn update_and_render(
    perf: &mut PerfCounter,
    world: &mut World,
    scheduler: &mut Scheduler,
    state: &mut State,
    controller_input_reciever: &Receiver<(ControllerInput, ControllerAxisState)>,
) -> bool {
    // Pipe window input into World
    {
        // SAFETY:
        //
        // The EventWriter and ResMut have mutually exclusive, mutable access to different
        // memory and are dropped before the scheduler runs. Nobody can access this memory
        // at the same time.
        let mut controller_event = unsafe { EventWriter::new(world.as_unsafe_world()) };
        let mut controller_axis_state = unsafe { ResMut::new(world.as_unsafe_world()) };
        for (input, axis_state) in controller_input_reciever.try_iter() {
            // info!("Controller Input: {:#?}", input);
            controller_event.send(input);
            *controller_axis_state = axis_state;
        }
    }

    // Insert last frame time
    {
        let mut dt = unsafe { ResMut::new(world.as_unsafe_world()) };
        *dt = DeltaT(perf.last_frame_duration().unwrap_or_default().as_secs_f64());
    }

    perf.start_debug_event();

    // Update World
    scheduler.run(world);
    world.flush_events();

    perf.stop_debug_event();
    debug!(
        "Update World: {}ms",
        perf.query_last_debug_event()
            .unwrap_or_default()
            .as_millis()
    );

    // let instance_data = state
    //     .instances
    //     .iter()
    //     .map(Instance::to_raw)
    //     .collect::<Vec<_>>();

    // state.queue.write_buffer(
    //     &state.instance_buffer,
    //     0,
    //     bytemuck::cast_slice(&instance_data),
    // );

    perf.start_debug_event();

    // Render
    {
        let dt = unsafe { Res::new(world.as_unsafe_world()) };
        let boid_params = unsafe { Res::<BoidParams>::new(world.as_unsafe_world()) };
        state.update(&dt, boid_params.as_ref());
    }

    {
        let mut params = unsafe { ResMut::new(world.as_unsafe_world()) };
        let mut param_presets = unsafe { ResMut::<BoidParamPresets>::new(world.as_unsafe_world()) };
        match state.render(&mut params, &mut param_presets) {
            Ok(_) => {}
            // Reconfigure the surface if lost
            Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
            // The system is out of memory, we should probably quit
            Err(wgpu::SurfaceError::OutOfMemory) => {
                exit_game(perf, world, scheduler);
                return false;
            }
            // All other errors (Outdated, Timeout) should be resolved by the next frame
            Err(e) => error!("{:?}", e),
        }
    }

    perf.stop_debug_event();
    debug!(
        "Render: {}ms",
        perf.query_last_debug_event()
            .unwrap_or_default()
            .as_millis()
    );

    while !perf.should_advance() {}
    perf.stop();

    true
}

pub fn exit_game(perf: &PerfCounter, world: &World, scheduler: &mut Scheduler) {
    scheduler.exit(world);
    perf.exit_stats();
}

fn read_file(path: &String) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut f = std::fs::File::open(path)?;
    let metadata = f.metadata()?;
    let mut buf = vec![0u8; metadata.len() as usize];
    f.read(&mut buf)?;

    Ok(buf)
}

#[derive(Debug, Resource, TypeGetter)]
pub struct BoidParamPresets {
    presets: Vec<BoidParams>,
    index: usize,
}

pub struct InputBuffer {
    buf: [Option<KeyInput>; 10],
    index: u8,
    len: u8,
}

impl Default for InputBuffer {
    fn default() -> Self {
        InputBuffer {
            buf: [None; 10],
            index: 0,
            len: 10,
        }
    }
}

impl InputBuffer {
    pub fn push(&mut self, e: KeyInput) {
        if self.index < self.len - 1 {
            self.buf[(self.index + 1) as usize] = Some(e);
            self.index += 1;
        } else {
            panic!("Need a bigger input buffer");
        }
    }

    pub fn pop(&mut self) -> Option<KeyInput> {
        let val = std::mem::replace(&mut self.buf[self.index as usize], None);
        self.index = self.index.saturating_sub(1);

        val
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum KeyCode {
    Unknown,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Space,
    Shift,
}

impl KeyCode {
    pub fn new(code: winit::keyboard::KeyCode) -> Self {
        match code {
            winit::keyboard::KeyCode::KeyA => KeyCode::A,
            winit::keyboard::KeyCode::KeyB => KeyCode::B,
            winit::keyboard::KeyCode::KeyC => KeyCode::C,
            winit::keyboard::KeyCode::KeyD => KeyCode::D,
            winit::keyboard::KeyCode::KeyE => KeyCode::E,
            winit::keyboard::KeyCode::KeyF => KeyCode::F,
            winit::keyboard::KeyCode::KeyG => KeyCode::G,
            winit::keyboard::KeyCode::KeyH => KeyCode::H,
            winit::keyboard::KeyCode::KeyI => KeyCode::I,
            winit::keyboard::KeyCode::KeyJ => KeyCode::J,
            winit::keyboard::KeyCode::KeyK => KeyCode::K,
            winit::keyboard::KeyCode::KeyL => KeyCode::L,
            winit::keyboard::KeyCode::KeyM => KeyCode::M,
            winit::keyboard::KeyCode::KeyN => KeyCode::N,
            winit::keyboard::KeyCode::KeyO => KeyCode::O,
            winit::keyboard::KeyCode::KeyP => KeyCode::P,
            winit::keyboard::KeyCode::KeyQ => KeyCode::Q,
            winit::keyboard::KeyCode::KeyR => KeyCode::R,
            winit::keyboard::KeyCode::KeyS => KeyCode::S,
            winit::keyboard::KeyCode::KeyT => KeyCode::T,
            winit::keyboard::KeyCode::KeyU => KeyCode::U,
            winit::keyboard::KeyCode::KeyV => KeyCode::V,
            winit::keyboard::KeyCode::KeyW => KeyCode::W,
            winit::keyboard::KeyCode::KeyX => KeyCode::X,
            winit::keyboard::KeyCode::KeyY => KeyCode::Y,
            winit::keyboard::KeyCode::KeyZ => KeyCode::Z,
            winit::keyboard::KeyCode::Space => KeyCode::Space,
            winit::keyboard::KeyCode::ShiftLeft => KeyCode::Shift,
            winit::keyboard::KeyCode::ShiftRight => KeyCode::Shift,
            _ => KeyCode::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, Event, TypeGetter)]
pub struct KeyInput {
    pub code: KeyCode,
    pub state: KeyState,
}

impl KeyInput {
    pub fn new(code: KeyCode, state: KeyState) -> Self {
        Self { code, state }
    }
}

#[derive(Debug, Clone, Copy, Event, TypeGetter)]
pub struct MouseInput {
    pub dx: f64,
    pub dy: f64,
}

impl MouseInput {
    pub fn new(dx: f64, dy: f64) -> Self {
        Self { dx, dy }
    }
}

#[derive(Debug)]
pub enum ControllerInputState {
    ButtonPressed(Button),
    ButtonRepeated(Button),
    ButtonReleased(Button),
    AxisChanged(Axis, f32),
    Connected,
    Disconnected,
    Unknown,
}

#[derive(Debug, Event, TypeGetter)]
pub struct ControllerInput {
    pub input: ControllerInputState,
}

impl ControllerInput {
    pub fn new(input: ControllerInputState) -> Self {
        Self { input }
    }
}

impl ControllerInputState {
    pub fn from(value: EventType) -> Self {
        match value {
            EventType::ButtonPressed(b, _) => ControllerInputState::ButtonPressed(Button::from(b)),
            EventType::ButtonRepeated(b, _) => {
                ControllerInputState::ButtonRepeated(Button::from(b))
            }
            EventType::ButtonReleased(b, _) => {
                ControllerInputState::ButtonReleased(Button::from(b))
            }
            EventType::AxisChanged(a, f, _) => ControllerInputState::AxisChanged(Axis::from(a), f),
            EventType::Connected => ControllerInputState::Connected,
            EventType::Disconnected => ControllerInputState::Disconnected,
            _ => ControllerInputState::Unknown,
        }
    }

    pub fn axis_state(&self) -> Option<AxisState> {
        match self {
            Self::AxisChanged(axis, f) => Some(AxisState::new(*axis, *f)),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Button {
    A,
    X,
    Y,
    B,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    LeftStick,
    RigthStick,
    LeftTrigger,
    LeftBumper,
    RightTrigger,
    RightBumper,
    Unknown,
}

impl Button {
    pub fn from(value: gilrs::Button) -> Self {
        match value {
            gilrs::Button::DPadUp => Button::DPadUp,
            gilrs::Button::DPadDown => Button::DPadDown,
            gilrs::Button::DPadLeft => Button::DPadLeft,
            gilrs::Button::DPadRight => Button::DPadRight,
            gilrs::Button::South => Button::A,
            gilrs::Button::West => Button::X,
            gilrs::Button::North => Button::Y,
            gilrs::Button::East => Button::B,
            gilrs::Button::LeftThumb => Button::LeftStick,
            gilrs::Button::RightThumb => Button::RigthStick,
            gilrs::Button::RightTrigger2 => Button::RightTrigger,
            gilrs::Button::RightTrigger => Button::RightBumper,
            gilrs::Button::LeftTrigger2 => Button::LeftTrigger,
            gilrs::Button::LeftTrigger => Button::LeftBumper,
            _ => Button::Unknown,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Axis {
    LeftStickX,
    LeftStickY,
    LeftZ,
    RightStickX,
    RightStickY,
    RightZ,
    Unknown,
}

impl Axis {
    pub fn from(axis: gilrs::Axis) -> Self {
        match axis {
            gilrs::Axis::Unknown => Self::Unknown,
            gilrs::Axis::LeftStickX => Self::LeftStickX,
            gilrs::Axis::LeftStickY => Self::LeftStickY,
            gilrs::Axis::LeftZ => Self::LeftZ,
            gilrs::Axis::RightStickX => Self::RightStickX,
            gilrs::Axis::RightStickY => Self::RightStickY,
            gilrs::Axis::RightZ => Self::RightZ,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AxisState {
    pub axis: Axis,
    pub value: f32,
}

impl AxisState {
    pub fn new(axis: Axis, value: f32) -> Self {
        Self { axis, value }
    }
}

#[derive(Debug, TypeGetter, Resource, Clone, Copy)]
pub struct ControllerAxisState {
    left_stick_x: AxisState,
    left_stick_y: AxisState,
    left_z: AxisState,
    right_stick_x: AxisState,
    right_stick_y: AxisState,
    right_z: AxisState,
}

impl ControllerAxisState {
    pub fn new() -> Self {
        Self {
            left_stick_x: AxisState::new(Axis::LeftStickX, 0.0),
            left_stick_y: AxisState::new(Axis::LeftStickY, 0.0),
            left_z: AxisState::new(Axis::LeftZ, 0.0),
            right_stick_x: AxisState::new(Axis::RightStickX, 0.0),
            right_stick_y: AxisState::new(Axis::RightStickY, 0.0),
            right_z: AxisState::new(Axis::RightZ, 0.0),
        }
    }

    pub fn iter_non_zero(self) -> impl Iterator<Item = AxisState> {
        let mut state_vec = vec![];
        if self.left_stick_x.value != 0.0 {
            state_vec.push(self.left_stick_x);
        }
        if self.left_stick_y.value != 0.0 {
            state_vec.push(self.left_stick_y);
        }
        if self.left_z.value != 0.0 {
            state_vec.push(self.left_z);
        }
        if self.right_stick_x.value != 0.0 {
            state_vec.push(self.right_stick_x);
        }
        if self.right_stick_y.value != 0.0 {
            state_vec.push(self.right_stick_y);
        }
        if self.right_z.value != 0.0 {
            state_vec.push(self.right_z);
        }

        state_vec.into_iter()
    }

    pub fn apply_new_state(&mut self, new_state: AxisState) {
        match new_state.axis {
            Axis::Unknown => error!("unknown stick state"),
            Axis::LeftStickX => self.left_stick_x.value = new_state.value,
            Axis::LeftStickY => self.left_stick_y.value = new_state.value,
            Axis::LeftZ => self.left_z.value = new_state.value,
            Axis::RightStickX => self.right_stick_x.value = new_state.value,
            Axis::RightStickY => self.right_stick_y.value = new_state.value,
            Axis::RightZ => self.right_z.value = new_state.value,
        }
    }
}

pub struct PerfCounter {
    begin: Option<SystemTime>,
    begin_debug_event: Option<SystemTime>,
    end: Option<SystemTime>,
    end_debug_event: Option<SystemTime>,
    last_fram_duration: Option<Duration>,
    frames: usize,
    total_frames: usize,
    lost_frames: usize,
    lost_frames_sum: usize,
    highest_lost_frames: usize,
    frames_sum: f64,
    iterations: usize,
    target_frame_len: Option<f64>,
    duration: f64,
    start_of_second: Duration,
}

impl PerfCounter {
    pub fn new(target_frame_len: Option<f64>) -> Self {
        Self {
            begin: None,
            begin_debug_event: None,
            end: None,
            end_debug_event: None,
            last_fram_duration: None,
            frames: 0,
            total_frames: 0,
            lost_frames: 0,
            lost_frames_sum: 0,
            highest_lost_frames: 0,
            frames_sum: 0.0,
            iterations: 0,
            target_frame_len,
            duration: 0.0,
            start_of_second: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is a construct"),
        }
    }

    pub fn last_frame_duration(&self) -> Option<Duration> {
        self.last_fram_duration
    }

    pub fn start(&mut self) {
        self.begin = Some(SystemTime::now());
    }

    pub fn start_debug_event(&mut self) {
        self.begin_debug_event = Some(SystemTime::now());
    }

    pub fn current_frame_len(&self) -> Result<Duration, std::time::SystemTimeError> {
        Ok(SystemTime::now().duration_since(self.begin.unwrap())?)
    }

    pub fn should_advance(&self) -> bool {
        self.target_frame_len.is_none()
            || self
                .current_frame_len()
                .map(|dur| dur.as_secs_f64())
                .unwrap_or_default()
                >= self.target_frame_len.unwrap()
    }

    pub fn stop(&mut self) {
        self.end = Some(SystemTime::now());

        // trace!(
        //     "> Measured Frame Length: {},\tTarget Frame Length: {},\tLoss: {}",
        //     self.current_frame_len().unwrap_or_default().as_secs_f64(),
        //     self.target_frame_len.unwrap_or_default(),
        //     (self.current_frame_len().unwrap_or_default().as_secs_f64()
        //         - self.target_frame_len.unwrap_or_default())
        //     .abs()
        // );
        self.frames_sum += self.current_frame_len().unwrap_or_default().as_secs_f64();

        self.frames += 1;

        self.last_fram_duration = Some(self.current_frame_len().unwrap_or_default());

        self.duration = self
            .end
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .expect("time is a construct")
            .as_secs_f64()
            - self.start_of_second.as_secs_f64();

        if self.duration >= 1.0 {
            self.start_of_second = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is a construct");
            self.total_frames += self.frames;

            trace!(
                "< Frames {},\tDuration: {},\tExpected {} Frames: {},\tLost Frames: {}",
                self.frames,
                self.duration,
                self.frames,
                self.frames_sum,
                self.lost_frames
            );

            if self.lost_frames > self.highest_lost_frames {
                self.highest_lost_frames = self.lost_frames;
            }
            self.frames = 0;
            self.lost_frames = 0;
            self.frames_sum = 0.0;
            self.iterations += 1;
        }
    }

    pub fn stop_debug_event(&mut self) {
        self.end_debug_event = Some(SystemTime::now());
    }

    pub fn query_last_debug_event(&self) -> Option<Duration> {
        if let Some(start) = self.begin_debug_event {
            if let Some(end) = self.end_debug_event {
                let dur = end.duration_since(start);
                if dur.is_ok() {
                    return Some(dur.unwrap());
                } else {
                    return None;
                }
            }
        }

        None
    }

    pub fn exit_stats(&self) {
        info!(
            ">> Iterations: {},\tFPS: {},\tTotal Lost Frames: {},\tAverage: {},\tHigh:{}",
            self.iterations,
            self.total_frames / self.iterations,
            self.lost_frames_sum,
            self.lost_frames_sum / self.iterations,
            self.highest_lost_frames
        );
    }
}
