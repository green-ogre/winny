use std::{cell::UnsafeCell, marker::PhantomData};

use crate::{DumbVec, Res, ResMut, Resource, TypeGetter, TypeId, TypeName, World};

#[derive(Debug, Clone, Copy)]
pub struct UnsafeWorldCell<'w>(*mut World, PhantomData<&'w World>);

unsafe impl<'w> Send for UnsafeWorldCell<'w> {}
unsafe impl<'w> Sync for UnsafeWorldCell<'w> {}

impl<'w> UnsafeWorldCell<'w> {
    pub fn new(world: &World) -> Self {
        Self(world as *const World as *mut World, PhantomData)
    }

    pub unsafe fn read_only<'a>(&self) -> &'a World {
        &*self.0 as &World
    }

    pub unsafe fn read_and_write<'a>(&self) -> &'a mut World {
        &mut *self.0 as &mut World
    }

    unsafe fn world(&self) -> &World {
        &*self.0 as &World
    }

    unsafe fn mut_world(&self) -> &mut World {
        &mut *self.0
    }

    pub unsafe fn insert_resource<R: Resource + TypeGetter>(&self, res: R) {
        unsafe {
            self.mut_world().resources.insert(res);
        }
    }

    pub unsafe fn insert_stored_resource(&self, storage: DumbVec, type_id: TypeId) {
        self.mut_world().resources.insert_storage(storage, type_id);
    }

    pub unsafe fn resource<R: Resource + TypeGetter>(&self) -> *const R {
        let id = R::type_id();

        unsafe { self.world().resources.get_resource_by_id(id) }
    }

    pub unsafe fn resource_ref<R: Resource + TypeGetter>(&self) -> Res<'_, R> {
        Res::new(*self)
    }

    pub unsafe fn resource_mut<R: Resource + TypeGetter>(&self) -> *mut R {
        let id = R::type_id();

        unsafe { self.mut_world().resources.get_resource_mut_by_id(id) }
    }
}
