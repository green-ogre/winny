use crate::Asset;
use cereal::{WinnyDeserialize, WinnySerialize};
use ecs::{SparseArrayIndex, WinnyAsEgui, WinnyComponent};
use std::{hash::Hash, marker::PhantomData};

// TODO: could become enum with strong and weak variants which determine
// dynamic loading behaviour

/// Handle to an [`LoadedAsset`] stored within the appropriate [`Assets`] resource.
///
/// Aquired from [`AssetServer::load`].
#[derive(WinnySerialize, WinnyDeserialize, WinnyComponent, Debug)]
pub struct Handle<A: Asset>(AssetId, HandleGeneration, PhantomData<A>);

impl<A: Asset> Clone for Handle<A> {
    fn clone(&self) -> Self {
        Handle::new(self.id())
    }
}

impl<A: Asset> PartialEq for Handle<A> {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl<A: Asset> Eq for Handle<A> {}

impl<A: Asset> Hash for Handle<A> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl<A: Asset> Handle<A> {
    pub fn new(id: AssetId) -> Self {
        Self(id, HandleGeneration::default(), PhantomData)
    }

    pub fn id(&self) -> AssetId {
        self.0
    }

    /// Indicates that the handle does not point to a valid [`LoadedAsset`].
    ///
    /// This will always return [`None`] from [`Assets::get`].
    pub fn dangling() -> Self {
        Self(AssetId(u32::MAX), HandleGeneration::default(), PhantomData)
    }

    pub fn is_dangling(&self) -> bool {
        self.0 .0 == u32::MAX
    }

    pub fn point_to(&mut self, other: &Handle<A>) {
        self.0 = other.id();
        self.mark_changed();
    }

    pub fn mark_changed(&mut self) {
        self.1.increment();
    }

    pub fn is_changed(&mut self) -> bool {
        self.1.is_changed()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ErasedHandle(AssetId);

impl ErasedHandle {
    pub fn new(id: AssetId) -> Self {
        Self(id)
    }

    pub fn id(&self) -> AssetId {
        self.0
    }
}

impl<A: Asset> Into<Handle<A>> for ErasedHandle {
    fn into(self) -> Handle<A> {
        Handle::new(self.0)
    }
}

/// Index into an [`Assets`] resource.
#[derive(
    WinnyAsEgui, WinnySerialize, WinnyDeserialize, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
pub struct AssetId(pub(crate) u32);

impl SparseArrayIndex for AssetId {
    fn index(&self) -> usize {
        self.0 as usize
    }
}

/// Tracks change between ticks.
#[derive(WinnySerialize, WinnyDeserialize, Debug, Copy, Clone, Default)]
struct HandleGeneration {
    previous: u16,
    current: u16,
}

impl HandleGeneration {
    /// Checks if generation has changed, then increments current generation.
    ///
    /// Will always return true after [`Self::increment`].
    pub fn is_changed(&mut self) -> bool {
        let changed = self.previous != self.current;
        if changed {
            self.previous = self.current;
        }

        changed
    }

    pub fn increment(&mut self) {
        self.current = self.current.wrapping_add(1);
    }
}
