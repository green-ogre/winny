pub mod commands;
pub mod unsafe_world;

use commands::*;

use std::{
    borrow::{Borrow, BorrowMut},
    cell::{Ref, RefCell, RefMut},
    collections::VecDeque,
    fmt::Debug,
    marker::PhantomData,
    num::NonZeroUsize,
    ops::Deref,
};

use ecs_derive::TestTypeGetter;
use fxhash::FxHashMap;
use itertools::Itertools;
use logging::*;

use crate::{
    Any, Archetype, Component, ComponentSet, Event, EventQueue, Resource, ResourceStorage, Table,
    TypeGetter, TypeId, TypeName,
};

use crate::entity::*;
use crate::storage::*;

/*
 * TODO:
 *
 * - Add event types and resource types so that they can index into storage,
 *      instead of iterating over all types.
 * */

#[derive(Debug)]
pub struct World {
    pub archetypes: Vec<Archetype>,
    pub entities: Vec<EntityMeta>,
    pub free_entities: Vec<u32>,
    pub tables: Vec<Table>,

    pub resources: Vec<Box<dyn ResourceStorage>>,
    pub events: Vec<Box<dyn EventQueue>>,
}

impl Default for World {
    fn default() -> Self {
        World {
            archetypes: Vec::new(),
            entities: Vec::new(),
            free_entities: Vec::new(),
            tables: Vec::new(),

            resources: Vec::new(),
            events: Vec::new(),
        }
    }
}

impl World {
    pub fn spawn<T: Bundle>(&mut self, bundle: T) -> Entity {
        let meta_location = self.find_or_create_archetype(bundle);

        let entity = self.new_entity(meta_location);
        self.archetypes[meta_location.archetype_id]
            .entities
            .push((entity, meta_location.table_row));

        entity
    }

    pub fn spawn_box(&mut self, bundle: Box<dyn Bundle>) -> Entity {
        let meta_location = self.find_or_create_archetype_box(bundle);

        let entity = self.new_entity(meta_location);
        self.archetypes[meta_location.archetype_id]
            .entities
            .push((entity, meta_location.table_row));

        entity
    }

    fn find_or_create_archetype<T: Bundle>(&mut self, bundle: T) -> MetaLocation {
        let comp_ids = bundle.ids();

        match self
            .archetypes
            .iter()
            .find(|arch| arch.comp_set_eq(&comp_ids))
            .map(|arch| arch.id)
        {
            Some(arch_id) => {
                let _ = bundle.push_storage(&mut self.tables[self.archetypes[arch_id].table_id]);
                self.tables[self.archetypes[arch_id].table_id].len += 1;

                let arch = &self.archetypes[arch_id];
                MetaLocation::new(
                    arch.table_id,
                    TableRow(self.tables[self.archetypes[arch_id].table_id].len - 1),
                    arch.id,
                    arch.entities.len().saturating_sub(1),
                )
            }
            None => {
                let table_id = self.tables.len();
                let arch_id = self.archetypes.len();
                let component_ids = bundle.ids();
                let mut component_desc = FxHashMap::default();

                let component_storage_locations = bundle.storage_locations();
                for (id, location) in component_ids.iter().zip(component_storage_locations.iter()) {
                    component_desc.insert(*id, *location);
                }

                self.tables.push(Table::new(bundle));
                self.archetypes.push(Archetype::new(
                    arch_id,
                    table_id,
                    component_ids,
                    component_desc,
                    vec![],
                ));

                MetaLocation::new(table_id, TableRow(0), arch_id, 0)
            }
        }
    }

