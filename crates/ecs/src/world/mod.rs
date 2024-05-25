pub mod commands;
pub mod entity;
pub mod unsafe_world;

pub use commands::*;
pub use entity::*;
use logger::error;

use std::collections::HashMap;
use std::{collections::VecDeque, fmt::Debug};

use crate::{
    Any, Archetype, Component, Event, EventId, Events, Res, ResMut, Resource, Resources, Table,
    TypeGetter, TypeId,
};

use crate::storage::*;

pub use self::unsafe_world::UnsafeWorldCell;

#[derive(Debug)]
pub struct World {
    pub archetypes: Archetypes,
    pub tables: Tables,
    pub events: Events,
    pub resources: Resources,

    //component_ids: fxhash::FxHashMap<TypeId, ComponentId>,
    //resource_ids: fxhash::FxHashMap<TypeId, ResourceId>,
    //event_ids: fxhash::FxHashMap<TypeId, EventId>,
    component_ids: HashMap<TypeId, ComponentId>,
    resource_ids: HashMap<TypeId, ResourceId>,
    event_ids: HashMap<TypeId, EventId>,
    next_comp_id: usize,

    entities: Vec<EntityMeta>,
    free_entities: Vec<u32>,
}

impl Default for World {
    fn default() -> Self {
        World {
            archetypes: Archetypes::new(),
            entities: Vec::new(),
            tables: Tables::new(),
            events: Events::new(),

            // component_ids: fxhash::FxHashMap::default(),
            // resource_ids: fxhash::FxHashMap::default(),
            // event_ids: fxhash::FxHashMap::default(),
            component_ids: HashMap::default(),
            resource_ids: HashMap::default(),
            event_ids: HashMap::default(),
            next_comp_id: 0,

            resources: Resources::new(),
            free_entities: Vec::new(),
        }
    }
}

impl World {
    pub unsafe fn as_unsafe_world<'w>(&self) -> UnsafeWorldCell<'w> {
        UnsafeWorldCell::new(self)
    }

    pub fn spawn<T: Bundle>(&mut self, bundle: T) -> Entity {
        self.register_component_ids(&bundle.type_ids());
        let meta_location = self.find_or_create_storage(bundle);

        let entity = self.new_entity(meta_location);
        self.archetypes
            .get_mut(meta_location.archetype_id)
            .entities
            .push(ArchEntity::new(entity, meta_location.table_row));

        entity
    }

    fn register_component_ids(&mut self, type_ids: &[TypeId]) {
        for id in type_ids.iter() {
            if !self.component_ids.contains_key(id) {
                self.component_ids
                    .insert(*id, ComponentId::new(self.next_comp_id));
                self.next_comp_id += 1;
            }
        }
    }

    fn find_or_create_storage<T: Bundle>(&mut self, bundle: T) -> MetaLocation {
        if let Some(arch) = self.archetypes.get_from_type_ids(&mut bundle.type_ids()) {
            let world = unsafe { self.as_unsafe_world() };
            self.tables
                .get_mut(arch.table_id)
                .expect("must exist")
                .new_entity(bundle, world);

            MetaLocation::new(
                arch.table_id,
                TableRow(self.tables.get(arch.table_id).expect("must exist").depth() - 1),
                arch.id,
                arch.entities.len().saturating_sub(1),
            )
        } else {
            let table_id = self.tables.new_id();
            let arch_id = self.archetypes.new_id();
            let component_ids = bundle.type_ids();
            let mut component_desc = SparseSet::new();

            let component_storage_locations = bundle.storage_locations();
            for (id, location) in bundle
                .component_ids(unsafe { self.as_unsafe_world() })
                .into_iter()
                .zip(component_storage_locations.iter())
            {
                component_desc.insert(id, *location);
            }

            self.tables.new_table(
                table_id,
                Table::from_bundle(bundle, unsafe { self.as_unsafe_world() }),
            );
            self.archetypes.new_archetype(
                arch_id,
                Archetype::new(arch_id, table_id, component_ids, component_desc, vec![]),
            );

            let arch = self.archetypes.get(arch_id).expect("just constructed");

            MetaLocation::new(
                arch.table_id,
                TableRow(self.tables.get(arch.table_id).expect("must exist").depth() - 1),
                arch.id,
                arch.entities.len().saturating_sub(1),
            )
        }
    }

    pub fn new_entity(&mut self, meta_location: MetaLocation) -> Entity {
        match self.free_entities.pop() {
            Some(free_space) => {
                let meta = &mut self.entities[free_space as usize];
                meta.free = false;
                meta.generation += 1;
                meta.location = meta_location;

                Entity::new(meta.generation, free_space)
            }
            None => {
                self.entities.push(EntityMeta::new(meta_location));
                Entity::new(0, self.entities.len() as u32 - 1)
            }
        }
    }

