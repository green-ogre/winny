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
        let resource_id = self.world_mut().get_or_make_resource_id(R::type_id());
        unsafe {
            self.world_mut().resources.insert(res, resource_id);
        }
    }

    pub unsafe fn insert_stored_resource(&self, storage: DumbVec, type_id: TypeId) {
        let resource_id = self.world_mut().get_or_make_resource_id(type_id);
        self.world_mut()
            .resources
            .insert_storage(storage, resource_id);
    }

    pub unsafe fn try_resource<R: Resource>(&self) -> Option<*const R> {
        if self.world().resource_ids.contains_key(&R::type_id()) {
            Some(self.resource_ptr())
        } else {
            None
        }
    }

    pub unsafe fn try_resource_mut<R: Resource>(&self) -> Option<*mut R> {
        if self.world_mut().resource_ids.contains_key(&R::type_id()) {
            Some(self.resource_ptr_mut())
        } else {
            None
        }
    }

    pub unsafe fn resource_ptr<R: Resource>(&self) -> *const R {
        let resource_id = self.read_only().get_resource_id::<R>();
        unsafe { self.world().resources.get_resource_by_id(resource_id) }
    }

    pub unsafe fn resource_ptr_mut<R: Resource>(&self) -> *mut R {
        let resource_id = self.read_only().get_resource_id::<R>();
        unsafe {
            self.world_mut()
                .resources
                .get_resource_mut_by_id(resource_id)
        }
    }

    // pub unsafe fn resource_ref<R: Resource + TypeGetter>(&self) -> Res<'_, R> {
    //     Res::new(*self)
    // }

    // pub unsafe fn resource_mut<R: Resource + TypeGetter>(&self) -> *mut R {
    //     unsafe {
    //         self.mut_world()
    //             .resources
    //             .get_resource_mut_by_id(R::type_id())
    //     }
    // }
}
