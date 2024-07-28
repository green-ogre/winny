use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use app::{
    plugins::Plugin,
    window::{Window, WindowResized},
};
use fxhash::FxHashMap;
use util::tracing::trace;

use ecs::{
    Commands, Res, ResMut, SparseArrayIndex, SparseSet, Take, WinnyComponent, WinnyResource,
};
use wgpu::TextureFormat;

pub mod prelude;

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
            .insert_resource(BindGroups::default())
            .insert_resource(Buffers::default())
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
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.05,
                        g: 0.05,
                        b: 0.05,
                        a: 1.0,
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

// NOTE: this MUST be the first system ran as the StartUp schedule EXPECTS the renderer and its resources to exist
fn startup(mut commands: Commands, window: Res<Window>) {
    let (device, queue, renderer) = Renderer::new(Arc::clone(&window.winit_window));
    let config = renderer.config();
    trace!("Render startup: {:?}", config);

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
    pub fn new(window: Arc<app::winit::window::Window>) -> (wgpu::Device, wgpu::Queue, Self) {
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
                // memory_hints: wgpu::MemoryHints::Performance,
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
pub struct RenderConfig {
    pub dimensions: Dimensions,
    pub format: wgpu::TextureFormat,
    pub max_z: f32,
}

impl RenderConfig {
    fn from_config(value: &wgpu::SurfaceConfiguration) -> Self {
        Self {
            dimensions: Dimensions(value.width, value.height),
            format: value.format,
            max_z: 1000.,
        }
    }
}

impl RenderConfig {
    pub fn width(&self) -> f32 {
        self.dimensions.0 as f32
    }

    pub fn height(&self) -> f32 {
        self.dimensions.1 as f32
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }
}

#[derive(Debug, Clone)]
pub struct Dimensions(pub u32, pub u32);

#[derive(WinnyComponent)]
pub struct RenderBuffer(pub wgpu::Buffer);

impl Deref for RenderBuffer {
    type Target = wgpu::Buffer;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(WinnyComponent)]
pub struct RenderBindGroup(pub wgpu::BindGroup);

impl Deref for RenderBindGroup {
    type Target = wgpu::BindGroup;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(WinnyComponent, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BindGroupHandle(pub usize);

impl SparseArrayIndex for BindGroupHandle {
    fn index(&self) -> usize {
        self.0
    }
}

#[derive(WinnyResource, Default)]
pub struct BindGroups {
    bindings: SparseSet<BindGroupHandle, RenderBindGroup>,
    stored_bindings: FxHashMap<String, BindGroupHandle>,
}

impl BindGroups {
    pub fn get(&self, handle: BindGroupHandle) -> Option<&RenderBindGroup> {
        self.bindings.get(&handle)
    }

    pub fn get_handle_or_insert_with(
        &mut self,
        path: &String,
        bind_group: impl FnOnce() -> RenderBindGroup,
    ) -> BindGroupHandle {
        if let Some(handle) = self.stored_bindings.get(path) {
            *handle
        } else {
            util::tracing::info!("inserting new bind group");
            let index = self.bindings.insert_in_first_empty(bind_group());
            let handle = BindGroupHandle(index);
            self.stored_bindings.insert(path.clone(), handle);

            handle
        }
    }

    pub fn get_with_path(&self, path: &String) -> Option<BindGroupHandle> {
        self.stored_bindings.get(path).cloned()
    }
}

#[derive(WinnyComponent, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BufferHandle(usize);

impl SparseArrayIndex for BufferHandle {
    fn index(&self) -> usize {
        self.0
    }
}

#[derive(WinnyResource, Default)]
pub struct Buffers {
    buffers: SparseSet<BufferHandle, RenderBuffer>,
    stored_buffers: FxHashMap<String, BufferHandle>,
}

impl Buffers {
    pub fn get(&self, handle: BufferHandle) -> Option<&RenderBuffer> {
        self.buffers.get(&handle)
    }

    pub fn get_handle_or_insert_with(
        &mut self,
        path: &String,
        bind_group: impl FnOnce() -> RenderBuffer,
    ) -> BufferHandle {
        if let Some(handle) = self.stored_buffers.get(path) {
            *handle
        } else {
            util::tracing::info!("inserting new buffer");
            let index = self.buffers.insert_in_first_empty(bind_group());
            let handle = BufferHandle(index);
            self.stored_buffers.insert(path.clone(), handle);

            handle
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderContext {
    pub queue: RenderQueue,
    pub device: RenderDevice,
    pub config: RenderConfig,
}

#[derive(WinnyComponent, PartialEq, Eq)]
pub struct RenderLayer(pub u8);

// pub trait RenderPassCommand: Send + Sync + 'static {
//     // TODO: errors?
//     fn render(
//         &self,
//         // pipelines: &Pipelines,
//         // bind_groups: &BindGroups,
//         // vertex_buffers: &VertexBuffers,
//         // uniform_buffers: &UniformBuffers,
//     );
// }
//
// #[derive(WinnyComponent)]
// pub struct RenderPass {
//     commands: Vec<Box<dyn RenderPassCommand>>,
// }

// impl RenderPass {
//     pub fn new(commands: Vec<Box<dyn RenderPassCommand>>) -> Self {
//         Self { commands }
//     }
//
//     pub fn run(&self
//         pipelines: &Pipelines,
//         bind_groups: &BindGroups,
//         vertex_buffers: &VertexBuffers,
//         uniform_buffers: &UniformBuffers,
//     ) {
//         self.commands.iter().for_each(|c| c.render(pipelines, bind_groups, vertex_buffers, uniform_buffers));
//     }
// }
//
// pub struct Pipelines(FxHashMap<RenderHandle<Pipeline>, >)
//
// #[derive(WinnyBundle)]
// pub struct RenderPassBundle {
//     pub pass: RenderPass,
//     pub layer: RenderLayer,
// }

// pub struct SetPipeline(Handle<RenderPipeline>);
// impl RenderPassCommand for SetPipeline {}
// pub struct SetBindGroup(Handle<BindGroup>);
// impl RenderPassCommand for SetBindGroup {}
// pub struct SetVertexBuffer(Handle<VertexBuffer>);
// impl RenderPassCommand for SetVertexBuffer {}
// pub struct SetUniformBuffer(Handle<UniformBuffer>);
// impl RenderPassCommand for SetUniformBuffer {}
// pub struct DrawInstanced();
// impl RenderPassCommand for DrawInstanced {}
// pub struct Draw();
// impl RenderPassCommand for Draw {}
