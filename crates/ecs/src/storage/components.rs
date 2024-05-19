use super::*;

pub trait Component: 'static {}

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
        ids.insert(0, any::ENTITY);
        Self { ids }
    }

    pub fn contains<T: TypeGetter>(&self) -> bool {
        self.ids.contains(&TypeId::of::<T>())
    }

    pub fn contains_id(&self, id: &TypeId) -> bool {
        self.ids.contains(id)
    }

    pub fn equivalent(&self, components: &[TypeId]) -> bool {
        self.ids.eq(components)
    }
}
