use app::render::{RenderContext, RenderDevice, RenderQueue};
use app::{
    app::{App, AppSchedule, Schedule},
    input::mouse_and_key::{KeyCode, KeyInput, KeyState, MouseButton, MouseInput, MouseMotion},
    plugins::Plugin,
    window::Window,
};
use ecs::{Commands, EventReader, Res, ResMut, Resource, WinnyResource};
use egui::{Context, Rect, Vec2};
use egui_wgpu::ScreenDescriptor;
use std::{marker::PhantomData, ops::Deref};

use util::prelude::*;

use crate::render::{RenderEncoder, RenderView};

#[derive(WinnyResource)]
pub struct EguiRenderer {
    context: Context,
    renderer: egui_wgpu::Renderer,
    start_time: app::chrono::DateTime<app::chrono::Local>,
    egui_input: egui::RawInput,
    viewport_id: egui::ViewportId,
    pointer_pos_in_points: Option<egui::Pos2>,
    ui_callback: Option<Box<dyn FnOnce(&Context) + Send + Sync + 'static>>,
}

unsafe impl Send for EguiRenderer {}

impl std::fmt::Debug for EguiRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EguiRenderer")
    }
}

impl EguiRenderer {
    pub fn new(device: &RenderDevice, output_color_format: wgpu::TextureFormat) -> Self {
        let msaa_samples = 1;
        let context = egui::Context::default();
        let viewport_id = context.viewport_id();
        let renderer = egui_wgpu::Renderer::new(device, output_color_format, None, msaa_samples);

        Self {
            context,
            renderer,
            viewport_id,
            start_time: app::chrono::Local::now(),
            egui_input: egui::RawInput::default(),
            pointer_pos_in_points: None,
            ui_callback: None,
        }
    }

    pub fn egui_context(&self) -> &Context {
        &self.context
    }

    pub fn draw(&mut self, run_ui: impl FnOnce(&Context) + Send + Sync + 'static) {
        self.ui_callback = Some(Box::new(run_ui));
    }

    // https://github.com/emilk/egui/blob/34db001db14940c948eb03d3fe87f2af2c45daba/crates/egui-winit/src/lib.rs#L698
    pub fn on_key_event(&mut self, event: &KeyInput) {
        util::tracing::trace!("{event:?}");
        let pressed = event.state == KeyState::Pressed;

        if pressed {
            self.egui_input.events.push(egui::Event::Key {
                key: into_key(event.code),
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: self.egui_input.modifiers,
            });
        }
    }

    fn on_mouse_motion(&mut self, window: &Window, event: &MouseMotion) {
        util::tracing::trace!("{event:?}");
        let native_pixels_per_point = window.winit_window.scale_factor() as f32;
        let egui_zoom_factor = self.context.zoom_factor();
        let pixels_per_point = egui_zoom_factor * native_pixels_per_point;

        let pos_in_points = egui::pos2(
            event.0 as f32 / pixels_per_point,
            event.1 as f32 / pixels_per_point,
        );

        self.pointer_pos_in_points = Some(pos_in_points);

        self.egui_input
            .events
            .push(egui::Event::PointerMoved(pos_in_points));
    }

    fn on_mouse_input(&mut self, event: &MouseInput) {
        util::tracing::trace!("{event:?}");
        if let Some(pos) = self.pointer_pos_in_points {
            let pressed = event.state == KeyState::Pressed;
            let button = match event.button {
                MouseButton::Left => egui::PointerButton::Primary,
                MouseButton::Right => egui::PointerButton::Secondary,
            };

            self.egui_input.events.push(egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers: self.egui_input.modifiers,
            });
        }
    }

    // https://github.com/emilk/egui/blob/master/crates/egui-winit/src/lib.rs#L227
    fn take_egui_input(&mut self, window: &Window) -> egui::RawInput {
        self.egui_input.time = Some(
            app::chrono::Local::now()
                .signed_duration_since(self.start_time)
                .num_milliseconds() as f64
                * 1e-3,
        );

        let size = window.winit_window.inner_size();
        let screen_size_in_pixels = Vec2::new(size.width as f32, size.height as f32);

        let native_pixels_per_point = window.winit_window.scale_factor() as f32;
        let egui_zoom_factor = self.context.zoom_factor();
        let screen_size_in_points =
            screen_size_in_pixels / (egui_zoom_factor * native_pixels_per_point);

        self.egui_input.screen_rect = (screen_size_in_points.x > 0.0
            && screen_size_in_points.y > 0.0)
            .then(|| Rect::from_min_size(Default::default(), screen_size_in_points));

        // Tell egui which viewport is now active:
        self.egui_input.viewport_id = self.viewport_id;

        self.egui_input
            .viewports
            .entry(self.viewport_id)
            .or_default()
            .native_pixels_per_point = Some(window.winit_window.scale_factor() as f32);

        self.egui_input.take()
    }

    fn render(
        &mut self,
        device: &RenderDevice,
        queue: &RenderQueue,
        encoder: &mut RenderEncoder,
        window: &Window,
        window_surface_view: &RenderView,
        run_ui: impl FnOnce(&Context),
    ) {
        let size = window.winit_window.inner_size();
        let pixels_per_point = window.winit_window.scale_factor() as f32;

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [size.width as u32, size.height as u32],
            pixels_per_point,
        };
        let raw_input = self.take_egui_input(window);
        let full_output = self.context.run(raw_input, |ui| {
            // callback(ui);
            run_ui(ui);
        });

        let tris = self
            .context
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device.deref(), queue.deref(), *id, &image_delta);
        }

        self.renderer.update_buffers(
            device.deref(),
            queue.deref(),
            &mut encoder.0,
            &tris,
            &screen_descriptor,
        );
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &window_surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                label: Some("egui main render pass"),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.renderer.render(&mut rpass, &tris, &screen_descriptor);
        }

        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }
    }
}

