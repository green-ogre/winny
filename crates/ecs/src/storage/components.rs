use std::any::TypeId;

use super::*;

pub trait Component: 'static + Send + Sync + Debug {
    // fn component_id() -> ComponentId;
}

pub trait ComponentStorageType {
    fn storage_type(&self) -> StorageType;
}

pub trait Storage {
    fn storage_type() -> StorageType;
}

impl<T: Storage + 'static> ComponentStorageType for T {
    fn storage_type(&self) -> StorageType {
        T::storage_type()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComponentSet {
    pub ids: Vec<TypeId>,
}

impl ComponentSet {
    pub fn new(mut ids: Vec<TypeId>) -> Self {
        // Assume that Entity is the first Component?
        ids.insert(0, std::any::TypeId::of::<Entity>());
        Self { ids }
    }

    pub fn contains<T: Component>(&self) -> bool {
        self.ids.contains(&TypeId::of::<T>())
    }

    pub fn contains_id(&self, id: &TypeId) -> bool {
        self.ids.contains(id)
    }

    pub fn equivalent(&self, components: &[TypeId]) -> bool {
        self.ids.eq(components)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ComponentId(usize);

impl ComponentId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn id(&self) -> usize {
        self.0
    }
}

impl SparseArrayIndex for ComponentId {
    fn to_index(&self) -> usize {
        self.id()
    }
}
