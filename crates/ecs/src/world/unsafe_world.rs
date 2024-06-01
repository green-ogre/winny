use std::marker::PhantomData;

use super::*;

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

    unsafe fn world_mut(&self) -> &mut World {
        &mut *self.0
    }

    pub unsafe fn insert_resource<R: Resource>(&self, res: R) {
        let resource_id = self
            .world_mut()
            .register_resource(std::any::TypeId::of::<R>());
        self.world_mut().resources.insert(res, resource_id);
    }

    pub unsafe fn insert_stored_resource(&self, storage: DumbVec, type_id: TypeId) {
        let resource_id = self.world_mut().register_resource(type_id);
        self.world_mut()
            .resources
            .insert_storage(storage, resource_id);
    }

    pub unsafe fn try_resource<R: Resource>(&self, id: ResourceId) -> *const R {
        self.world().resources.get_resource_by_id(id)
    }

    pub unsafe fn resource_ptr<R: Resource>(&self, id: ResourceId) -> *const R {
        self.world().resources.get_resource_by_id(id)
    }

    pub unsafe fn resource_ptr_mut<R: Resource>(&self, id: ResourceId) -> *mut R {
        self.world_mut().resources.get_resource_mut_by_id(id)
    }
}
