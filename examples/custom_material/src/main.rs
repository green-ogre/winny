use winny::prelude::{render_pipeline::buffer::AsGpuBuffer, *};

fn main() {
    App::default()
        .add_plugins((
            DefaultPlugins {
                window: WindowPlugin {
                    close_on_escape: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            MaterialPlugin::<CustomMaterial>::new(),
        ))
        .run();
}

impl Material for CustomMaterial {
    const BLEND_STATE: winny::gfx::wgpu::BlendState = winny::gfx::wgpu::BlendState::ALPHA_BLENDING;

    fn resource_state<'s>(&self, texture: &'s Texture) -> <Self as AsWgpuResources>::State<'s> {
        texture
    }
}

impl AsWgpuResources for CustomMaterial {
    type State<'s> = &'s Texture;

    fn as_wgpu_resources<'s>(
        self,
        context: &RenderContext,
        label: &'static str,
        state: Self::State<'s>,
        _buffer_type: Option<BufferType>,
    ) -> Vec<WgpuResource> {
        let texture_resources =
            state.as_wgpu_resources(context, label, SamplerFilterType::Nearest, None);
        let uniform_resources = <&[RawCustomMaterial] as AsWgpuResources>::as_wgpu_resources(
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

impl AsBindGroup for CustomMaterial {
    const LABEL: &'static str = "custom material";
    const BINDING_TYPES: &'static [wgpu::BindingType] =
        &[DEFAULT_TEXTURE_BINDING, DEFAULT_SAMPLER_BINDING, UNIFORM];
    const VISIBILITY: &'static [wgpu::ShaderStages] = &[wgpu::ShaderStages::FRAGMENT; 3];
}

/// Struct which implements [`Material`].
///
/// Must add [`MaterialPlugin`] to the App.
#[derive(Component, Default, Debug, Clone, Copy)]
pub struct CustomMaterial {}

impl CustomMaterial {
    pub(crate) fn as_raw(&self) -> RawCustomMaterial {
        RawCustomMaterial {}
    }
}

/// Uniform of [`CustomMaterial`].
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawCustomMaterial {}

unsafe impl AsGpuBuffer for RawCustomMaterial {}
