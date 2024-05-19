use std::{
    cell::{Ref, RefCell, RefMut},
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use fxhash::FxHashMap;
use logger::error;

use crate::{
    any::*, new_dumb_drop, unsafe_world::UnsafeWorldCell, world, DumbVec, MutableSparseSet, World,
};

pub trait Resource: Debug + Send {}

pub struct Res<'a, T> {
    value: &'a T,
}

impl<'a, T> Deref for Res<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, T: Debug> Debug for Res<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Res").field("value", &self.value).finish()
    }
}

impl<'a, T: TypeGetter + Resource> Res<'a, T> {
    pub fn new(world: UnsafeWorldCell<'a>) -> Self {
        Self {
            value: unsafe { &*world.resource::<T>() },
        }
    }

    pub fn from_ref(res: *const T) -> Self {
        Self {
            value: unsafe { &*res },
        }
    }
}

pub struct ResMut<'a, T> {
    value: &'a mut T,
}

impl<'a, T> Deref for ResMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, T> DerefMut for ResMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl<'a, T: TypeGetter + Resource> ResMut<'a, T> {
    pub fn new(world: UnsafeWorldCell<'a>) -> Self {
        Self {
            value: unsafe { &mut *world.resource_mut::<T>() },
        }
    }

    pub fn from_ref_mut(res: *mut T) -> Self {
        Self {
            value: unsafe { &mut *res },
        }
    }
}

impl<'a, T: Resource + TypeGetter> AsRef<T> for Res<'a, T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<'a, T: Resource + TypeGetter> AsMut<T> for ResMut<'a, T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

// TODO: the goal here would be to completely type erase this and store
// everything with pointers :P
#[derive(Debug)]
pub struct Resources {
    resources: MutableSparseSet<TypeId, DumbVec>,
}

unsafe impl Sync for Resources {}
unsafe impl Send for Resources {}

impl Resources {
    pub fn new() -> Self {
        Self {
            resources: MutableSparseSet::new(),
        }
    }

    pub fn insert<T: Resource + TypeGetter>(&mut self, res: T) {
        let mut storage = DumbVec::new(std::alloc::Layout::new::<T>(), 1, new_dumb_drop::<T>());
        storage.push(res).unwrap();

        self.resources.insert(T::type_id(), storage);
    }

    pub fn insert_storage(&mut self, storage: DumbVec, type_id: TypeId) {
        self.resources.insert(type_id, storage);
    }

    pub unsafe fn get_resource_by_id<T: Resource + TypeGetter>(&self, id: TypeId) -> &T {
        if let Some(res) = self.resources.get_value(&id) {
            return res.get_unchecked(0).cast::<T>().as_ref();
        } else {
            error!(
            "Resource [{}] does not exist: Remeber to 'app.insert_resource::<...>()' your resource!",
            T::type_name().as_string()
        );
            panic!();
        }
    }

    pub fn get_resource_mut_by_id<T: Resource + TypeGetter>(&mut self, id: TypeId) -> &mut T {
        if let Some(res) = self.resources.get_value_mut(&id) {
            return unsafe { res.get_unchecked(0).cast::<T>().as_mut() };
        } else {
            error!(
            "Resource [{}] does not exist: Remeber to 'app.insert_resource::<...>()' your resource!",
            T::type_name().as_string()
        );
            panic!();
        }
    }
}