    pub fn get_entity(&self, entity: Entity) -> Option<EntityMeta> {
        self.entities
            .get(entity.index() as usize)
            .and_then(|m| (entity.generation() == m.generation).then_some(m.clone()))
    }

    pub fn get_entity_mut(&mut self, entity: Entity) -> Option<&mut EntityMeta> {
        self.entities
            .get_mut(entity.index() as usize)
            .and_then(|m| (entity.generation() == m.generation).then_some(m))
    }

    pub fn despawn(&mut self, entity: Entity) -> Result<(), ()> {
        let Some(meta) = self
            .entities
            .get_mut(entity.index() as usize)
            .and_then(|m| (entity.generation() == m.generation).then_some(m))
        else {
            return Ok(());
        };

        meta.free = true;
        self.free_entities.push(entity.index());

        self.remove_entity(entity)?;

        Ok(())
    }

    fn check_entity_generation(&self, entity: Entity) -> Result<(), ()> {
        if self.entities[entity.index() as usize].generation == entity.generation() {
            return Ok(());
        }

        Err(())
    }

    fn remove_entity(&mut self, _entity: Entity) -> Result<(), ()> {
        // let meta = self.get_entity(entity).ok_or(())?;

        // self.tables[meta.location.table_id].len -= 1;

        // let changed_table_row = TableRow(self.tables[meta.location.table_id].len);
        // for v in self.tables[meta.location.table_id].storage.iter_mut() {
        //     v.swap_remove(meta.location.table_row.0)?;
        // }

        // if changed_table_row == meta.location.table_row {
        //     self.archetypes[meta.location.archetype_id]
        //         .entities
        //         .swap_remove(meta.location.archetype_index);
        //     return Ok(());
        // }

        // if let Some(moved_entity) = self.archetypes[meta.location.archetype_id]
        //     .entities
        //     .iter()
        //     .rev()
        //     .find(|a_entity| a_entity.row == changed_table_row)
        //     .map(|a_entity| a_entity.entity)
        // {
        //     let changed_meta = self.get_entity_mut(moved_entity).ok_or(())?;
        //     changed_meta.location = meta.location;
        // } else if let Some(moved_entity) = self
        //     .archetypes
        //     .iter()
        //     .filter(|arch| arch.table_id == meta.location.table_id)
        //     .map(|arch| {
        //         arch.entities
        //             .iter()
        //             .rev()
        //             .find(|a_entity| a_entity.row == changed_table_row)
        //             .map(|a_entity| a_entity.entity)
        //     })
        //     .exactly_one()
        //     .map_err(|_| ())?
        // {
        //     let changed_meta = self.get_entity_mut(moved_entity).ok_or(())?;
        //     changed_meta.location.table_row = changed_table_row;
        // }

        // self.archetypes[meta.location.archetype_id]
        //     .entities
        //     .swap_remove(meta.location.archetype_index);

        // self.archetypes[meta.location.archetype_id].entities[meta.location.archetype_index].row =
        //     meta.location.table_row;

        Ok(())
    }

    pub fn insert_resource<R: Resource>(&mut self, resource: R) {
        let type_id = R::type_id();
        if !self.resource_ids.contains_key(&type_id) {
            self.resource_ids.insert(type_id, self.resources.new_id());
        }
        unsafe { self.as_unsafe_world().insert_resource(resource) }
    }

    pub fn register_event<E: Event>(&mut self) {
        let id = self.events.new_id();
        self.event_ids.insert(E::type_id(), id);
        self.events.insert::<E>(id);
    }

    pub fn resource<R: Resource>(&self) -> Res<'_, R> {
        Res::new(unsafe { self.as_unsafe_world() })
    }

    pub fn resource_mut<R: Resource>(&self) -> ResMut<'_, R> {
        ResMut::new(unsafe { self.as_unsafe_world() })
    }

    pub fn flush_events(&mut self) {
        for event_queue in self.events.iter_mut() {
            event_queue.flush();
        }
    }

    pub fn get_component_ids(&self, type_ids: &[TypeId]) -> Result<Vec<ComponentId>, ()> {
        let mut component_ids = Vec::with_capacity(type_ids.len());
        for t in type_ids.iter() {
            component_ids.push(*self.component_ids.get(t).ok_or(()).map_err(|_| {
                error!("Component Id is not stored in world for type [{:?}]", t,);
                ()
            })?)
        }

        Ok(component_ids)
    }

    pub fn get_event_id(&self, type_id: TypeId) -> EventId {
        *self.event_ids.get(&type_id).expect("event is registered")
    }

    pub fn get_resource_id(&self, type_id: TypeId) -> ResourceId {
        *self
            .resource_ids
            .get(&type_id)
            .expect("resource is registered")
    }
}
