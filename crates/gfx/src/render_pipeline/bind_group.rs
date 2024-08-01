use super::buffer::AsGpuBuffer;
use crate::texture::{SamplerFilterType, Texture};
use render::RenderContext;
use wgpu::BufferUsages;

pub const READ_ONLY_STORAGE: wgpu::BindingType = wgpu::BindingType::Buffer {
    ty: wgpu::BufferBindingType::Storage { read_only: true },
    has_dynamic_offset: false,
    min_binding_size: None,
};

pub const READ_WRITE_STORAGE: wgpu::BindingType = wgpu::BindingType::Buffer {
    ty: wgpu::BufferBindingType::Storage { read_only: false },
    has_dynamic_offset: false,
    min_binding_size: None,
};

pub const UNIFORM: wgpu::BindingType = wgpu::BindingType::Buffer {
    ty: wgpu::BufferBindingType::Uniform,
    has_dynamic_offset: false,
    min_binding_size: None,
};

/// Wraps handle to wgpu resource
pub enum WgpuResource {
    Buffer(wgpu::Buffer),
    TextureView(wgpu::TextureView),
    Sampler(wgpu::Sampler),
}

impl WgpuResource {
    pub fn as_entire_binding(&self) -> wgpu::BindingResource<'_> {
        match self {
            Self::Buffer(b) => b.as_entire_binding(),
            Self::TextureView(v) => wgpu::BindingResource::TextureView(&v),
            Self::Sampler(s) => wgpu::BindingResource::Sampler(&s),
        }
    }
}

/// Converts type into a vector of [`WgpuResource`] which a render pipeline can bind to.
pub trait AsWgpuResources {
    type State<'s>;

    fn as_wgpu_resources<'s>(
        self,
        context: &RenderContext,
        label: &'static str,
        state: &Self::State<'s>,
    ) -> Vec<WgpuResource>;
}

impl<T: AsGpuBuffer> AsWgpuResources for &[T] {
    type State<'s> = BufferUsages;

    fn as_wgpu_resources<'s>(
        self,
        context: &RenderContext,
        label: &'static str,
        state: &Self::State<'s>,
    ) -> Vec<WgpuResource> {
        vec![WgpuResource::Buffer(T::create_buffer_init(
            Some(label),
            context,
            self,
            state,
        ))]
    }
}

impl AsWgpuResources for &Texture {
    type State<'s> = SamplerFilterType;

    fn as_wgpu_resources<'s>(
        self,
        context: &RenderContext,
        _label: &'static str,
        state: &Self::State<'s>,
    ) -> Vec<WgpuResource> {
        vec![
            WgpuResource::TextureView(self.create_view()),
            WgpuResource::Sampler(self.create_sampler(context, state)),
        ]
    }
}

pub const DEFAULT_TEXTURE_BINDING: wgpu::BindingType = wgpu::BindingType::Texture {
    sample_type: wgpu::TextureSampleType::Float { filterable: true },
    view_dimension: wgpu::TextureViewDimension::D2,
    multisampled: false,
};

pub const DEFAULT_SAMPLER_BINDING: wgpu::BindingType =
    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering);

/// [`Texture`] that is visible to the fragment shader.
pub struct FragTexture<'a>(pub &'a Texture);

impl AsBindGroup for FragTexture<'_> {
    const LABEL: &'static str = "frag texture";
    const BINDING_TYPES: &'static [wgpu::BindingType] = &[
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
    ];
    const VISIBILITY: &'static [wgpu::ShaderStages] =
        &[wgpu::ShaderStages::FRAGMENT, wgpu::ShaderStages::FRAGMENT];
}

impl AsWgpuResources for FragTexture<'_> {
    type State<'s> = SamplerFilterType;

    fn as_wgpu_resources<'s>(
        self,
        context: &RenderContext,
        label: &'static str,
        state: &Self::State<'s>,
    ) -> Vec<WgpuResource> {
        self.0.as_wgpu_resources(context, label, state)
    }
}

/// [`Texture`] that is visible to the vertex shader.
pub struct VertTexture<'a>(pub &'a Texture);

impl AsBindGroup for VertTexture<'_> {
    const LABEL: &'static str = "vert texture";
    const BINDING_TYPES: &'static [wgpu::BindingType] = &[
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
    ];
    const VISIBILITY: &'static [wgpu::ShaderStages] =
        &[wgpu::ShaderStages::VERTEX, wgpu::ShaderStages::VERTEX];
}

impl AsWgpuResources for VertTexture<'_> {
    type State<'s> = SamplerFilterType;

    fn as_wgpu_resources<'s>(
        self,
        context: &RenderContext,
        label: &'static str,
        state: &Self::State<'s>,
    ) -> Vec<WgpuResource> {
        self.0.as_wgpu_resources(context, label, state)
    }
}

/// [`Texture`] that is visible to the vertex and fragment shader.
pub struct VertFragTexture<'a>(pub &'a Texture);

impl AsBindGroup for VertFragTexture<'_> {
    const LABEL: &'static str = "vert texture";
    const BINDING_TYPES: &'static [wgpu::BindingType] = &[
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
    ];
    const VISIBILITY: &'static [wgpu::ShaderStages] = &[
        wgpu::ShaderStages::VERTEX_FRAGMENT,
        wgpu::ShaderStages::VERTEX_FRAGMENT,
    ];
}

