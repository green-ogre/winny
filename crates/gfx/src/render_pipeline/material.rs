use super::{
    bind_group::{
        AsBindGroup, AsWgpuResources, BufferType, DEFAULT_SAMPLER_BINDING, DEFAULT_TEXTURE_BINDING,
        UNIFORM,
    },
    buffer::AsGpuBuffer,
    shader::{FragmentShader, FragmentShaderSource},
};
use crate::{
    particle::ParticlePlugin,
    sprite::SpriteMaterialPlugin,
    texture::{SamplerFilterType, Texture},
};
use app::plugins::Plugin;
use app::render::RenderContext;
use asset::prelude::*;
use ecs::{Component, WinnyComponent, WinnyWidget};
use std::marker::PhantomData;
use winny_math::vector::Vec4f;

pub struct MaterialPlugin<M: Material>(PhantomData<M>);

impl<M: Material> Plugin for MaterialPlugin<M> {
    fn build(&mut self, app: &mut app::app::App) {
        app.add_plugins((ParticlePlugin::<M>::new(), SpriteMaterialPlugin::<M>::new()));
    }
}

impl<M: Material> MaterialPlugin<M> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

pub trait Material: AsBindGroup + Clone + Component {
    const BLEND_STATE: wgpu::BlendState;

    fn resource_state<'s>(&self, texture: &'s Texture) -> <Self as AsWgpuResources>::State<'s>;

    fn particle_fragment_shader(&self, _server: &mut AssetServer) -> Handle<FragmentShaderSource> {
        Handle::dangling()
    }

    fn sprite_fragment_shader(&self, _server: &mut AssetServer) -> Handle<FragmentShaderSource> {
        Handle::dangling()
    }
}

impl Material for Material2d {
    const BLEND_STATE: wgpu::BlendState = wgpu::BlendState::ALPHA_BLENDING;

    fn resource_state<'s>(&self, texture: &'s Texture) -> <Self as AsWgpuResources>::State<'s> {
        texture
    }
}

impl AsWgpuResources for Material2d {
    type State<'s> = &'s Texture;

    fn as_wgpu_resources<'s>(
        self,
        context: &RenderContext,
        label: &'static str,
        state: Self::State<'s>,
        _buffer_type: Option<BufferType>,
    ) -> Vec<super::bind_group::WgpuResource> {
        let texture_resources =
            state.as_wgpu_resources(context, label, SamplerFilterType::Nearest, None);
        let uniform_resources = <&[RawMaterial2d] as AsWgpuResources>::as_wgpu_resources(
            &[self.as_raw()],
            context,
            label,
            wgpu::BufferUsages::UNIFORM,
            Some(BufferType::Init),
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
#[derive(WinnyWidget, Debug, Clone, Copy)]
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
#[derive(WinnyWidget, Debug, Clone, Copy)]
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
#[derive(WinnyWidget, Debug, Clone, Copy)]
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
