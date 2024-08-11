use super::{
    bind_group::{
        AsBindGroup, AsWgpuResources, BufferType, DEFAULT_SAMPLER_BINDING, DEFAULT_TEXTURE_BINDING,
        UNIFORM,
    },
    buffer::AsGpuBuffer,
    shader::FragmentShaderSource,
};
// #[cfg(target_arch = "wasm32")]
use crate::{mesh2d::Mesh2dMatPlugin, particle::CpuParticlePlugin, Image, RenderAssets};
use crate::{
    particle::ParticlePlugin,
    sprite::SpriteMaterialPlugin,
    texture::{SamplerFilterType, Texture},
    BindGroup, CpuParticlePipeline, MaterialMarker, ParticlePipeline, SpritePipeline,
};
use app::render_util::RenderContext;
use app::{core::App, plugins::Plugin};
use asset::{server::AssetServer, *};
use ecs::{system_param::SystemParam, *};
use ecs::{Component, WinnyComponent};
use math::vector::{Vec2f, Vec4f};
use std::{fmt::Debug, marker::PhantomData};

#[derive(Debug)]
pub struct MaterialEguiPlugin;

impl Plugin for MaterialEguiPlugin {
    fn build(&mut self, app: &mut App) {
        app.egui_component::<ColorMaterial>();
    }
}

pub struct MaterialPlugin<M: Material>(PhantomData<M>);

impl<M: Material> Debug for MaterialPlugin<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MaterialPlugin")
    }
}

impl<M: Material> Plugin for MaterialPlugin<M> {
    fn build(&mut self, app: &mut App) {
        // #[cfg(target_arch = "wasm32")]
        // TODO: GPU particles are not working at the moment.
        let particle_plugin = CpuParticlePlugin::<M>::new();
        // #[cfg(not(target_arch = "wasm32"))]
        // let particle_plugin = ParticlePlugin::<M>::new();
        app.add_plugins((
            particle_plugin,
            SpriteMaterialPlugin::<M>::new(),
            Mesh2dMatPlugin::<M>::new(),
        ));
    }
}

impl<M: Material> MaterialPlugin<M> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

pub trait Material: AsBindGroup + Clone + Component {
    const BLEND_STATE: wgpu::BlendState;

    fn resource_state<'s, 'w>(
        &'s self,
        textures: &'w mut RenderAssets<Texture>,
        images: &Assets<Image>,
        context: &Res<RenderContext>,
    ) -> Option<<Self as AsWgpuResources>::State<'w>>;

    fn particle_fragment_shader(&self, server: &AssetServer) -> Handle<FragmentShaderSource> {
        Handle::dangling()
    }
    fn cpu_particle_fragment_shader(&self, server: &AssetServer) -> Handle<FragmentShaderSource> {
        Handle::dangling()
    }
    fn sprite_fragment_shader(&self, server: &AssetServer) -> Handle<FragmentShaderSource> {
        Handle::dangling()
    }
    fn mesh_2d_fragment_shader(&self, server: &AssetServer) -> Handle<FragmentShaderSource> {
        Handle::dangling()
    }

    fn is_init(&self, server: &AssetServer, shaders: &Assets<FragmentShaderSource>) -> bool {
        shaders
            .get(&self.particle_fragment_shader(server))
            .is_some()
            && shaders
                .get(&self.cpu_particle_fragment_shader(server))
                .is_some()
            && shaders.get(&self.sprite_fragment_shader(server)).is_some()
    }

    fn update(&self, _context: &RenderContext, _binding: &BindGroup) {}
}

impl Material for Material2d {
    const BLEND_STATE: wgpu::BlendState = wgpu::BlendState::ALPHA_BLENDING;

