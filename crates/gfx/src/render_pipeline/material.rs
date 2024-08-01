use super::{
    bind_group::{
        AsBindGroup, AsWgpuResources, DEFAULT_SAMPLER_BINDING, DEFAULT_TEXTURE_BINDING, UNIFORM,
    },
    buffer::AsGpuBuffer,
};
use crate::texture::{SamplerFilterType, Texture};
use app::plugins::Plugin;
use ecs::{Component, WinnyComponent};
use render::RenderContext;
use winny_math::vector::Vec4f;

pub struct MaterialPlugin;

impl Plugin for MaterialPlugin {
    fn build(&mut self, app: &mut app::app::App) {
        // app.register_resource::<ShaderMaterial2dPipeline>()
        //     .add_systems(Schedule::StartUp, startup)
        //     .add_systems(AppSchedule::PreRender, apply_shader_materials);
    }
}

pub trait Material: AsBindGroup + Clone + Component {
    fn resource_state<'s>(&self, texture: &'s Texture) -> <Self as AsWgpuResources>::State<'s>;
    fn fragment_shader(&self) -> &'static str;
}

impl Material for Material2d {
    fn resource_state<'s>(&self, texture: &'s Texture) -> <Self as AsWgpuResources>::State<'s> {
        texture
    }

    fn fragment_shader(&self) -> &'static str {
        include_str!("../shaders/material2d.wgsl")
    }
}

impl AsWgpuResources for Material2d {
    type State<'s> = &'s Texture;

    fn as_wgpu_resources<'s>(
        self,
        context: &RenderContext,
        label: &'static str,
        state: &Self::State<'s>,
    ) -> Vec<super::bind_group::WgpuResource> {
        let texture_resources =
            state.as_wgpu_resources(context, label, &SamplerFilterType::Nearest);
        let uniform_resources = <&[RawMaterial2d] as AsWgpuResources>::as_wgpu_resources(
            &[self.as_raw()],
            context,
            label,
            &wgpu::BufferUsages::UNIFORM,
        );

        vec![texture_resources, uniform_resources]
            .into_iter()
            .flatten()
            .collect()
    }
}

impl AsBindGroup for Material2d {
    const LABEL: &'static str = "default 2d material";
    const BINDING_TYPES: &'static [wgpu::BindingType] =
        &[DEFAULT_TEXTURE_BINDING, DEFAULT_SAMPLER_BINDING, UNIFORM];
    const VISIBILITY: &'static [wgpu::ShaderStages] = &[wgpu::ShaderStages::FRAGMENT; 3];
}

/// Default [`Material`] for all 2D Sprites and Particles.
#[derive(WinnyComponent, Default, Debug, Clone, Copy)]
pub struct Material2d {
    pub opacity: Opacity,
    pub saturation: Saturation,
    pub modulation: Modulation,
}

impl Material2d {
    pub(crate) fn as_raw(&self) -> RawMaterial2d {
        RawMaterial2d {
            modulation: self.modulation.clamp(),
            opacity: self.opacity.clamp(),
            saturation: self.saturation.clamp(),
        }
    }
}

/// Uniform of [`Material2d`].
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawMaterial2d {
    modulation: Vec4f,
    opacity: f32,
    saturation: f32,
}

unsafe impl AsGpuBuffer for RawMaterial2d {}

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
