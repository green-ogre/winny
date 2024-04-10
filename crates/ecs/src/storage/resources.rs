use std::{
    cell::{Ref, RefCell, RefMut},
    fmt::Debug,
    marker::PhantomData,
    ops::Deref,
};

use fxhash::FxHashMap;
use logging::error;

use crate::{any::*, unsafe_world::UnsafeWorldCell, World};

pub trait Resource: Debug + Send {}

pub struct Res<'a, T> {
    value: &'a T,
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

impl<'a, T: Resource> Deref for Res<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

pub struct ResMut<'a, T> {
    value: &'a mut T,
}

impl<'a, T: TypeGetter + Resource> ResMut<'a, T> {
    pub fn new(world: UnsafeWorldCell<'a>) -> Self {
        Self {
            value: unsafe { &mut *world.resource_mut::<T>() },
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
    resources: FxHashMap<TypeId, Box<dyn ResourceData>>,
}

unsafe impl Sync for Resources {}
unsafe impl Send for Resources {}

pub trait ResourceData: Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Resource + TypeGetter + Debug> ResourceData for T {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}

impl Resources {
    pub fn new() -> Self {
        Self {
            resources: FxHashMap::default(),
        }
    }

    pub fn insert<T: Resource + TypeGetter>(&mut self, res: T) {
        self.resources.insert(T::type_id(), Box::new(res));
    }

    pub unsafe fn get_resource_by_id<T: Resource + TypeGetter>(&self, id: TypeId) -> &T {
        if T::type_id() != id {
            error!("Resource {} does not exist: Remeber to 'app.insert_resource::<...>()' your resource!", T::type_name().as_string());
            panic!();
        }

        self.resources
            .get(&id)
            .unwrap()
            .as_ref()
            .as_any()
            .downcast_ref::<T>()
            .unwrap()
    }

    pub fn get_resource_mut_by_id<T: Resource + TypeGetter>(&mut self, id: TypeId) -> &mut T {
        if T::type_id() != id {
            error!("Resource {} does not exist: Remeber to 'app.insert_resource::<...>()' your resource!", T::type_name().as_string());
            panic!();
        }

        self.resources
            .get_mut(&id)
            .unwrap()
            .as_mut()
            .as_any_mut()
            .downcast_mut::<T>()
            .unwrap()
    }
}
