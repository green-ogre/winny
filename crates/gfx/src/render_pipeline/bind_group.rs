use super::buffer::AsGpuBuffer;
use crate::texture::{Image, SamplerFilterType, Texture};
use crate::RenderView;
use app::render_util::RenderContext;
use app::{core::App, plugins::Plugin};
use asset::*;
use ecs::system_param::SystemParam;
use ecs::{SparseArrayIndex, SparseSet, WinnyAsEgui, WinnyComponent, WinnyResource};
use fxhash::FxHashMap;
use wgpu::BufferUsages;

#[derive(Debug)]
pub struct BindGroupPlugin;

impl Plugin for BindGroupPlugin {
    fn build(&mut self, app: &mut App) {
        app.insert_resource(AssetBindGroups::default());
    }
}

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
#[derive(Debug)]
pub enum WgpuResource {
    Buffer {
        buffer: wgpu::Buffer,
        usage: wgpu::BufferUsages,
    },
    TextureView(wgpu::TextureView),
    Sampler(wgpu::Sampler),
}

#[derive(Debug, Clone, Copy)]
pub enum BufferType {
    Empty(u64),
    Init,
}

impl WgpuResource {
    pub fn as_entire_binding(&self) -> wgpu::BindingResource<'_> {
        match self {
            Self::Buffer { buffer, .. } => buffer.as_entire_binding(),
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
        state: Self::State<'s>,
        buffer_type: Option<BufferType>,
    ) -> Vec<WgpuResource>;
}

impl<T: AsGpuBuffer> AsWgpuResources for &[T] {
    type State<'s> = BufferUsages;

    fn as_wgpu_resources<'s>(
        self,
        context: &RenderContext,
        label: &'static str,
        state: Self::State<'s>,
        buffer_type: Option<BufferType>,
    ) -> Vec<WgpuResource> {
        vec![if let Some(buffer_type) = buffer_type {
            WgpuResource::Buffer {
                buffer: match buffer_type {
                    BufferType::Init => T::create_buffer_init(Some(label), context, self, state),
                    BufferType::Empty(size) => T::create_buffer(Some(label), context, size, state),
                },
                usage: state,
            }
        } else {
            panic!("attempted to create a GpuBuffer without supplying BufferType");
        }]
    }
}

impl AsWgpuResources for &Texture {
    type State<'s> = SamplerFilterType;

    fn as_wgpu_resources<'s>(
        self,
        context: &RenderContext,
        _label: &'static str,
        state: Self::State<'s>,
        _buffer_type: Option<BufferType>,
    ) -> Vec<WgpuResource> {
        vec![
            WgpuResource::TextureView(self.create_view()),
            WgpuResource::Sampler(self.create_sampler(context, &state)),
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
        state: Self::State<'s>,
        _buffer_type: Option<BufferType>,
    ) -> Vec<WgpuResource> {
        self.0.as_wgpu_resources(context, label, state, None)
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
        state: Self::State<'s>,
        _buffer_type: Option<BufferType>,
    ) -> Vec<WgpuResource> {
        self.0.as_wgpu_resources(context, label, state, None)
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
        state: Self::State<'s>,
        _buffer_type: Option<BufferType>,
    ) -> Vec<WgpuResource> {
        self.0.as_wgpu_resources(context, label, state, None)
    }
}

/// Container for a set of binded [`WgpuResource`]s. Contains binding information. Obtained from
/// [`AsBindGroup::as_entire_binding`].
#[derive(Debug)]
pub struct BindGroup {
    resources: Vec<WgpuResource>,
    layout: wgpu::BindGroupLayout,
    binding: wgpu::BindGroup,
}

impl BindGroup {
    pub fn new(
        resources: Vec<WgpuResource>,
        layout: wgpu::BindGroupLayout,
        binding: wgpu::BindGroup,
    ) -> Self {
        Self {
            resources,
            layout,
            binding,
        }
    }

    pub fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    pub fn binding(&self) -> &wgpu::BindGroup {
        &self.binding
    }

    /// Panics #
    ///     If there are no buffers in resources.
    pub fn single_buffer(&self) -> &wgpu::Buffer {
        &self
            .resources
            .iter()
            .find_map(|r| match r {
                WgpuResource::Buffer { buffer, .. } => Some(buffer),
                _ => None,
            })
            .unwrap()
    }

    /// Panics #
    ///     If there are no views in resources.
    pub fn single_texture_view(&self) -> &wgpu::TextureView {
        &self
            .resources
            .iter()
            .find_map(|r| match r {
                WgpuResource::TextureView(view) => Some(view),
                _ => None,
            })
            .unwrap()
    }

    /// Panics #
    ///     If there are no views in resources.
    pub fn take_texture_view(&mut self) -> wgpu::TextureView {
        let index = self
            .resources
            .iter()
            .enumerate()
            .find_map(|(i, r)| match r {
                WgpuResource::TextureView(_) => Some(i),
                _ => None,
            })
            .unwrap();

        match self.resources.remove(index) {
            WgpuResource::TextureView(view) => view,
            _ => unreachable!(),
        }
    }

    pub fn insert_texture_view(&mut self, view: RenderView) {
        self.resources.push(WgpuResource::TextureView(view.0));
    }
}

/// Describes a set of [`WgpuResource`] in a render pipeline.
pub trait AsBindGroup: Sized + AsWgpuResources {
    const LABEL: &'static str;
    const BINDING_TYPES: &'static [wgpu::BindingType];
    const VISIBILITY: &'static [wgpu::ShaderStages];

