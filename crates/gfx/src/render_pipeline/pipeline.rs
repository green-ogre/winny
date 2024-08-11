use super::{
    bind_group::BindGroup,
    material::Material,
    shader::{FragmentShader, FragmentShaderSource, VertexShader},
    vertex_buffer::VertexBuffer,
};
use crate::FragmentShaderLoader;
use app::render_util::RenderContext;
use asset::{server::AssetServer, *};

pub struct RenderPipeline2d(pub wgpu::RenderPipeline);

#[derive(Debug)]
pub enum FragmentType {
    Sprite,
    Particle,
    CpuParticle,
    Mesh2d,
}

impl RenderPipeline2d {
    pub fn material_frag<'s, M: Material>(
        material: &M,
        server: &AssetServer,
        frag_type: FragmentType,
        shaders: &'s mut Assets<FragmentShaderSource>,
        context: &RenderContext,
    ) -> &'s FragmentShader {
        let handle = match frag_type {
            FragmentType::Sprite => material.sprite_fragment_shader(server),
            FragmentType::Particle => material.particle_fragment_shader(server),
            FragmentType::CpuParticle => material.cpu_particle_fragment_shader(server),
            FragmentType::Mesh2d => material.mesh_2d_fragment_shader(server),
        };

        shaders
            .get_mut(&handle)
            .expect("Material should produce valid handle to shader: {frag_type:?}")
            .shader(context)
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
        frag_shaders: &mut Assets<FragmentShaderSource>,
        material: M,
    ) -> Self {
        let blend_state = M::BLEND_STATE;
        let mut bind_groups = bind_groups.iter().map(|b| b.layout()).collect::<Vec<_>>();
        bind_groups.push(material_layout);

        let frag_shader =
            Self::material_frag(&material, server, fragment_type, frag_shaders, context);

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
            None,
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
        format: Option<wgpu::TextureFormat>,
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
                            format: format.unwrap_or_else(|| context.config.format()),
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

pub struct LineRenderPipeline(pub wgpu::RenderPipeline);

impl LineRenderPipeline {
    pub fn new(
        label: &str,
        context: &RenderContext,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
        buffers: &[wgpu::VertexBufferLayout<'static>],
        vert_shader: &VertexShader,
        frag_shader: &FragmentShader,
        blend_state: wgpu::BlendState,
        format: Option<wgpu::TextureFormat>,
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
                            format: format.unwrap_or_else(|| context.config.format()),
                            blend: Some(blend_state),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::LineList,
                        ..Default::default()
                    },
                    multisample: Default::default(),
                    depth_stencil: None,
                    multiview: None,
                }),
        )
    }
}
