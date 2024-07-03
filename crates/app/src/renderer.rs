use std::sync::Arc;

use crate::{
    app::App ,
    plugins::Plugin,
};
use ecs::{ResMut, WinnyResource};

use wgpu::PipelineCompilationOptions;
use winit::window::Window;

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&mut self, app: &mut App) {
        // HACK: need to add RendererPlugin before all other renderers, but
        // need to call render after all others...
        app.add_systems(ecs::Schedule::FlushEvents, render);
    }
}

fn render(mut renderer: ResMut<Renderer>, mut context: ResMut<RenderContext>) {
    context.submit();
    renderer.present();
}

#[derive(WinnyResource)]
pub struct RenderContext {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    encoder: Option<wgpu::CommandEncoder>,
    command_buffer: Vec<wgpu::CommandBuffer>,
}

impl RenderContext {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            device,
            queue,
            encoder: None,
            command_buffer: Vec::new(),
        }
    }

    pub fn begin_render_pass<'a>(
        &'a mut self,
        desc: wgpu::RenderPassDescriptor<'a, '_>,
    ) -> wgpu::RenderPass<'a> {
        let encoder = self.encoder();
        encoder.begin_render_pass(&desc)
    }

    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        self.encoder.get_or_insert(
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default()),
        )
    }

    pub fn finish_encoder(&mut self) {
        if let Some(encoder) = self.encoder.take() {
            self.command_buffer.push(encoder.finish());
        }
    }

    pub fn submit(&mut self) {
        self.queue.submit(self.command_buffer.drain(..));
    }
}

#[derive(WinnyResource)]
pub struct Renderer {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub config: wgpu::SurfaceConfiguration,
    pub size: [u32; 2],
    pub virtual_size: [u32; 2],
    view: Option<(wgpu::SurfaceTexture, wgpu::TextureView)>,
    surface: wgpu::Surface<'static>,
    pub window: Arc<Window>
}

unsafe impl Send for Renderer {}
unsafe impl Sync for Renderer {}

impl Renderer {
    pub fn new(window: Arc<Window>, size: [u32; 2], virtual_size: [u32; 2]) -> Self {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
            },
            None, // Trace path
        ))
        .unwrap();

        let device = Arc::new(device);
        let queue = Arc::new(queue);

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

        let view = None;

        Renderer {
            surface,
            device,
            queue,
            config,
            size,
            view,
            virtual_size,
            window,
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

    pub fn view(&mut self) -> &wgpu::TextureView {
        &self
            .view
            .get_or_insert_with(|| {
                let texture = self
                    .surface
                    .get_current_texture()
                    .map_err(|err| {
                        logger::error!(
                            "Unable to retrieve renderer surface current texture: {:?}",
                            err
                        );
                    })
                    .unwrap();

                let view = texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                (texture, view)
            })
            .1
    }

    pub fn present(&mut self) {
        if let Some((surface, _)) = self.view.take() {
            surface.present();
        }
    }
}

pub fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
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
            compilation_options: PipelineCompilationOptions::default(),
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
            compilation_options: PipelineCompilationOptions::default(),
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
