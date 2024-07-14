use std::{cell::UnsafeCell, marker::PhantomData};

use crate::{
    Archetypes, Bundles, Components, Entities, Res, ResMut, Resource, ResourceId, Resources,
    Tables, World,
};

use util::tracing::error;

// Useful for giving safe, multithreaded access to a ['World']
#[derive(Debug, Clone, Copy)]
pub struct UnsafeWorldCell<'w>(*mut World, PhantomData<(&'w World, &'w UnsafeCell<World>)>);

unsafe impl<'w> Send for UnsafeWorldCell<'w> {}
unsafe impl<'w> Sync for UnsafeWorldCell<'w> {}

impl<'w> UnsafeWorldCell<'w> {
    pub fn new(world: &'w World) -> Self {
        Self(world as *const World as *mut World, PhantomData)
    }

    pub unsafe fn read_only(&self) -> &'w World {
        &*self.0 as &World
    }

    pub unsafe fn read_and_write(&self) -> &'w mut World {
        &mut *self.0 as &mut World
    }

    pub unsafe fn tables(self) -> &'w Tables {
        &self.read_only().tables
    }

    pub unsafe fn tables_mut(self) -> &'w mut Tables {
        &mut self.read_and_write().tables
    }

    pub unsafe fn archetypes(self) -> &'w Archetypes {
        &self.read_only().archetypes
    }

    pub unsafe fn resources(self) -> &'w Resources {
        &self.read_only().resources
    }

    pub unsafe fn components(self) -> &'w Components {
        &self.read_only().components
    }

    pub unsafe fn components_mut(self) -> &'w mut Components {
        &mut self.read_and_write().components
    }

    pub unsafe fn entities(self) -> &'w Entities {
        &self.read_only().entities
    }

    pub unsafe fn bundles(self) -> &'w Bundles {
        &self.read_only().bundles
    }

    pub unsafe fn get_resource_id<R: Resource>(self) -> ResourceId {
        self.resources().id::<R>().unwrap_or_else(|| {
            error!(
                "Resource ['{}'] is not registered. Remember to 'app.insert_resource::<R>()...'",
                std::any::type_name::<R>()
            );
            panic!();
        })
    }

    pub unsafe fn get_resource<R: Resource>(self) -> &'w R {
        let id = self.get_resource_id::<R>();
        self.get_resource_by_id(id)
    }

    pub unsafe fn get_resource_mut<R: Resource>(self) -> &'w mut R {
        let id = self.get_resource_id::<R>();
        self.get_resource_mut_by_id(id)
    }

    pub unsafe fn get_resource_ref<R: Resource>(self) -> Res<'w, R> {
        Res::new(self.get_resource::<R>())
    }

    pub unsafe fn get_resource_mut_ref<R: Resource>(self) -> ResMut<'w, R> {
        ResMut::new(self.get_resource_mut::<R>())
    }

    pub unsafe fn get_resource_ref_by_id<R: Resource>(self, id: ResourceId) -> Res<'w, R> {
        Res::new(self.get_resource_by_id::<R>(id))
    }

    pub unsafe fn get_resource_mut_ref_by_id<R: Resource>(self, id: ResourceId) -> ResMut<'w, R> {
        ResMut::new(self.get_resource_mut_by_id::<R>(id))
    }

    pub unsafe fn get_resource_by_id<R: Resource>(self, id: ResourceId) -> &'w R {
        self.try_get_resource_by_id(id).unwrap_or_else(|| {
            error!(
                "Resource ['{}'] is not in storage",
                std::any::type_name::<R>()
            );
            panic!();
        })
    }

    pub unsafe fn get_resource_mut_by_id<R: Resource>(self, id: ResourceId) -> &'w mut R {
        self.try_get_resource_mut_by_id(id).unwrap_or_else(|| {
            error!(
                "Resource ['{}'] is not in storage",
                std::any::type_name::<R>()
            );
            panic!();
        })
    }

    pub unsafe fn try_get_resource_by_id<R: Resource>(self, id: ResourceId) -> Option<&'w R> {
        self.resources().get_ptr::<R>(id).map(|ptr| ptr.as_ref())
    }

    pub unsafe fn try_get_resource_mut_by_id<R: Resource>(
        self,
        id: ResourceId,
    ) -> Option<&'w mut R> {
        self.resources()
            .get_ptr::<R>(id)
            .map(|mut ptr| ptr.as_mut())
    }

    pub unsafe fn try_get_resource_ref_by_id<R: Resource>(
        self,
        id: ResourceId,
    ) -> Option<Res<'w, R>> {
        self.resources()
            .get_ptr::<R>(id)
            .map(|ptr| Res::new(ptr.as_ref()))
    }

    pub unsafe fn try_get_resource_mut_ref_by_id<R: Resource>(
        self,
        id: ResourceId,
    ) -> Option<ResMut<'w, R>> {
        self.resources()
            .get_ptr::<R>(id)
            .map(|mut ptr| ResMut::new(ptr.as_mut()))
    }
}