fn render_gui<S: UiRenderState>(
    mut egui: ResMut<EguiRenderer>,
    mut encoder: ResMut<RenderEncoder>,
    mut state: ResMut<S>,
    context: Res<RenderContext>,
    window: Res<Window>,
    view: Res<RenderView>,
) {
    let size = window.winit_window.inner_size();
    if size.width != context.config.width() as u32 || size.height != context.config.height() as u32
    {
        util::tracing::warn!("skipping frame: render/window size mismatch");
        return;
    }

    egui.render(
        &context.device,
        &context.queue,
        &mut encoder,
        &window,
        &view,
        state.ui(),
    );
}

pub trait UiRenderState: Resource {
    fn ui(&mut self) -> impl FnOnce(&Context);
}

fn handle_input(
    mut egui: ResMut<EguiRenderer>,
    window: Res<Window>,
    key_input: EventReader<KeyInput>,
    mouse_input: EventReader<MouseInput>,
    mouse_motion: EventReader<MouseMotion>,
) {
    for key in key_input.peak_read() {
        egui.on_key_event(key);
    }

    for mouse in mouse_input.peak_read() {
        egui.on_mouse_input(mouse);
    }

    for motion in mouse_motion.peak_read() {
        egui.on_mouse_motion(&window, motion);
    }
}

fn startup(mut commands: Commands, context: Res<RenderContext>) {
    let egui_renderer = EguiRenderer::new(&context.device, context.config.format());
    commands.insert_resource(egui_renderer);
}

pub struct EguiPlugin<S: UiRenderState>(PhantomData<S>);

impl<S: UiRenderState> EguiPlugin<S> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<S: UiRenderState> Plugin for EguiPlugin<S> {
    fn build(&mut self, app: &mut App) {
        app.add_systems(Schedule::StartUp, startup)
            .add_systems(Schedule::PreUpdate, handle_input)
            .add_systems(AppSchedule::PostRender, render_gui::<S>)
            .register_resource::<EguiRenderer>();
    }
}

fn into_key(key: KeyCode) -> egui::Key {
    match key {
        KeyCode::A => egui::Key::A,
        KeyCode::B => egui::Key::B,
        KeyCode::C => egui::Key::C,
        KeyCode::D => egui::Key::D,
        KeyCode::E => egui::Key::E,
        KeyCode::F => egui::Key::F,
        KeyCode::G => egui::Key::G,
        KeyCode::H => egui::Key::H,
        KeyCode::I => egui::Key::I,
        KeyCode::J => egui::Key::J,
        KeyCode::K => egui::Key::K,
        KeyCode::L => egui::Key::L,
        KeyCode::M => egui::Key::M,
        KeyCode::N => egui::Key::N,
        KeyCode::O => egui::Key::O,
        KeyCode::P => egui::Key::P,
        KeyCode::Q => egui::Key::Q,
        KeyCode::R => egui::Key::R,
        KeyCode::S => egui::Key::S,
        KeyCode::T => egui::Key::T,
        KeyCode::U => egui::Key::U,
        KeyCode::V => egui::Key::V,
        KeyCode::W => egui::Key::W,
        KeyCode::X => egui::Key::X,
        KeyCode::Y => egui::Key::Y,
        KeyCode::Z => egui::Key::Z,
        KeyCode::Key1 => egui::Key::Num1,
        KeyCode::Key2 => egui::Key::Num2,
        KeyCode::Key3 => egui::Key::Num3,
        KeyCode::Key4 => egui::Key::Num4,
        KeyCode::Key5 => egui::Key::Num5,
        KeyCode::Key6 => egui::Key::Num6,
        KeyCode::Key7 => egui::Key::Num7,
        KeyCode::Key8 => egui::Key::Num8,
        KeyCode::Key9 => egui::Key::Num9,
        KeyCode::Key0 => egui::Key::Num0,
        KeyCode::Space => egui::Key::Space,
        KeyCode::Escape => egui::Key::Escape,
        _ => {
            // TODO: all keys
            warn!("{:?} not converted to egui::Key", key);
            egui::Key::F35
        }
    }
}
