use super::{
    bind_group::BindGroup,
    material::Material,
    shader::{FragmentShader, VertexShader},
    vertex_buffer::VertexBuffer,
};
use app::render::RenderContext;
use asset::{AssetServer, Assets};

pub struct RenderPipeline2d(pub wgpu::RenderPipeline);

pub enum FragmentType {
    Sprite,
    Particle,
}

impl RenderPipeline2d {
    pub fn from_material<M: Material>(
        label: &str,
        fragment_type: FragmentType,
        context: &RenderContext,
        server: &mut AssetServer,
        bind_groups: &[&BindGroup],
        vertex_buffers: &[&VertexBuffer],
        vert_shader: &VertexShader,
        frag_shaders: &Assets<FragmentShader>,
        material: M,
    ) -> Self {
        let handle = match fragment_type {
            FragmentType::Sprite => material.sprite_fragment_shader(server),
            FragmentType::Particle => material.particle_fragment_shader(server),
        };
        let frag_shader = &frag_shaders
            .get(&handle)
            .expect("Material should produce valid handle to shader")
            .asset;
        let blend_state = M::BLEND_STATE;

        Self::new(
            label,
            context,
            &bind_groups.iter().map(|b| b.layout()).collect::<Vec<_>>(),
            &vertex_buffers
                .iter()
                .map(|b| b.layout())
                .collect::<Vec<_>>(),
            vert_shader,
            frag_shader,
            blend_state,
        )
    }

    pub fn from_material_layout<M: Material>(
        label: &str,
        fragment_type: FragmentType,
        context: &RenderContext,
        server: &mut AssetServer,
        bind_groups: &[&BindGroup],
        material_layout: &wgpu::BindGroupLayout,
        vertex_buffers: &[&VertexBuffer],
        vert_shader: &VertexShader,
        frag_shaders: &Assets<FragmentShader>,
        material: M,
    ) -> Self {
        let handle = match fragment_type {
            FragmentType::Sprite => material.sprite_fragment_shader(server),
            FragmentType::Particle => material.particle_fragment_shader(server),
        };
        let frag_shader = &frag_shaders
            .get(&handle)
            .expect("Material should produce valid handle to shader")
            .asset;
        let blend_state = M::BLEND_STATE;
        let mut bind_groups = bind_groups.iter().map(|b| b.layout()).collect::<Vec<_>>();
        bind_groups.push(material_layout);

        Self::new(
            label,
            context,
            &bind_groups,
            &vertex_buffers
                .iter()
                .map(|b| b.layout())
                .collect::<Vec<_>>(),
            vert_shader,
            frag_shader,
            blend_state,
        )
    }

    pub fn new(
        label: &str,
        context: &RenderContext,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
        buffers: &[wgpu::VertexBufferLayout<'static>],
        vert_shader: &VertexShader,
        frag_shader: &FragmentShader,
        blend_state: wgpu::BlendState,
    ) -> Self {
        let layout = context
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts,
                push_constant_ranges: &[],
            });

        Self(
            context
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &vert_shader.0,
                        entry_point: "vs_main",
                        buffers,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &frag_shader.0,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: context.config.format(),
                            blend: Some(blend_state),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    }),
                    primitive: Default::default(),
                    multisample: Default::default(),
                    depth_stencil: None,
                    multiview: None,
                }),
        )
    }
}
