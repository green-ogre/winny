use app::prelude::*;
use ecs::{AsEgui, Commands, Res, ResMut, Take, WinnyAsEgui, WinnyResource};
use math::vector::Vec4f;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};
use util::{info, tracing::trace};

use crate::Modulation;

#[derive(Debug)]
pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&mut self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));

        app.egui_resource::<ClearColor>()
            .register_resource::<Renderer>()
            .register_resource::<RenderView>()
            .register_resource::<RenderContext>()
            .register_resource::<RenderOutput>()
            .register_resource::<RenderEncoder>()
            .register_resource::<RenderContext>()
            .insert_resource(ClearColor(Modulation(Vec4f::new(0.05, 0.05, 0.05, 1.0))))
            .add_systems(AppSchedule::Resized, resize)
            .add_systems(AppSchedule::RenderStartup, startup)
            .add_systems(AppSchedule::PrepareRender, start_render)
            .add_systems(AppSchedule::PreRender, clear_screen)
            .add_systems(AppSchedule::Present, present);
    }
}

/// Sets default background color.
#[derive(WinnyResource, WinnyAsEgui)]
pub struct ClearColor(pub Modulation);

fn clear_screen(mut encoder: ResMut<RenderEncoder>, view: Res<RenderView>, clear: Res<ClearColor>) {
    {
        let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("downscale pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: clear.0 .0.x as f64,
                        g: clear.0 .0.y as f64,
                        b: clear.0 .0.z as f64,
                        a: clear.0 .0.w as f64,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
    }
}

fn startup(mut commands: Commands, window: Res<Window>) {
    let (device, queue, renderer) = Renderer::new(&window);

    let context = RenderContext {
        device: RenderDevice::new(device),
        queue: RenderQueue::new(queue),
        config: RenderConfig::from_config(&renderer.config),
    };

    commands.insert_resource(context).insert_resource(renderer);
}

/// Handle to the [`wgpu::Surface`]. Used to present the active [`RenderOutput`].
#[derive(WinnyResource)]
pub struct Renderer {
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
}

impl Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer").finish_non_exhaustive()
    }
}

unsafe impl Send for Renderer {}
unsafe impl Sync for Renderer {}

impl Renderer {
    pub fn new(window: &Window) -> (wgpu::Device, wgpu::Queue, Self) {
        let size = window.winit_window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.winit_window.clone())
            .unwrap();
        let adapter = pollster::block_on(wgpu::util::initialize_adapter_from_env_or_default(
            &instance,
            Some(&surface),
        ))
        .expect("No suitable GPU adapters found on the system");

        #[cfg(not(target_arch = "wasm32"))]
        let required_limits = wgpu::Limits::default();
        #[cfg(target_arch = "wasm32")]
        let required_limits = wgpu::Limits::downlevel_webgl2_defaults();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_limits,
                required_features: wgpu::Features::default(),
                label: None,
            },
            None,
        ))
        .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        util::tracing::info!("Surface capabilities: {:?}", surface_caps);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        (device, queue, Self { surface, config })
    }

    pub fn resize(&mut self, device: &RenderDevice, width: u32, height: u32) {
        trace!("resizing renderer");
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&device, &self.config);
        }
    }
}

fn resize(
    mut renderer: ResMut<Renderer>,
    mut context: ResMut<RenderContext>,
    resized: Res<WindowResized>,
) {
    renderer.resize(&context.device, resized.0, resized.1);
    context.config = RenderConfig::from_config(&renderer.config);
}

pub fn start_render(mut commands: Commands, renderer: Res<Renderer>, context: Res<RenderContext>) {
    let output = RenderOutput::new(renderer.surface.get_current_texture().unwrap());
    commands.insert_resource(RenderView::new(
        output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default()),
    ));
    commands.insert_resource(output);
    commands.insert_resource(RenderEncoder::new(
        context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default()),
    ));
}

pub fn present(
    context: Res<RenderContext>,
    encoder: Option<Take<RenderEncoder>>,
    output: Option<Take<RenderOutput>>,
) {
    let Some(output) = output else {
        panic!("[`RenderOutput`] unavailable for present, removed before the end of the [`Schedule::Render`] schedule");
    };

    let Some(encoder) = encoder else {
        panic!("[`RenderEncoder`] unavailable for present, removed before the end of the [`Schedule::Render`] schedule");
    };

    context
        .queue
        .submit(std::iter::once(encoder.into_inner().finish()));

    // NOTE: present() does not submit the window, it buffers the submit to be executed
    // let window = world.resource::<WinitWindow>();
    // window.pre_present_notify();
    output.into_inner().present();
}

/// Handle to the active [`wgpu::CommandEncoder`] in the render app schedule
#[derive(Debug, WinnyResource)]
pub struct RenderEncoder(pub wgpu::CommandEncoder);

impl Deref for RenderEncoder {
    type Target = wgpu::CommandEncoder;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RenderEncoder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl RenderEncoder {
    pub fn new(encoder: wgpu::CommandEncoder) -> Self {
        Self(encoder)
    }

    pub(crate) fn finish(self) -> wgpu::CommandBuffer {
        self.0.finish()
    }
}

/// Handle to the active [`wgpu::TextureView`] in the render app schedule
#[derive(Debug, WinnyResource)]
pub struct RenderView(pub(crate) wgpu::TextureView);

impl Deref for RenderView {
    type Target = wgpu::TextureView;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RenderView {
    pub fn new(view: wgpu::TextureView) -> Self {
        Self(view)
    }
}

/// Wraps the [`wgpu::SurfaceTexture`].
#[derive(Debug, WinnyResource)]
pub struct RenderOutput(wgpu::SurfaceTexture);

impl Deref for RenderOutput {
    type Target = wgpu::SurfaceTexture;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RenderOutput {
    pub fn new(texture: wgpu::SurfaceTexture) -> Self {
        Self(texture)
    }

    pub(crate) fn present(self) {
        self.0.present();
    }
}

// #[derive(WinnyComponent, PartialEq, Eq)]
// pub struct RenderLayer(pub u8);
