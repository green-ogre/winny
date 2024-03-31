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
    pub fn new<T: Bundle>(bundle: T) -> Self {
        Self {
            data: Box::new(bundle),
        }
    }

    pub fn commit(self, world: &mut World) {
        world.spawn(self.data);
    }
}

impl Bundle for Box<dyn Bundle> {
    fn push(self, table: &mut Table) -> Result<(), crate::IntoStorageError> {
        self.push(table)
    }

    fn into_storage(self) -> Vec<Box<dyn crate::ComponentVec>> {
        self.into_storage()
    }

    fn ids(&self) -> Vec<TypeId> {
        self.ids()
    }

    fn storage_locations(&self) -> Vec<StorageType> {
        self.storage_locations()
    }
}

// trait PhantomRemoveCommand {
//     fn remove(self, world: &mut World);
// }
//
// pub trait RemoveCommand {
//     fn remove(world: &mut World);
// }
//
// impl<T: RemoveCommand> PhantomRemoveCommand for PhantomData<T> {
//     fn remove(self) -> T {
//         T::remove()
//     }
// }

pub struct EntityCommands {
    entity: Entity,
    insert: Vec<Box<*mut Component>>,
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

    pub fn insert<T: Component>(&mut self, component: T) -> &mut Self {
        self.insert.push(Box::new(component));
        self
    }

    pub fn remove<T: Component + TypeGetter>(&mut self) -> &mut Self {
        self.remove.push(TypeId::of::<T>());
        self
    }

    pub fn commit(self, world: &mut World) {
        world.apply_entity_commands(self);
    }
}

pub struct Commands {
    entity_commands: VecDeque<EntityCommands>,
    new_entity_commands: VecDeque<NewEntityCommands>,
}

impl Commands {
    pub fn new(world: &World) -> Self {
        Self {
            entity_commands: VecDeque::new(),
            new_entity_commands: VecDeque::new(),
        }
    }

    pub fn spawn<T: Bundle>(&mut self, bundle: T) -> &NewEntityCommands {
        self.new_entity_commands
            .push_back(NewEntityCommands::new(bundle));

        self.new_entity_commands.back().unwrap()
    }

    pub fn get_entity(&mut self, entity: Entity) -> &mut EntityCommands {
        self.entity_commands.push_back(EntityCommands::new(entity));

        &mut self.entity_commands.back().unwrap()
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

        let arch = self
            .archetypes
            .iter_mut()
            .find(|arch| arch.contains_id_set(&comp_ids))
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
                self.archetypes.last_mut().unwrap()
            });

        let table_row = TableRow(self.tables[arch.table_id].len);
        self.tables.push(Table::new(bundle));
        bundle.push(&mut self.tables[arch.table_id]);

        let entity = self.new_entity(arch.table_id, table_row, arch.id, arch.entities.len());
        arch.entities.push((entity, table_row));

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

    fn get_entity(&mut self, entity: Entity) -> Option<&EntityMeta> {
        self.entities
            .get(entity.index() as usize)
            .and_then(|m| (entity.generation() == m.generation).then_some(m))
    }

    pub fn despawn(&mut self, entity: Entity) -> Result<(), ()> {
        let Some(meta) = self.get_entity(entity) else {
            return Ok(());
        };

        meta.free = true;
        self.free_entities.push(entity.index());

        self.remove_entity_from_table(entity, meta)?;

        Ok(())
    }

    fn remove_entity_from_table(&mut self, entity: Entity, meta: &EntityMeta) -> Result<(), ()> {
        let changed_table_row = TableRow(self.tables[meta.table_id].len - 1);
        for v in self.tables[meta.table_id].storage.iter() {
            v.swap_remove(meta.table_row.0)?;
        }

        if let Some(changed_meta) = self.archetypes[meta.archetype_id]
            .entities
            .iter()
            .rev()
            .find(|(_, table_row)| *table_row == changed_table_row)
            .and_then(|(e, _)| self.get_entity(*e))
        {
            // WARN: idk if this actually changes the value
            changed_meta.table_row = changed_table_row;
        } else if let Some(changed_meta) = self
            .archetypes
            .iter()
            .filter(|arch| arch.table_id == meta.table_id)
            .map(|arch| {
                arch.entities
                    .iter()
                    .rev()
                    .find(|(_, table_row)| *table_row == changed_table_row)
                    .and_then(|(e, _)| self.get_entity(*e))
            })
            .exactly_one()
            .map_err(|_| ())?
        {
            changed_meta.table_row = changed_table_row;
        }

        Ok(())
    }

    pub fn apply_entity_commands(&mut self, commands: EntityCommands) -> Result<(), ()> {
        if commands.despawn {
            self.despawn(commands.entity);
            return Ok(());
        }

        if commands.insert.is_empty() && commands.remove.is_empty() {
            return Ok(());
        }

        let meta = self.get_entity(commands.entity).ok_or(())?;
        let mut new_comp_set: Vec<_> = self.archetypes[meta.archetype_id]
            .component_desc
            .into_iter()
            .filter(|id| !commands.remove.contains(&id.0))
            .collect();
        new_comp_set.append(
            &mut commands
                .insert
                .iter()
                .map(|c| (c.type_id(), c.storage_type())),
        );

        // TODO: needs to look for tables with same storage
        let arch = self
            .archetypes
            .iter()
            .find(|arch| arch.contains_id_set(&new_comp_set))
            .unwrap_or_else(|| {
                let table_id = self.tables.len();
                let arch_id = self.archetypes.len();
                let component_ids = new_comp_set.iter().map(|(id, _)| id).collect();
                let mut component_desc = new_comp_set;

                self.archetypes.push(Archetype::new(
                    arch_id,
                    table_id,
                    component_ids,
                    component_desc,
                    vec![],
                ));
                self.archetypes.last_mut().unwrap()
            });

        let mut bundle: Vec<_> = self.tables[meta.table_id]
            .storage
            .iter()
            .filter(|vec| !arch.contains_id(&vec.stored_type_id()))
            .map(|vec| vec.duplicate())
            .collect();
        for component in commands.insert.iter() {
            bundle.push(Box::new(RefCell::new(vec![component])));
        }

        let old_table = &mut self.tables[meta.table_id];
        self.tables.push(Table::new(bundle));
        let new_table = &mut self.tables[meta.table_id];
        debug_assert_ne!(old_table, new_table);

        let table_row = TableRow(self.tables[arch.table_id].len);
        let table_id = arch.table_id;
        let archetype_id = arch.id;
        let archetype_index = arch.entities.len();

        meta.table_row = table_row;
        meta.table_id = table_id;
        meta.archetype_id = archetype_id;
        meta.archetype_index = archetype_index;

        arch.entities.push((commands.entity, table_row));

        Ok(())
    }

    pub fn insert_resource<T: Debug + Resource + TypeGetter>(&mut self, resource: T) {
        let new_resource: RefCell<T> = RefCell::new(resource);
        self.resources.push(Box::new(new_resource));
    }

    pub fn register_event<T: Event + TypeGetter>(&mut self) {
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
