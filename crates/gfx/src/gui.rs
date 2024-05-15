pub struct EguiRenderer {
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
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

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        window: &Window,
        window_surface_view: &wgpu::TextureView,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
        run_ui: impl FnOnce(&egui::Context),
    ) {
        // Call before take_egui_input
        egui_winit::update_viewport_info(
            &mut egui::ViewportInfo::default(),
            self.state.egui_ctx(),
            window,
        );
        let raw_input = self.state.take_egui_input(&window);
        let full_output = self.state.egui_ctx().run(raw_input, |_| {
            run_ui(&self.state.egui_ctx());
        });

        self.state
            .handle_platform_output(&window, full_output.platform_output);

        let tris = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, window.scale_factor() as f32);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(&device, &queue, *id, &image_delta);
        }
        self.renderer
            .update_buffers(&device, &queue, encoder, &tris, &screen_descriptor);

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
        drop(rpass);
        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }
}