    fn find_or_create_archetype_box(&mut self, bundle: Box<dyn Bundle>) -> MetaLocation {
        let comp_ids = bundle.ids();

        match self
            .archetypes
            .iter()
            .find(|arch| arch.comp_set_eq(&comp_ids))
            .map(|arch| arch.id)
        {
            Some(arch_id) => {
                let _ =
                    bundle.push_storage_box(&mut self.tables[self.archetypes[arch_id].table_id]);
                self.tables[self.archetypes[arch_id].table_id].len += 1;

                let arch = &self.archetypes[arch_id];
                MetaLocation::new(
                    arch.table_id,
                    TableRow(self.tables[self.archetypes[arch_id].table_id].len - 1),
                    arch.id,
                    arch.entities.len().saturating_sub(1),
                )
            }
            None => {
                let table_id = self.tables.len();
                let arch_id = self.archetypes.len();
                let component_ids = bundle.ids();
                let mut component_desc = FxHashMap::default();

                let component_storage_locations = bundle.storage_locations();
                for (id, location) in component_ids.iter().zip(component_storage_locations.iter()) {
                    component_desc.insert(*id, *location);
                }

                self.tables.push(Table::new_box(bundle));
                self.archetypes.push(Archetype::new(
                    arch_id,
                    table_id,
                    component_ids,
                    component_desc,
                    vec![],
                ));

                MetaLocation::new(table_id, TableRow(0), arch_id, 0)
            }
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

    fn remove_entity(&mut self, entity: Entity) -> Result<(), ()> {
        let meta = self.get_entity(entity).ok_or(())?;

        self.tables[meta.location.table_id].len -= 1;

        let changed_table_row = TableRow(self.tables[meta.location.table_id].len);
        for v in self.tables[meta.location.table_id].storage.iter_mut() {
            v.swap_remove(meta.location.table_row.0)?;
        }

        if changed_table_row == meta.location.table_row {
            self.archetypes[meta.location.archetype_id]
                .entities
                .swap_remove(meta.location.archetype_index);
            return Ok(());
        }

        if let Some(moved_entity) = self.archetypes[meta.location.archetype_id]
            .entities
            .iter()
            .rev()
            .find(|(_, table_row)| *table_row == changed_table_row)
            .map(|(e, _)| e)
        {
            let changed_meta = self.get_entity_mut(*moved_entity).ok_or(())?;
            changed_meta.location = meta.location;
        } else if let Some(moved_entity) = self
            .archetypes
            .iter()
            .filter(|arch| arch.table_id == meta.location.table_id)
            .map(|arch| {
                arch.entities
                    .iter()
                    .rev()
                    .find(|(_, table_row)| *table_row == changed_table_row)
                    .map(|(e, _)| e)
            })
            .exactly_one()
            .map_err(|_| ())?
        {
            let changed_meta = self.get_entity_mut(*moved_entity).ok_or(())?;
            changed_meta.location.table_row = changed_table_row;
        }

        self.archetypes[meta.location.archetype_id]
            .entities
            .swap_remove(meta.location.archetype_index);

        self.archetypes[meta.location.archetype_id].entities[meta.location.archetype_index].1 =
            meta.location.table_row;

        Ok(())
    }

    pub fn apply_entity_commands(&mut self, commands: EntityCommands) -> Result<(), ()> {
        if commands.despawn {
            self.despawn(commands.entity)?;
            return Ok(());
        }

        if commands.insert.is_empty() && commands.remove.is_empty() {
            return Ok(());
        }

        let meta = self.get_entity(commands.entity).ok_or(())?;
        let mut bundle: Vec<_> = self.tables[meta.location.table_id]
            .storage
            .iter()
            .filter(|vec| !commands.remove.contains(&vec.stored_type_id()))
            .filter(|vec| {
                !commands
                    .insert
                    .iter()
                    .any(|c| c.type_id == vec.stored_type_id())
            })
            .map(|vec| {
                vec.duplicate(meta.location.table_row.0)
                    .expect("valid index")
            })
            .collect();
        for insert_component in commands.insert.into_iter() {
            bundle.push(insert_component.component);
        }

        self.remove_entity(commands.entity).unwrap();
        let new_location = self.find_or_create_archetype(bundle);

        {
            let meta = self.get_entity_mut(commands.entity).ok_or(())?;
            meta.location = new_location;
        }

        let meta = self.get_entity(commands.entity).ok_or(())?;
        self.archetypes[meta.location.archetype_id]
            .entities
            .push((commands.entity, meta.location.table_row));

        // info!("{:#?}", self);

        Ok(())
    }

    pub fn insert_resource<T: Debug + Resource + TypeGetter>(&mut self, resource: T) {
        let new_resource: RefCell<T> = RefCell::new(resource);
        self.resources.push(Box::new(new_resource));
    }

    pub fn register_event<T: Debug + Event + TypeGetter>(&mut self) {
        let new_event: RefCell<VecDeque<T>> = RefCell::new(VecDeque::new());
        self.events.push(Box::new(new_event));
    }

    pub fn resource<T: Resource + TypeGetter>(&self) -> Ref<'_, T> {
        for resource in self.resources.iter() {
            if let Some(resource) = resource.as_any().downcast_ref::<RefCell<T>>() {
                return resource.borrow();
            }
        }

        panic!("Resource does not exits: {:?}", TypeName::of::<T>());
    }

    pub fn resource_mut<T: Resource + TypeGetter>(&self) -> RefMut<T> {
        for resource in self.resources.iter() {
            if let Some(resource) = resource.as_any().downcast_ref::<RefCell<T>>() {
                return resource.borrow_mut();
            }
        }

        panic!("Resource does not exits: {:?}", TypeName::of::<T>());
    }

    pub fn flush_events(&mut self) {
        for event_queue in self.events.iter_mut() {
            event_queue.flush();
        }
    }
}
