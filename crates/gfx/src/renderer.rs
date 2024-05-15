use logger::*;

use wgpu::{
    rwh::{HasDisplayHandle, HasWindowHandle},
    SurfaceTargetUnsafe,
};

use crate::gui::EguiRenderer;

pub struct Renderer<'w> {
    egui_renderer: EguiRenderer,
    render_pipeline: wgpu::RenderPipeline,
    surface: wgpu::Surface<'w>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: [u32; 2],
    // vertex_buffer: wgpu::Buffer,
    // camera_buffer: wgpu::Buffer,
    // camera_bind_group: wgpu::BindGroup,
    // camera_uniform: CameraUniform,
}

impl<'w> Renderer<'w> {
    async fn new<T>(window: &T, size: [u32; 2]) -> Self
    where
        T: HasDisplayHandle + HasWindowHandle,
    {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = unsafe {
            instance
                .create_surface_unsafe(SurfaceTargetUnsafe::from_window(window).unwrap())
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
        info!("Surface Config: {:#?}", config);
        surface.configure(&device, &config);

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

        let egui_renderer = EguiRenderer::new(&device, config.format, 1, window);

        Renderer {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            egui_renderer,
            // vertex_buffer: wgpu::Buffer,
            // camera_buffer: wgpu::Buffer,
            // camera_bind_group: wgpu::BindGroup,
            // camera_uniform: CameraUniform,
        }
    }

    fn resize(&mut self, new_size: [u32; 2]) {
        if new_size[0] > 0 && new_size[1] > 0 {
            self.size = new_size;
            self.config.width = new_size[0];
            self.config.height = new_size[1];
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn update(&mut self) {
        // self.queue.write_buffer(
        //     &self.boid_buffer,
        //     0,
        //     bytemuck::cast_slice(&self.boids.iter().map(|b| b.to_raw()).collect::<Vec<_>>()),
        // );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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
        // render_pass.set_bind_group(0, &self.boid_sprite_bind_group, &[]);
        // render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        // render_pass.set_bind_group(2, &self.boid_color_bind_group, &[]);
        // render_pass.set_bind_group(3, &self.boid_rot_color_bind_group, &[]);
        // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        // render_pass.set_vertex_buffer(1, self.boid_buffer.slice(..));
        // render_pass.draw(0..NUM_BOIDS as u32 * 3, 0..NUM_BOIDS as u32);
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