    fn as_entire_binding<'s>(
        context: &RenderContext,
        raw_resources: Self,
        state: <Self as AsWgpuResources>::State<'s>,
    ) -> BindGroup {
        let resources = Self::as_resources(raw_resources, context, state, Some(BufferType::Init));
        let layout = Self::layout(context);
        let binding = Self::binding(
            context,
            &layout,
            resources
                .iter()
                .map(|r| r.as_entire_binding())
                .collect::<Vec<_>>(),
        );

        BindGroup::new(resources, layout, binding)
    }

    fn as_entire_binding_empty<'s>(
        context: &RenderContext,
        raw_resources: Self,
        size: u64,
        state: <Self as AsWgpuResources>::State<'s>,
    ) -> BindGroup {
        let resources =
            Self::as_resources(raw_resources, context, state, Some(BufferType::Empty(size)));
        let layout = Self::layout(context);
        let binding = Self::binding(
            context,
            &layout,
            resources
                .iter()
                .map(|r| r.as_entire_binding())
                .collect::<Vec<_>>(),
        );

        BindGroup::new(resources, layout, binding)
    }

    fn as_resources<'s>(
        raw_resources: Self,
        context: &RenderContext,
        state: <Self as AsWgpuResources>::State<'s>,
        buffer_type: Option<BufferType>,
    ) -> Vec<WgpuResource> {
        raw_resources.as_wgpu_resources(context, Self::LABEL, state, buffer_type)
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

    /// Panics #
    ///     If buffer_index is greater than or equal to the number of buffers.
    fn write_buffer_resize<T: AsGpuBuffer>(
        context: &RenderContext,
        bind_group: &mut BindGroup,
        contents: &[T],
        buffer_index: usize,
    ) {
        let (buffer, usage) = bind_group
            .resources
            .iter_mut()
            .filter_map(|r| match r {
                WgpuResource::Buffer { buffer, usage } => Some((buffer, usage)),
                _ => None,
            })
            .nth(buffer_index)
            .unwrap();

        if contents.len() * std::mem::size_of::<T>() <= buffer.size() as usize {
            <T as AsGpuBuffer>::write_buffer(context, buffer, contents);
        } else {
            *buffer = <T as AsGpuBuffer>::create_buffer_init(
                Some(Self::LABEL),
                context,
                contents,
                *usage,
            );
        }
    }
}

pub trait BindableAsset: Asset {}

impl BindableAsset for Image {}

/// [`BindGroup`] for a [`BindableAsset`].
#[derive(WinnyComponent, Debug)]
pub struct RenderBindGroup(pub BindGroup);

/// Handle to a [`RenderBindGroup`] and its respective [`BindableAsset`]. Stored within the [`AssetBindGroups`] resource.
#[derive(WinnyComponent, Debug, Clone, Copy)]
pub struct BindGroupHandle(BindGroupId, AssetId);

impl BindGroupHandle {
    pub fn new(bind_id: BindGroupId, asset_id: AssetId) -> Self {
        Self(bind_id, asset_id)
    }

    pub fn id(&self) -> BindGroupId {
        self.0
    }
}

#[derive(WinnyAsEgui, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindGroupId(usize);

impl SparseArrayIndex for BindGroupId {
    fn index(&self) -> usize {
        self.0
    }
}

/// Stores [`RenderBindGroup`]s. Register and retrieve a RenderBindGroup with a
/// [`BindGroupHandle`].
#[derive(WinnyResource, Default)]
pub struct AssetBindGroups {
    bindings: SparseSet<BindGroupId, RenderBindGroup>,
    stored_bindings: FxHashMap<AssetId, BindGroupId>,
}

impl AssetBindGroups {
    pub fn get(&self, handle: BindGroupHandle) -> Option<&RenderBindGroup> {
        self.bindings.get(&handle.0)
    }

    pub fn get_from_id(&self, id: BindGroupId) -> Option<&RenderBindGroup> {
        self.bindings.get(&id)
    }

    pub fn contains(&self, id: AssetId) -> bool {
        self.stored_bindings.contains_key(&id)
    }

    pub fn get_from_handle<A: Asset>(&self, handle: &Handle<A>) -> Option<&RenderBindGroup> {
        self.stored_bindings
            .get(&handle.id())
            .and_then(|b| self.bindings.get(b))
    }

    pub fn get_handle<A: Asset>(&self, handle: &Handle<A>) -> Option<BindGroupHandle> {
        self.stored_bindings
            .get(&handle.id())
            .map(|id| BindGroupHandle::new(*id, handle.id()))
    }

    pub fn insert<A: Asset>(
        &mut self,
        handle: Handle<A>,
        bind_group: RenderBindGroup,
    ) -> BindGroupHandle {
        let bind_id = self
            .bindings
            .insert_in_first_empty(bind_group, |index| BindGroupId(index));
        self.stored_bindings.insert(handle.id(), bind_id);
        BindGroupHandle::new(bind_id, handle.id())
    }

    pub fn get_handle_or_insert_with<A: BindableAsset>(
        &mut self,
        handle: Handle<A>,
        bind_group: impl FnOnce() -> RenderBindGroup,
    ) -> BindGroupHandle {
        if let Some(bind_id) = self.stored_bindings.get(&handle.id()) {
            BindGroupHandle::new(*bind_id, handle.id())
        } else {
            self.insert(handle, bind_group())
        }
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
        #[allow(non_snake_case)]
        impl<$($t: AsWgpuResources),*> AsWgpuResources for ($($t),*) {
            type State<'s> = ($($t::State<'s>,)*);

            fn as_wgpu_resources<'s>(
                self,
                context: &RenderContext,
                label: &'static str,
                state: Self::State<'s>,
                buffer_type: Option<BufferType>,
            ) -> Vec<WgpuResource> {
                let ($($t,)*) = self;

                let resources = vec![
                    $($t.as_wgpu_resources(context, label, tuple_index!(state, $idx), buffer_type),)*
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
