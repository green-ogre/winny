use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use app::{
    plugins::Plugin,
    window::{WindowResized, WinitWindow},
};
use util::tracing::trace;

use app::window::winit::window::Window;
use ecs::{Commands, Res, ResMut, Take, WinnyResource};
use wgpu::TextureFormat;

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&mut self, app: &mut app::app::App) {
        #[cfg(target_arch = "wasm32")]
        {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        }

        app.register_resource::<Renderer>()
            .register_resource::<RenderView>()
            .register_resource::<RenderOutput>()
            .register_resource::<RenderEncoder>()
            .register_resource::<RenderQueue>()
            .register_resource::<RenderDevice>()
            .register_resource::<RenderConfig>()
            .add_systems(ecs::Schedule::Resized, resize)
            // .add_systems(ecs::Schedule::SubmitEncoder, submit_encoder)
            .add_systems(ecs::Schedule::PreStartUp, startup)
            .add_systems(ecs::Schedule::PrepareRender, (start_render, clear_screen))
            .add_systems(ecs::Schedule::Present, present);
    }
}

fn clear_screen(mut encoder: ResMut<RenderEncoder>, view: Res<RenderView>) {
    {
        let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("downscale pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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

// NOTE: this MUST be the first system ran as the StartUp schedule EXPECTS the renderer and its resources to exist
fn startup(mut commands: Commands, window: Res<WinitWindow>) {
    let (device, queue, renderer) = Renderer::new(Arc::clone(&window));
    let config = renderer.config();
    util::tracing::info!("Render startup: {:?}", config);

    commands
        .insert_resource(RenderQueue(Arc::new(queue)))
        .insert_resource(RenderDevice(Arc::new(device)))
        .insert_resource(config)
        .insert_resource(renderer);
}

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
    pub fn new(window: Arc<Window>) -> (wgpu::Device, wgpu::Queue, Self) {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                // For compute shaders
                //required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                required_features: wgpu::Features::default(),
                label: None,
            },
            None,
        ))
        .unwrap();
        let surface_caps = surface.get_capabilities(&adapter);
        util::tracing::info!("Surface capabilities: {:?}", surface_caps.formats);
        util::tracing::info!("Surface usages: {:?}", surface_caps.usages);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        util::tracing::info!("Surface present_modes: {:?}", surface_caps.present_modes);
        util::tracing::info!("Surface apha_modes: {:?}", surface_caps.alpha_modes);

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

    pub fn config(&self) -> RenderConfig {
        RenderConfig::from_config(&self.config)
    }

    pub fn surface_config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }
}

fn resize(
    mut renderer: ResMut<Renderer>,
    mut config: ResMut<RenderConfig>,
    device: Res<RenderDevice>,
    resized: Res<WindowResized>,
) {
    renderer.resize(&device, resized.0, resized.1);
    *config = RenderConfig::from_config(&renderer.config);
}

pub fn start_render(mut commands: Commands, renderer: Res<Renderer>, device: Res<RenderDevice>) {
    let output = RenderOutput::new(renderer.surface.get_current_texture().unwrap());
    commands.insert_resource(RenderView::new(
        output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default()),
    ));
    commands.insert_resource(output);
    commands.insert_resource(RenderEncoder::new(
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default()),
    ));
}

pub fn present(
    queue: Res<RenderQueue>,
    encoder: Option<Take<RenderEncoder>>,
    output: Option<Take<RenderOutput>>,
) {
    let Some(output) = output else {
        panic!("[`RenderOutput`] unavailable for present, removed before the end of the [`Schedule::Render`] schedule");
    };

    let Some(encoder) = encoder else {
        panic!("[`RenderEncoder`] unavailable for present, removed before the end of the [`Schedule::Render`] schedule");
    };

    queue.submit(std::iter::once(encoder.into_inner().finish()));

    // NOTE: present() does not submit the window, it buffers the submit to be executed
    // let window = world.resource::<WinitWindow>();
    // window.pre_present_notify();
    output.into_inner().present();
}

pub fn submit_encoder(
    mut commands: Commands,
    queue: Res<RenderQueue>,
    device: Res<RenderDevice>,
    encoder: Take<RenderEncoder>,
) {
    queue.submit(std::iter::once(encoder.into_inner().finish()));
    commands.insert_resource(RenderEncoder::new(
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default()),
    ));
}

#[derive(Debug, WinnyResource)]
pub struct RenderEncoder(wgpu::CommandEncoder);

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

    pub fn finish(self) -> wgpu::CommandBuffer {
        self.0.finish()
    }
}

#[derive(Debug, WinnyResource)]
pub struct RenderView(wgpu::TextureView);

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

    pub fn present(self) {
        self.0.present();
    }
}

#[derive(Debug, WinnyResource, Clone)]
pub struct RenderQueue(pub Arc<wgpu::Queue>);

impl Deref for RenderQueue {
    type Target = wgpu::Queue;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, WinnyResource, Clone)]
pub struct RenderDevice(pub Arc<wgpu::Device>);

impl Deref for RenderDevice {
    type Target = wgpu::Device;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, WinnyResource, Clone)]
pub struct RenderConfig(pub Dimensions, pub wgpu::TextureFormat);

impl Deref for RenderConfig {
    type Target = Dimensions;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RenderConfig {
    fn from_config(value: &wgpu::SurfaceConfiguration) -> Self {
        Self(Dimensions(value.width, value.height), value.format)
    }
}

impl RenderConfig {
    pub fn width(&self) -> u32 {
        self.0 .0
    }

    pub fn height(&self) -> u32 {
        self.0 .1
    }

    pub fn format(&self) -> TextureFormat {
        self.1
    }
}

#[derive(Debug, Clone)]
pub struct Dimensions(pub u32, pub u32);

#[derive(Debug, Clone)]
pub struct RenderContext {
    pub queue: RenderQueue,
    pub device: RenderDevice,
    pub config: RenderConfig,
}
