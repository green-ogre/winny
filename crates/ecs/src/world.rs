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

use crate::{
    Any, Archetype, Component, ComponentSet, Event, EventQueue, Resource, ResourceStorage, Table,
    TypeGetter, TypeId, TypeName,
};

use crate::entity::*;
use crate::storage::*;

pub struct NewEntityCommands {
    data: Box<dyn Bundle>,
}

impl NewEntityCommands {
    pub fn new<T: Bundle + 'static>(bundle: T) -> Self {
        Self {
            data: Box::new(bundle),
        }
    }

    pub fn commit(self, world: &mut World) {
        world.spawn_dyn(self.data);
    }
}

#[derive(Debug)]
struct InsertComponent {
    component: Box<dyn ComponentVec>,
    type_id: TypeId,
    storage_type: StorageType,
}

impl InsertComponent {
    pub fn new<T: Component + TypeGetter + Storage + Clone + Debug>(component: T) -> Self {
        Self {
            type_id: component.type_id(),
            storage_type: component.storage_type(),
            component: Box::new(RefCell::new(vec![component])),
        }
    }
}

pub struct EntityCommands {
    entity: Entity,
    insert: Vec<InsertComponent>,
    remove: Vec<TypeId>,
    despawn: bool,
}

impl EntityCommands {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            insert: vec![],
            remove: vec![],
            despawn: false,
        }
    }

    pub fn insert<T: Component + TypeGetter + Storage + Debug + Clone>(
        &mut self,
        component: T,
    ) -> &mut Self {
        self.insert.push(InsertComponent::new(component));
        self
    }

    pub fn remove<T: Component + TypeGetter>(&mut self) -> &mut Self {
        self.remove.push(TypeId::of::<T>());
        self
    }

    pub fn despawn(&mut self) {
        self.despawn = true;
    }

    pub fn commit(self, world: &mut World) {
        let res = world.apply_entity_commands(self);
        debug_assert!(res.is_ok());
    }
}

pub struct Commands {
    entity_commands: VecDeque<EntityCommands>,
    new_entity_commands: VecDeque<NewEntityCommands>,
}

impl Commands {
    pub fn new() -> Self {
        Self {
            entity_commands: VecDeque::new(),
            new_entity_commands: VecDeque::new(),
        }
    }

    pub fn spawn<T: Bundle + 'static>(&mut self, bundle: T) -> &NewEntityCommands {
        self.new_entity_commands
            .push_back(NewEntityCommands::new(bundle));

