pub mod commands;
pub mod entity;
pub mod unsafe_world;

pub use commands::*;
pub use entity::*;

use crate::{Event, Events, Res, ResMut, Resource, Resources};

use crate::storage::*;

pub use self::unsafe_world::UnsafeWorldCell;

#[derive(Debug, Default)]
pub struct World {
    pub(crate) archetypes: Archetypes,
    pub(crate) tables: Tables,
    pub(crate) resources: Resources,
    pub(crate) components: Components,
    pub(crate) entities: Entities,
    pub(crate) bundles: Bundles,
}

impl World {
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn as_unsafe_world(&mut self) -> UnsafeWorldCell<'_> {
        UnsafeWorldCell::new_mut(self)
    }

    pub unsafe fn as_unsafe_world_read_only(&self) -> UnsafeWorldCell<'_> {
        UnsafeWorldCell::new(self)
    }

    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        unsafe { self.as_unsafe_world().spawn_bundle::<B>(bundle) }
    }

    pub fn register_resource<R: Resource>(&mut self) -> ResourceId {
        self.resources.register::<R>()
    }

    pub fn insert_resource<R: Resource>(&mut self, res: R) {
        let id = self.register_resource::<R>();
        self.resources.insert(res, id);
    }

    pub fn take_resource<R: Resource>(&mut self) -> Option<R> {
        let id = self.register_resource::<R>();
        self.resources.take::<R>(id)
    }

    pub fn get_resource_id<R: Resource>(&self) -> ResourceId {
        unsafe { self.as_unsafe_world_read_only().get_resource_id::<R>() }
    }

    pub fn resource<R: Resource>(&self) -> Res<R> {
        unsafe { self.as_unsafe_world_read_only().get_resource_ref::<R>() }
    }

    pub fn resource_mut<R: Resource>(&mut self) -> ResMut<R> {
        unsafe { self.as_unsafe_world().get_resource_mut_ref::<R>() }
    }

    // pub fn register_component<C: Component>(&mut self) -> ComponentId {
    //     self.components.register::<C>()
    // }
    //
    // pub fn register_component_by_id(
    //     &mut self,
    //     id: std::any::TypeId,
    //     name: &'static str,
    // ) -> ComponentId {
    //     self.components.register_by_id(id, name)
    // }

    pub fn get_component_id(&self, id: &std::any::TypeId) -> ComponentId {
        self.components.id(id)
    }

    pub fn get_component_ids(&self, ids: &[std::any::TypeId]) -> Vec<ComponentId> {
        let mut c_ids = Vec::new();
        for id in ids.iter() {
            c_ids.push(self.components.id(id))
        }

        c_ids
    }

    pub fn register_event<E: Event>(&mut self) {
        self.insert_resource(Events::<E>::new());
    }

    pub fn push_event<E: Event>(&mut self, event: E) {
        let mut events = self.resource_mut::<Events<E>>();
        events.push(event);
    }

    pub fn push_event_queue<E: Event>(&mut self, event_queue: Vec<E>) {
        let mut events = self.resource_mut::<Events<E>>();
        events.append(event_queue.into_iter());
    }

    pub fn entity(&self, entity: Entity) -> EntityRef<'_> {
        EntityRef::new(self, entity)
    }

    pub fn entity_mut(&mut self, entity: Entity) -> EntityMut<'_> {
        EntityMut::new(self, entity)
    }
}