    fn resource_state<'s, 'w>(
        &'s self,
        textures: &'w mut RenderAssets<Texture>,
        _images: &Assets<Image>,
        _context: &Res<RenderContext>,
    ) -> Option<<Self as AsWgpuResources>::State<'w>> {
        textures.get(&self.texture)
    }

    fn particle_fragment_shader(&self, server: &AssetServer) -> Handle<FragmentShaderSource> {
        server.load("winny/res/shaders/material2d_particle.wgsl")
    }

    fn cpu_particle_fragment_shader(&self, server: &AssetServer) -> Handle<FragmentShaderSource> {
        server.load("winny/res/shaders/material2d_cpu_particle.wgsl")
    }

    fn sprite_fragment_shader(&self, server: &AssetServer) -> Handle<FragmentShaderSource> {
        server.load("winny/res/shaders/material2d_sprite.wgsl")
    }

    fn mesh_2d_fragment_shader(&self, server: &AssetServer) -> Handle<FragmentShaderSource> {
        server.load("winny/res/shaders/material2d_mesh.wgsl")
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
#[derive(WinnyComponent, Debug, Clone)]
pub struct Material2d {
    pub opacity: Opacity,
    pub saturation: Saturation,
    pub modulation: Modulation,
    pub texture: Handle<Image>,
}

impl Default for Material2d {
    fn default() -> Self {
        Self {
            texture: Handle::dangling(),
            ..Default::default()
        }
    }
}

impl Asset for Material2d {}

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

impl ecs::egui_widget::Widget for Opacity {
    fn display(&mut self, ui: &mut ecs::egui::Ui) {
        ui.with_layout(
            ecs::egui::Layout::left_to_right(ecs::egui::Align::Min),
            |ui| {
                ui.add(
                    egui::DragValue::new(&mut self.0)
                        .speed(0.0005)
                        .range(0.0..=1.0),
                );
            },
        );
    }
}

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

impl ecs::egui_widget::Widget for Saturation {
    fn display(&mut self, ui: &mut ecs::egui::Ui) {
        ui.with_layout(
            ecs::egui::Layout::left_to_right(ecs::egui::Align::Min),
            |ui| {
                ui.add(
                    egui::DragValue::new(&mut self.0)
                        .speed(0.0005)
                        .range(0.0..=1.0),
                );
                self.0 = self.0.clamp(0.0, 1.0);
            },
        );
    }
}

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
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Modulation(pub Vec4f);

impl ecs::egui_widget::Widget for Modulation {
    fn display(&mut self, ui: &mut ecs::egui::Ui) {
        ui.with_layout(
            ecs::egui::Layout::left_to_right(ecs::egui::Align::Min),
            |ui| {
                ui.label("r: ");
                ui.add(
                    egui::DragValue::new(&mut self.0.x)
                        .speed(0.0005)
                        .range(0.0..=1.0),
                );
                ui.label("g: ");
                ui.add(
                    egui::DragValue::new(&mut self.0.y)
                        .speed(0.0005)
                        .range(0.0..=1.0),
                );
                ui.label("b: ");
                ui.add(
                    egui::DragValue::new(&mut self.0.z)
                        .speed(0.0005)
                        .range(0.0..=1.0),
                );
                ui.label("a: ");
                ui.add(
                    egui::DragValue::new(&mut self.0.w)
                        .speed(0.0005)
                        .range(0.0..=1.0),
                );
            },
        );
    }
}

impl Modulation {
    pub fn clamp(&self) -> Vec4f {
        self.0.normalize_homogenous()
    }

    pub fn wgpu_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.0.x as f64,
            g: self.0.y as f64,
            b: self.0.z as f64,
            a: self.0.w as f64,
        }
    }
}

impl Default for Modulation {
    fn default() -> Self {
        Self(Vec4f::zero())
    }
}

/// Simple color material.
#[derive(WinnyComponent, WinnyAsEgui, Default, Debug, Clone, Copy)]
pub struct ColorMaterial {
    pub opacity: Opacity,
    pub saturation: Saturation,
    pub modulation: Modulation,
}

impl Asset for ColorMaterial {}

impl ColorMaterial {
    pub(crate) fn as_raw(&self) -> RawColorMaterial {
        RawColorMaterial {
            modulation: self.modulation.clamp(),
            opacity: self.opacity.clamp(),
            saturation: self.saturation.clamp(),
            _padding: Vec2f::zero(),
        }
    }
}

/// Uniform of [`ColorMaterial`].
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawColorMaterial {
    modulation: Vec4f,
    opacity: f32,
    saturation: f32,
    _padding: Vec2f,
}

unsafe impl AsGpuBuffer for RawColorMaterial {}

impl Material for ColorMaterial {
    const BLEND_STATE: wgpu::BlendState = wgpu::BlendState::ALPHA_BLENDING;

    fn resource_state<'s, 'w>(
        &'s self,
        _textures: &'w mut RenderAssets<Texture>,
        _images: &'w Assets<Image>,
        _context: &Res<RenderContext>,
    ) -> Option<<Self as AsWgpuResources>::State<'w>> {
        Some(())
    }

    fn particle_fragment_shader(&self, server: &AssetServer) -> Handle<FragmentShaderSource> {
        server.load("winny/res/shaders/color_material_particle.wgsl")
    }

    fn cpu_particle_fragment_shader(&self, server: &AssetServer) -> Handle<FragmentShaderSource> {
        server.load("winny/res/shaders/color_material_cpu_particle.wgsl")
    }

    fn sprite_fragment_shader(&self, server: &AssetServer) -> Handle<FragmentShaderSource> {
        server.load("winny/res/shaders/color_material_sprite.wgsl")
    }

    fn mesh_2d_fragment_shader(&self, server: &AssetServer) -> Handle<FragmentShaderSource> {
        server.load("winny/res/shaders/color_material_sprite.wgsl")
    }

    fn update(&self, context: &RenderContext, binding: &BindGroup) {
        RawColorMaterial::write_buffer(context, binding.single_buffer(), &[self.as_raw()]);
    }
}

impl AsWgpuResources for ColorMaterial {
    type State<'s> = ();

    fn as_wgpu_resources<'s>(
        self,
        context: &RenderContext,
        label: &'static str,
        _state: Self::State<'s>,
        _buffer_type: Option<BufferType>,
    ) -> Vec<super::bind_group::WgpuResource> {
        <&[RawColorMaterial] as AsWgpuResources>::as_wgpu_resources(
            &[self.as_raw()],
            context,
            label,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            Some(BufferType::Init),
        )
    }
}

impl AsBindGroup for ColorMaterial {
    const LABEL: &'static str = "color material";
    const BINDING_TYPES: &'static [wgpu::BindingType] = &[UNIFORM];
    const VISIBILITY: &'static [wgpu::ShaderStages] = &[wgpu::ShaderStages::FRAGMENT];
}