impl AsWgpuResources for VertFragTexture<'_> {
    type State<'s> = SamplerFilterType;

    fn as_wgpu_resources<'s>(
        self,
        context: &RenderContext,
        label: &'static str,
        state: &Self::State<'s>,
    ) -> Vec<WgpuResource> {
        self.0.as_wgpu_resources(context, label, state)
    }
}
//
// impl<T: AsBindGroup> AsBindGroup for &T {
//     const LABEL: &'static str = T::LABEL;
//     const VISIBILITY: &'static [wgpu::ShaderStages] = T::VISIBILITY;
//     const BINDING_TYPES: &'static [wgpu::BindingType] = T::BINDING_TYPES;
// }

/// Describes a set of [`WgpuResource`] in a render pipeline.
pub trait AsBindGroup: Sized + AsWgpuResources {
    const LABEL: &'static str;
    const BINDING_TYPES: &'static [wgpu::BindingType];
    const VISIBILITY: &'static [wgpu::ShaderStages];

    fn as_entire_binding<'s>(
        context: &RenderContext,
        raw_resources: Self,
        state: &<Self as AsWgpuResources>::State<'s>,
    ) -> (Vec<WgpuResource>, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let resources = Self::as_resources(raw_resources, context, state);
        let layout = Self::layout(context);
        let binding = Self::binding(
            context,
            &layout,
            resources
                .iter()
                .map(|r| r.as_entire_binding())
                .collect::<Vec<_>>(),
        );

        (resources, layout, binding)
    }

    fn as_entire_binding_single_unwrap<'s>(
        context: &RenderContext,
        raw_resources: Self,
        state: &<Self as AsWgpuResources>::State<'s>,
    ) -> (WgpuResource, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let mut resources = Self::as_resources(raw_resources, context, state);
        let layout = Self::layout(context);
        let binding = Self::binding(
            context,
            &layout,
            resources
                .iter()
                .map(|r| r.as_entire_binding())
                .collect::<Vec<_>>(),
        );
        assert!(resources.len() == 1);

        (resources.pop().unwrap(), layout, binding)
    }

    fn as_entire_binding_single_buffer<'s>(
        context: &RenderContext,
        raw_resources: Self,
        state: &<Self as AsWgpuResources>::State<'s>,
    ) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let mut resources = Self::as_resources(raw_resources, context, state);
        let layout = Self::layout(context);
        let binding = Self::binding(
            context,
            &layout,
            resources
                .iter()
                .map(|r| r.as_entire_binding())
                .collect::<Vec<_>>(),
        );
        assert!(resources.len() == 1);

        (
            match resources.pop().unwrap() {
                WgpuResource::Buffer(b) => b,
                _ => unreachable!(),
            },
            layout,
            binding,
        )
    }

    fn as_resources<'s>(
        raw_resources: Self,
        context: &RenderContext,
        state: &<Self as AsWgpuResources>::State<'s>,
    ) -> Vec<WgpuResource> {
        raw_resources.as_wgpu_resources(context, Self::LABEL, state)
    }

    fn layout<'s>(context: &RenderContext) -> wgpu::BindGroupLayout {
        let entries = Self::VISIBILITY
            .iter()
            .enumerate()
            .zip(Self::BINDING_TYPES.iter())
            .map(|((i, v), ty)| wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility: *v,
                ty: *ty,
                count: None,
            })
            .collect::<Vec<_>>();

        context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &entries,
                label: Some(Self::LABEL),
            })
    }

    fn binding<'s>(
        context: &RenderContext,
        layout: &wgpu::BindGroupLayout,
        binding_resources: Vec<wgpu::BindingResource<'s>>,
    ) -> wgpu::BindGroup {
        let entries = binding_resources
            .into_iter()
            .enumerate()
            .map(|(i, r)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: r,
            })
            .collect::<Vec<_>>();

        context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                label: Some(Self::LABEL),
                entries: &entries,
            })
    }
}

macro_rules! expr {
    ($x:expr) => {
        $x
    };
}
macro_rules! tuple_index {
    ($tuple:expr, $idx:tt) => {
        expr!($tuple.$idx)
    };
}

macro_rules! impl_as_wgpu_resources {
    ($(($t:ident, $idx:tt))*) => {
        impl<$($t: AsWgpuResources),*> AsWgpuResources for ($($t),*) {
            type State<'s> = ($($t::State<'s>,)*);

            fn as_wgpu_resources<'s>(
                self,
                context: &RenderContext,
                label: &'static str,
                state: &Self::State<'s>,
            ) -> Vec<WgpuResource> {
                let ($($t,)*) = self;

                let resources = vec![
                    $($t.as_wgpu_resources(context, label, &tuple_index!(state, $idx)),)*
                ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                resources
            }
        }
    };
}

impl_as_wgpu_resources!((A, 0)(B, 1));
impl_as_wgpu_resources!((A, 0)(B, 1)(C, 2));
impl_as_wgpu_resources!((A, 0)(B, 1)(C, 2)(D, 3));
impl_as_wgpu_resources!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4));
impl_as_wgpu_resources!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5));
impl_as_wgpu_resources!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6));
impl_as_wgpu_resources!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7));
impl_as_wgpu_resources!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7)(J, 8));
impl_as_wgpu_resources!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7)(J, 8)(K, 9));
