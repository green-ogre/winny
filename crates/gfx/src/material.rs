use crate::{create_render_pipeline, create_uniform_bind_group, texture::Texture};
use app::{
    app::{AppSchedule, Schedule},
    plugins::Plugin,
};
use asset::{Assets, Handle};
use ecs::{Commands, Query, Res, ResMut, WinnyComponent, WinnyResource};
use render::{
    BindGroupHandle, BindGroups, Dimensions, RenderConfig, RenderDevice, RenderEncoder, RenderQueue,
};
use winny_math::vector::Vec4f;

pub struct MaterialShaderPlugin;

impl Plugin for MaterialShaderPlugin {
    fn build(&mut self, app: &mut app::app::App) {
        app.register_resource::<ShaderMaterial2dPipeline>()
            .add_systems(Schedule::StartUp, startup)
            .add_systems(AppSchedule::PreRender, apply_shader_materials);
    }
}

/// Applies modifications to a [`Texture`] in a fragment shader.
#[derive(WinnyComponent, Default, Debug, Clone, Copy)]
pub struct ShaderMaterial2d {
    pub opacity: Opacity,
    pub saturation: Saturation,
    pub modulation: Modulation,
}

impl ShaderMaterial2d {
    pub(crate) fn as_raw(&self) -> RawShaderMaterial2d {
        RawShaderMaterial2d {
            modulation: self.modulation.clamp(),
            opacity: self.opacity.clamp(),
            saturation: self.saturation.clamp(),
        }
    }
}

/// Uniform of [`ShaderMaterial2d`].
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawShaderMaterial2d {
    modulation: Vec4f,
    opacity: f32,
    saturation: f32,
}

/// Applies the `opacity` to the target of the [`ShaderMaterial2d`]
#[derive(Debug, Clone, Copy)]
pub struct Opacity(pub f32);

impl Opacity {
    pub fn clamp(&self) -> f32 {
        self.0.clamp(0.0, 1.0)
    }
}

impl Default for Opacity {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Applies the `saturation` to the target of the [`ShaderMaterial2d`]
#[derive(Debug, Clone, Copy)]
pub struct Saturation(pub f32);

impl Saturation {
    pub fn clamp(&self) -> f32 {
        self.0.clamp(0.0, 1.0)
    }
}

impl Default for Saturation {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Applies the `modulation` to the target of the [`ShaderMaterial2d`]
#[derive(Debug, Clone, Copy)]
pub struct Modulation(pub Vec4f);

impl Modulation {
    pub fn clamp(&self) -> Vec4f {
        self.0.normalize_homogenous()
    }
}

impl Default for Modulation {
    fn default() -> Self {
        Self(Vec4f::zero())
    }
}

#[derive(WinnyResource)]
struct ShaderMaterial2dPipeline {
    pipeline: wgpu::RenderPipeline,
    texture_buffer: Texture,
    material_uniform: wgpu::Buffer,
    material_uniform_binding: wgpu::BindGroup,
}

impl ShaderMaterial2dPipeline {
    pub fn new(device: &RenderDevice, config: &RenderConfig) -> Self {
        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("shader material"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/material.wgsl").into()),
        };

        let material_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atlas uniforms"),
            size: std::mem::size_of::<RawShaderMaterial2d>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (layout, material_uniform_binding) = create_uniform_bind_group(
            None,
            &device,
            &material_uniform,
            wgpu::ShaderStages::FRAGMENT,
        );

        let texture_layout = Texture::new_layout(device, None, wgpu::ShaderStages::FRAGMENT);

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shader material"),
            bind_group_layouts: &[&layout, &texture_layout],
            push_constant_ranges: &[],
        });

        let pipeline = create_render_pipeline(
            "shader material",
            device,
            &layout,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            None,
            &[],
            shader,
            false,
        );

        let texture_buffer = Texture::empty(
            Dimensions::new(config.width(), config.height()),
            &device,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );

        Self {
            pipeline,
            material_uniform,
            material_uniform_binding,
            texture_buffer,
        }
    }
}

fn startup(mut commands: Commands, device: Res<RenderDevice>, config: Res<RenderConfig>) {
    commands.insert_resource(ShaderMaterial2dPipeline::new(&device, &config));
}

fn apply_shader_materials(
    mut encoder: ResMut<RenderEncoder>,
    mut renderer: ResMut<ShaderMaterial2dPipeline>,
    queue: Res<RenderQueue>,
    device: Res<RenderDevice>,
    textures: Res<Assets<Texture>>,
    bindings: Res<BindGroups>,
    mats: Query<(Handle<Texture>, BindGroupHandle, ShaderMaterial2d)>,
) {
    for (texture_handle, bind_handle, material) in mats.iter() {
        if let Some(texture) = textures.get(texture_handle) {
            if let Some(binding) = bindings.get(*bind_handle) {
                if texture.height() * texture.width()
                    > renderer.texture_buffer.width() * renderer.texture_buffer.height()
                {
                    let texture = Texture::empty(
                        Dimensions::new(texture.width(), texture.height()),
                        &device,
                        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                    );
                    let _ = std::mem::replace(&mut renderer.texture_buffer, texture);
                }

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("shader_material_2d"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: renderer.texture_buffer.view(),
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });

                    queue.write_buffer(
                        &renderer.material_uniform,
                        0,
                        bytemuck::cast_slice(&[material.as_raw()]),
                    );
                    render_pass.set_pipeline(&renderer.pipeline);
                    render_pass.set_bind_group(0, &renderer.material_uniform_binding, &[]);
                    render_pass.set_bind_group(1, binding, &[]);
                    render_pass.draw(0..3, 0..1);
                }

                encoder.copy_texture_to_texture(
                    wgpu::ImageCopyTexture {
                        texture: &renderer.texture_buffer.texture(),
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::ImageCopyTexture {
                        texture: &texture.asset.texture(),
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::Extent3d {
                        width: texture.asset.width(),
                        height: texture.asset.height(),
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
    }
}