        self.new_entity_commands.back().unwrap()
    }

    pub fn get_entity(&mut self, entity: Entity) -> &mut EntityCommands {
        self.entity_commands.push_back(EntityCommands::new(entity));

        self.entity_commands.back_mut().unwrap()
    }

    pub fn sync(&mut self, world: &mut World) {
        self.new_entity_commands
            .drain(..)
            .for_each(|ec| ec.commit(world));

        self.entity_commands
            .drain(..)
            .for_each(|ec| ec.commit(world));
    }
}

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
        let comp_ids = bundle.ids();

        let arch_id = match self
            .archetypes
            .iter()
            .find(|arch| arch.contains_id_set(&comp_ids))
            .map(|arch| arch.id)
        {
            Some(arch_id) => {
                let _ = bundle.push_storage(&mut self.tables[self.archetypes[arch_id].table_id]);
                self.tables[self.archetypes[arch_id].table_id].len += 1;

                arch_id
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

                arch_id
            }
        };

        let table_row = TableRow(self.tables[self.archetypes[arch_id].table_id].len - 1);

        let entity = self.new_entity(
            self.archetypes[arch_id].table_id,
            table_row,
            self.archetypes[arch_id].id,
            self.archetypes[arch_id].entities.len(),
        );
        self.archetypes[arch_id].entities.push((entity, table_row));

        entity
    }

    // TODO: Not fixed
    pub fn spawn_dyn(&mut self, bundle: Box<dyn Bundle>) -> Entity {
        let comp_ids = bundle.ids();

        let arch_id = self
            .archetypes
            .iter()
            .find(|arch| arch.contains_id_set(&comp_ids))
            .map(|arch| arch.id)
            .unwrap_or_else(|| {
                let table_id = self.tables.len();
                let arch_id = self.archetypes.len();
                let component_ids = bundle.ids();
                let mut component_desc = FxHashMap::default();

                let component_storage_locations = bundle.storage_locations();
                for (id, location) in component_ids.iter().zip(component_storage_locations.iter()) {
                    component_desc.insert(*id, *location);
                }

                self.archetypes.push(Archetype::new(
                    arch_id,
                    table_id,
                    component_ids,
                    component_desc,
                    vec![],
                ));
                arch_id
            });

        let table_row = TableRow(self.tables[self.archetypes[arch_id].table_id].len);
        self.tables.push(Table::new_dyn(bundle));

        let entity = self.new_entity(
            self.archetypes[arch_id].table_id,
            table_row,
            self.archetypes[arch_id].id,
            self.archetypes[arch_id].entities.len(),
        );
        self.archetypes[arch_id].entities.push((entity, table_row));

        entity
    }

    pub fn new_entity(
        &mut self,
        table_id: usize,
        table_row: TableRow,
        archetype_id: usize,
        archetype_index: usize,
    ) -> Entity {
        match self.free_entities.pop() {
            Some(free_space) => {
                let meta = &mut self.entities[free_space as usize];
                meta.free = false;
                meta.generation += 1;

                meta.table_id = table_id;
                meta.table_row = table_row;
                meta.archetype_index = archetype_index;
                meta.archetype_id = archetype_id;

                Entity::new(meta.generation, free_space)
            }
            None => {
                self.entities.push(EntityMeta::new(
                    table_id,
                    table_row,
                    archetype_id,
                    archetype_index,
                ));
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

        self.remove_entity_from_table(entity)?;

        Ok(())
    }

    fn check_entity_generation(&self, entity: Entity) -> Result<(), ()> {
        if self.entities[entity.index() as usize].generation == entity.generation() {
            return Ok(());
        }

        Err(())
    }

    fn remove_entity_from_table(&mut self, entity: Entity) -> Result<(), ()> {
        let meta = self.get_entity(entity).ok_or(())?;

        self.tables[meta.table_id].len -= 1;

        let changed_table_row = TableRow(self.tables[meta.table_id].len);
        for v in self.tables[meta.table_id].storage.iter_mut() {
            v.swap_remove(meta.table_row.0)?;
        }

        if let Some(moved_entity) = self.archetypes[meta.archetype_id]
            .entities
            .iter()
            .rev()
            .find(|(_, table_row)| *table_row == changed_table_row)
            .map(|(e, _)| e)
        {
            let changed_meta = self.get_entity_mut(*moved_entity).ok_or(())?;
            changed_meta.table_row = meta.table_row;
        } else if let Some(moved_entity) = self
            .archetypes
            .iter()
            .filter(|arch| arch.table_id == meta.table_id)
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
            changed_meta.table_row = changed_table_row;
        }

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
        let mut new_comp_set: Vec<_> = self.archetypes[meta.archetype_id]
            .component_desc
            .iter()
            .filter(|id| !commands.remove.contains(&id.0))
            .map(|(id, storage)| (*id, *storage))
            .collect();
        new_comp_set.append(
            &mut commands
                .insert
                .iter()
                .filter(|c| new_comp_set.contains(&(c.type_id, c.storage_type)))
                .map(|c| (c.type_id, c.storage_type))
                .collect(),
        );

        let new_set: Vec<_> = new_comp_set.iter().map(|(id, _)| id.clone()).collect();
        let arch_id = self
            .archetypes
            .iter()
            .find(|arch| arch.contains_id_set(&new_set))
            .map(|arch| arch.id)
            .unwrap_or_else(|| {
                let table_id = self.tables.len();
                let arch_id = self.archetypes.len();
                let mut component_desc = FxHashMap::default();
                for (id, storage) in new_comp_set.iter() {
                    component_desc.insert(*id, *storage);
                }

                self.archetypes.push(Archetype::new(
                    arch_id,
                    table_id,
                    new_set,
                    component_desc,
                    vec![],
                ));
                arch_id
            });

        let mut bundle: Vec<_> = self.tables[meta.table_id]
            .storage
            .iter()
            .filter(|vec| self.archetypes[arch_id].contains_id(&vec.stored_type_id()))
            .filter(|vec| {
                !commands
                    .insert
                    .iter()
                    .any(|c| c.type_id == vec.stored_type_id())
            })
            .map(|vec| vec.duplicate(meta.table_row.0).expect("valid index"))
            .collect();
        for insert_component in commands.insert.into_iter() {
            bundle.push(insert_component.component);
        }

        self.tables.push(Table::new(bundle));

        let table_row = TableRow(self.tables[self.archetypes[arch_id].table_id].len);
        let table_id = self.archetypes[arch_id].table_id;
        let archetype_id = self.archetypes[arch_id].id;
        let archetype_index = self.archetypes[arch_id].entities.len();

        self.remove_entity_from_table(commands.entity).unwrap();

        let meta = self.get_entity_mut(commands.entity).ok_or(()).unwrap();

        meta.table_row = table_row;
        meta.table_id = table_id;
        meta.archetype_id = archetype_id;
        meta.archetype_index = archetype_index;

        self.archetypes[arch_id]
            .entities
            .push((commands.entity, table_row));

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
