pub mod commands;
pub mod entity;
pub mod unsafe_world;

pub use commands::*;
pub use entity::*;
use logger::error;

use core::panic;
use std::any::TypeId;
use std::collections::HashMap;
use std::{collections::VecDeque, fmt::Debug};

use crate::{Archetype, Component, Event, Events, Res, ResMut, Resource, Resources, Table};

use crate::storage::*;

pub use self::unsafe_world::UnsafeWorldCell;

#[derive(Debug)]
pub struct World {
    pub archetypes: Archetypes,
    pub tables: Tables,
    pub resources: Resources,

    component_ids: HashMap<TypeId, ComponentId>,
    pub resource_ids: HashMap<TypeId, ResourceId>,
    next_comp_id: usize,
    next_resource_id: usize,

    entities: Vec<EntityMeta>,
    free_entities: Vec<u32>,
}

impl Default for World {
    fn default() -> Self {
        World {
            archetypes: Archetypes::new(),
            entities: Vec::new(),
            tables: Tables::new(),

            // component_ids: fxhash::FxHashMap::default(),
            // resource_ids: fxhash::FxHashMap::default(),
            // event_ids: fxhash::FxHashMap::default(),
            component_ids: HashMap::default(),
            resource_ids: HashMap::default(),
            next_comp_id: 0,
            next_resource_id: 0,

            resources: Resources::new(),
            free_entities: Vec::new(),
        }
    }
}

struct UnstoredEntity {
    storage: SparseSet<ComponentId, DumbVec>,
}

impl World {
    pub unsafe fn as_unsafe_world<'w>(&self) -> UnsafeWorldCell<'w> {
        UnsafeWorldCell::new(self)
    }

    pub fn spawn<T: Bundle>(&mut self, bundle: T) -> Entity {
        self.register_components(&bundle.type_ids());
        let meta_location = self.find_or_create_storage(bundle);

        let entity = self.new_entity(meta_location);

        let table_row = TableRow(
            self.tables
                .get(meta_location.table_id)
                .expect("must exist")
                .depth()
                - 1,
        );

        let arch_index = self
            .archetypes
            .get_mut(meta_location.archetype_id)
            .new_entity(ArchEntity::new(entity, table_row));

        self.entities[entity.index() as usize]
            .location
            .archetype_index = arch_index;

        entity
    }

    pub fn register_components(&mut self, type_ids: &[TypeId]) {
        for id in type_ids.iter() {
            if !self.component_ids.contains_key(id) {
                self.component_ids
                    .insert(*id, ComponentId::new(self.next_comp_id));
                self.next_comp_id += 1;
            }
        }
    }

