#![allow(unused)]

// TODO: does not display

use app::{app::App, plugins::Plugin};
use ecs::{Commands, Res, ResMut, WinnyResource};
use egui_winit::EventResponse;
use winit::{event::WindowEvent, window::Window};

use crate::renderer::{Renderer, RendererContext};

#[derive(WinnyResource)]
pub struct EguiRenderer {
    pub state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

unsafe impl Send for EguiRenderer {}

impl std::fmt::Debug for EguiRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EguiRenderer")
    }
}

impl EguiRenderer {
    pub fn new(
        device: &wgpu::Device,
        output_color_format: wgpu::TextureFormat,
        msaa_samples: u32,
        window: &Window,
    ) -> Self {
        let egui_context = egui::Context::default();
        let egui_state =
            egui_winit::State::new(egui_context, egui::ViewportId::ROOT, &window, None, None);
        let egui_renderer =
            egui_wgpu::Renderer::new(device, output_color_format, None, msaa_samples);

        Self {
            state: egui_state,
            renderer: egui_renderer,
        }
    }

    pub fn handle_input(
        &mut self,
        window: &Window,
        event: Option<&WindowEvent>,
        mouse_delta: Option<(f64, f64)>,
    ) -> Option<EventResponse> {
        if let Some(event) = event.and_then(|e| Some(e)) {
            Some(self.state.on_window_event(&window, &event))
        } else if let Some(mouse_delta) = mouse_delta.and_then(|d| Some(d)) {
            let _ = self.state.on_mouse_motion(mouse_delta);
            None
        } else {
            None
        }
    }

    pub fn begin_frame(&mut self, window: &Window) {
        // Call before take_egui_input
        egui_winit::update_viewport_info(
            &mut egui::ViewportInfo::default(),
            self.state.egui_ctx(),
            window,
        );
        let raw_input = self.state.take_egui_input(&window);
        self.state.egui_ctx().begin_frame(raw_input);
    }

    pub fn end_frame(
        &mut self,
        renderer: &Renderer,
        renderer_context: &mut RendererContext,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
    ) {
        let full_output = self.state.egui_ctx().end_frame();

        self.state
            .handle_platform_output(&renderer.window, full_output.platform_output);

        let tris = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, renderer.window.scale_factor() as f32);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(&renderer.device, &renderer.queue, *id, &image_delta);
        }
        self.renderer.update_buffers(
            &renderer.device,
            &renderer.queue,
            &mut renderer_context.encoder.as_mut().unwrap(),
            &tris,
            &screen_descriptor,
        );

        let encoder = renderer_context.encoder.as_mut().unwrap();
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &renderer_context.view.as_ref().unwrap(),
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
        drop(rpass);
        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }
}

fn begin_frame(mut egui_renderer: ResMut<EguiRenderer>, renderer: ecs::Res<Renderer>) {
    egui_renderer.begin_frame(&renderer.window);
}

fn egui_render(
    renderer: ResMut<Renderer>,
    mut renderer_context: ResMut<RendererContext>,
    mut egui_renderer: ResMut<EguiRenderer>,
) {
    let pixels_per_point = renderer.window.scale_factor() as f32;
    let size_in_pixels = [
        renderer.window.inner_size().width,
        renderer.window.inner_size().height,
    ];

    egui_renderer.end_frame(
        &renderer,
        &mut renderer_context,
        egui_wgpu::ScreenDescriptor {
            size_in_pixels,
            pixels_per_point,
        },
    );
}

fn build_egui(mut commands: Commands, renderer: Res<Renderer>) {
    let egui_renderer = EguiRenderer::new(
        &renderer.device,
        renderer.config.format,
        1,
        &renderer.window,
    );

    commands.insert_resource(egui_renderer);
}

pub struct EguiPlugin;

impl Plugin for EguiPlugin {
    fn build(&mut self, app: &mut App) {
        app.add_systems(ecs::Schedule::StartUp, (build_egui,));
        app.add_systems(ecs::Schedule::PreUpdate, (begin_frame,));
        app.add_systems(ecs::Schedule::PostUpdate, (egui_render,));
    }
}