    fn find_or_create_storage<T: Bundle>(&mut self, bundle: T) -> MetaLocation {
        let world = unsafe { self.as_unsafe_world() };
        if let Some(arch) = self.archetypes.get_from_type_ids(&mut bundle.type_ids()) {
            self.tables
                .get_mut(arch.table_id)
                .expect("must exist")
                .new_entity(bundle, world);

            MetaLocation::new(arch.table_id, arch.id, ArchIndex::new(usize::MAX))
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
                Archetype::new(arch_id, table_id, component_ids, component_desc),
            );

            let arch = self.archetypes.get_mut(arch_id);

            MetaLocation::new(arch.table_id, arch.id, ArchIndex::new(usize::MAX))
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

    pub fn despawn(&mut self, entity: Entity) {
        let Some(meta) = self.get_entity_mut(entity) else {
            logger::error!("Tried to despawn invalid entity");
            return;
        };

        meta.free = true;
        self.free_entities.push(entity.index());

        self.remove_entity(entity);
    }

    fn check_entity_generation(&self, entity: Entity) -> Result<(), ()> {
        if self.entities[entity.index() as usize].generation == entity.generation() {
            return Ok(());
        }

        Err(())
    }

    fn remove_entity(&mut self, _entity: Entity) {
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
    }

    pub fn apply_entity_commands(
        &mut self,
        entity: Entity,
        new_ids: Vec<TypeId>,
        remove_ids: Vec<TypeId>,
        insert_ids: Vec<TypeId>,
        mut insert: Vec<InsertComponent>,
    ) {
        // Checked by commands before called
        let entity_meta = self.get_entity(entity).unwrap();

        self.register_components(&new_ids);

        let remove_component_ids = self.get_component_ids(&remove_ids);
        let component_ids = self.get_component_ids(&new_ids);
        let current_table = unsafe { self.as_unsafe_world().read_and_write() }
            .tables
            .get_mut(entity_meta.location.table_id)
            .unwrap();

        let current_table_row = self
            .archetypes
            .get(entity_meta.location.archetype_id)
            .unwrap()
            .get_entity_table_row(entity_meta);

        let meta = if let Some(arch) = self
            .archetypes
            .get_from_type_ids(new_ids.clone().as_mut_slice())
        {
            for (component_id, type_id) in component_ids.iter().zip(new_ids.iter()) {
                remove_entity_from_old_storage_and_put_in_new(
                    {
                        if insert_ids.contains(type_id) {
                            &mut insert
                                .iter_mut()
                                .find(|i| i.type_id == *type_id)
                                .unwrap()
                                .component
                        } else {
                            unsafe {
                                current_table
                                    .column_mut(*component_id)
                                    .unwrap()
                                    .storage_mut()
                            }
                        }
                    },
                    {
                        let table = self.tables.get_mut(arch.table_id).expect("must exist");
                        let column = table.column_mut(*component_id).unwrap();
                        unsafe { column.storage_mut() }
                    },
                    current_table_row.0,
                );
            }

            for _component_id in remove_component_ids.iter() {
                todo!();
            }

            let table_row = TableRow(
                self.tables
                    .get(arch.table_id)
                    .expect("must exist")
                    .depth()
                    .saturating_sub(1),
            );
            let arch_index = arch.new_entity(ArchEntity::new(entity, table_row));

            MetaLocation::new(arch.table_id, arch.id, arch_index)
        } else {
            let table_id = self.tables.new_id();
            let arch_id = self.archetypes.new_id();
            let component_desc = SparseSet::new();

            // TODO: Component desc

            let mut storages = current_table
                .storage
                .iter()
                .filter(|(id, _)| component_ids.contains(id))
                .map(|(id, c)| (*id, unsafe { c.storage() }.to_new_with_capacity(1)))
                .collect::<Vec<_>>();
            storages.extend(
                insert
                    .into_iter()
                    .map(|i| (self.get_component_ids(&[i.type_id])[0], i.component)),
            );

            for (component_id, type_id) in component_ids.iter().zip(new_ids.iter()) {
                if !insert_ids.contains(type_id) {
                    remove_entity_from_old_storage_and_put_in_new(
                        unsafe {
                            current_table
                                .column_mut(*component_id)
                                .unwrap()
                                .storage_mut()
                        },
                        {
                            storages
                                .iter_mut()
                                .find(|(id, _)| *id == *component_id)
                                .map(|(_, vec)| vec)
                                .unwrap()
                        },
                        current_table_row.0,
                    );
                }
            }

            self.tables.new_table(table_id, Table::new(storages));
            self.archetypes.new_archetype(
                arch_id,
                Archetype::new(arch_id, table_id, new_ids, component_desc),
            );

            let arch = self.archetypes.get_mut(arch_id);
            let table_row = TableRow(0);
            let arch_index = arch.new_entity(ArchEntity::new(entity, table_row));

            MetaLocation::new(arch.table_id, arch.id, arch_index)
        };

        self.archetypes
            .get_mut(entity_meta.location.archetype_id)
            .remove_entity(entity_meta.location.archetype_index);

        self.entities[entity.index() as usize].location = meta;
    }

    pub fn insert_resource<R: Resource>(&mut self, resource: R) {
        unsafe { self.as_unsafe_world().insert_resource(resource) }
    }

    pub fn register_event<E: Event>(&mut self) {
        self.register_resource(std::any::TypeId::of::<Events<E>>());
        self.insert_resource(Events::<E>::new());
    }

    pub fn resource<R: Resource>(&self) -> Res<'_, R> {
        let type_id = std::any::TypeId::of::<R>();
        let id = self.resource_ids.get(&type_id).unwrap_or_else(|| {
            logger::error!(
                "Plugin: Resource [{}] is not registered",
                std::any::type_name::<R>().to_string()
            );
            panic!();
        });
        Res::new(unsafe { self.as_unsafe_world() }, *id)
    }

    pub fn resource_mut<R: Resource>(&self) -> ResMut<'_, R> {
        let type_id = std::any::TypeId::of::<R>();
        let id = self.resource_ids.get(&type_id).unwrap_or_else(|| {
            logger::error!(
                "Plugin: Resource [{}] is not registered",
                std::any::type_name::<R>().to_string()
            );
            panic!();
        });
        ResMut::new(unsafe { self.as_unsafe_world() }, *id)
    }

    pub fn get_component_ids(&self, type_ids: &[TypeId]) -> Vec<ComponentId> {
        let mut component_ids = Vec::with_capacity(type_ids.len());
        for t in type_ids.iter() {
            component_ids.push(
                *self
                    .component_ids
                    .get(t)
                    .ok_or_else(|| error!("Failed to get component id: {:?}", t))
                    .unwrap(),
            )
        }

        component_ids
    }

    pub fn register_resource(&mut self, type_id: TypeId) -> ResourceId {
        if let Some(id) = self.resource_ids.get(&type_id) {
            *id
        } else {
            let new_resource_id = self.resources.new_id();
            self.resource_ids.insert(type_id, new_resource_id);
            new_resource_id
        }
    }

    pub fn get_resource_id<R: Resource>(&mut self) -> ResourceId {
        let type_id = std::any::TypeId::of::<R>();

        if let Some(id) = self.resource_ids.get(&type_id) {
            *id
        } else {
            self.register_resource(type_id)
        }
    }

    pub fn print_size(&self) {
        let archetypes = self.archetypes.len();
        let tables = self.tables.len();
        let resources = self.resources.len();
        let component_ids = self.component_ids.len();
        let resource_ids = self.resource_ids.len();
        let entities = self.entities.len();

        logger::info!("archetypes: {archetypes}, tables: {tables}, resources: {resources}, component_ids: {component_ids}, resource_ids: {resource_ids}, entities: {entities}");
    }
}

fn remove_entity_from_old_storage_and_put_in_new(
    src: &mut DumbVec,
    dst: &mut DumbVec,
    index: usize,
) {
    src.remove_and_push_other(dst, index);
}
