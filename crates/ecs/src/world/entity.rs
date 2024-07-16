use std::{fmt::Debug, sync::atomic::AtomicU32};

use crate::{
    ArchId, ArchRow, Archetype, Bundle, SparseArray, SparseArrayIndex, SwapEntity, TableId,
    TableRow, World,
};

use util::tracing::{error, trace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entity(u64);

impl Entity {
    pub fn new(generation: u32, storage_index: u32) -> Self {
        Self(((generation as u64) << 32) | storage_index as u64)
    }

    pub fn generation(&self) -> u32 {
        (self.0 >> 32) as u32
    }

    fn index(&self) -> usize {
        self.0 as u32 as usize
    }
}

impl SparseArrayIndex for Entity {
    fn index(&self) -> usize {
        self.index()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EntityMeta {
    pub location: MetaLocation,
    pub generation: u32,
    pub free: bool,
}

impl EntityMeta {
    pub fn new(location: MetaLocation) -> Self {
        Self {
            location,
            generation: 0,
            free: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetaLocation {
    pub table_id: TableId,
    pub archetype_id: ArchId,
    pub table_row: TableRow,
    pub arch_row: ArchRow,
}

impl MetaLocation {
    pub fn new(
        table_id: TableId,
        archetype_id: ArchId,
        table_row: TableRow,
        arch_row: ArchRow,
    ) -> Self {
        Self {
            table_id,
            archetype_id,
            table_row,
            arch_row,
        }
    }
}

pub struct Entities {
    entities: SparseArray<Entity, EntityMeta>,
    reserve_index: AtomicU32,
    free_entities: Vec<u32>,
}

impl Debug for Entities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entities = self.entities.iter().collect::<Vec<_>>();
        f.debug_struct("Entities")
            .field("entities", &entities)
            .field("reserve_index", &self.reserve_index)
            .field("free_entities", &self.free_entities)
            .finish()
    }
}

impl Default for Entities {
    fn default() -> Self {
        Self {
            entities: SparseArray::new(),
            reserve_index: AtomicU32::new(0),
            free_entities: Vec::new(),
        }
    }
}

impl Entities {
    pub fn new() -> Self {
        Self {
            entities: SparseArray::new(),
            reserve_index: AtomicU32::new(0),
            free_entities: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn meta(&self, entity: Entity) -> Option<&EntityMeta> {
        self.is_valid(entity).then(|| self.entities.get(&entity))?
    }

    pub fn meta_mut(&mut self, entity: Entity) -> Option<&mut EntityMeta> {
        self.is_valid(entity)
            .then(|| self.entities.get_mut(&entity))?
    }

    pub fn set_location(&mut self, entity: Entity, location: MetaLocation) {
        assert!(self.is_valid(entity));

        // entity is valid
        unsafe { self.entities.get_mut_unchecked(&entity).location = location }
    }

    pub fn spawn(
        &mut self,
        table_id: TableId,
        arch_id: ArchId,
        table_row: TableRow,
        arch_row: ArchRow,
    ) -> Entity {
        let location = MetaLocation::new(table_id, arch_id, table_row, arch_row);
        match self.free_entities.pop() {
            Some(free_space) => {
                let meta = &mut unsafe {
                    self.entities
                        .get_mut(&Entity::new(0, free_space))
                        .unwrap_unchecked()
                };
                let entity = Entity::new(meta.generation, free_space);
                meta.free = false;
                meta.generation += 1;
                meta.location = location;

                self.reserve_index.store(
                    self.entities.len() as u32,
                    std::sync::atomic::Ordering::Relaxed,
                );

                entity
            }
            None => {
                let index = self.entities.push(EntityMeta::new(location));

                self.reserve_index.store(
                    self.entities.len() as u32,
                    std::sync::atomic::Ordering::Relaxed,
                );

                Entity::new(0, index as u32)
            }
        }
    }

    pub fn spawn_at(
        &mut self,
        entity: Entity,
        table_id: TableId,
        arch_id: ArchId,
        table_row: TableRow,
        arch_row: ArchRow,
    ) {
        let location = MetaLocation::new(table_id, arch_id, table_row, arch_row);
        self.entities
            .insert(entity.index(), EntityMeta::new(location));

        self.reserve_index.store(
            self.entities.len() as u32,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    pub fn reserve(&self) -> Entity {
        let index = self
            .reserve_index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Entity::new(0, index)
    }

    pub fn despawn(&mut self, entity: Entity) {
        let Some(meta) = self.meta_mut(entity) else {
            panic!("reference to invalid entity");
        };
        meta.free = true;
        self.free_entities.push(entity.index() as u32);
    }

    fn is_valid(&self, entity: Entity) -> bool {
        self.entities
            .get(&entity)
            .is_some_and(|e| !e.free && e.generation == entity.generation())
    }
}

pub struct EntityRef<'w> {
    world: &'w World,
    entity: Entity,
}

impl<'w> EntityRef<'w> {
    pub fn new(world: &'w World, entity: Entity) -> Self {
        Self { world, entity }
    }
}

pub struct EntityMut<'w> {
    world: &'w mut World,
    entity: Entity,
}

impl<'w> EntityMut<'w> {
    pub fn new(world: &'w mut World, entity: Entity) -> Self {
        Self { world, entity }
    }

    // TODO: lots of cloning vecs here
    pub fn insert<B: Bundle>(&mut self, bundle: B) {
        if let Some(meta) = self.world.entities.meta(self.entity).cloned() {
            trace!("meta found: {:?}", meta);
            let old_arch_id = meta.location.archetype_id;
            let old_arch = self.world.archetypes.get_mut(old_arch_id).unwrap();
            let old_table_row = meta.location.table_row;
            let old_table_id = meta.location.table_id;
            let old_arch_row = meta.location.arch_row;

            match old_arch.swap_remove_entity(meta.location.arch_row) {
                SwapEntity::Swap => unsafe {
                    self.world
                        .entities
                        .set_location(old_arch.get_entity_unchecked(old_arch_row), meta.location);
                },
                SwapEntity::Pop => (),
            }

            let mut old_arch_type_ids: Vec<_> = old_arch.type_ids.clone().into();
            let mut bundle_type_ids = B::type_ids();
            let mut new_type_ids =
                Vec::with_capacity(bundle_type_ids.len() + old_arch_type_ids.len());
            new_type_ids.append(&mut old_arch_type_ids);
            new_type_ids.append(&mut bundle_type_ids);
            new_type_ids.sort();

            let world = unsafe { self.world.as_unsafe_world() };
            let arch = unsafe {
                if let Some(arch) = world
                    .archetypes_mut()
                    .get_mut_from_type_ids(new_type_ids.clone().into_boxed_slice())
                {
                    trace!("archetype found: {:?}", arch.arch_id);
                    arch
                } else {
                    trace!("no archteype found, creating arch + table");
                    let table = world.tables().get_unchecked(old_table_id).clone_empty();
                    let table_id = world.tables_mut().push(table);
                    let arch_id = world
                        .archetypes_mut()
                        .push(Archetype::new(table_id, new_type_ids));
                    world.archetypes_mut().get_mut_unchecked(arch_id)
                }
            };

            unsafe {
                world.insert_entity_into_world(self.entity, arch, |arch| {
                    bundle.push_storage(world, arch.table_id);
                    world.transfer_table_row(old_table_row, old_table_id, arch.table_id);
                });
            }
        } else {
            trace!("entity is reserved, spawning bundle");
            unsafe {
                self.world
                    .as_unsafe_world()
                    .spawn_bundle_with_entity::<B>(self.entity, bundle)
            };
        }
    }

    pub fn remove<B: Bundle>(&mut self) {
        let Some(meta) = self.world.entities.meta(self.entity).cloned() else {
            error!("meta not found");
            panic!();
        };

        unsafe { B::register_components(self.world.as_unsafe_world()) };
        let old_arch_id = meta.location.archetype_id;
        let old_arch = self.world.archetypes.get_mut(old_arch_id).unwrap();
        let old_table_row = meta.location.table_row;
        let old_table_id = meta.location.table_id;
        let old_arch_row = meta.location.arch_row;

        match old_arch.swap_remove_entity(meta.location.arch_row) {
            SwapEntity::Swap => unsafe {
                self.world
                    .entities
                    .set_location(old_arch.get_entity_unchecked(old_arch_row), meta.location);
            },
            SwapEntity::Pop => (),
        }

        let mut new_type_ids: Vec<_> = old_arch.type_ids.clone().into();
        let bundle_type_ids = B::type_ids();
        new_type_ids.retain(|t| !bundle_type_ids.contains(t));

        let bundle_component_ids = self.world.get_component_ids(&bundle_type_ids);

        let world = unsafe { self.world.as_unsafe_world() };
        let arch = unsafe {
            if let Some(arch) = world
                .archetypes_mut()
                .get_mut_from_type_ids(new_type_ids.clone().into_boxed_slice())
            {
                trace!("archetype found: {:?}", arch.arch_id);
                arch
            } else {
                trace!("no archteype found, creating arch + table");
                let table = world
                    .tables_mut()
                    .get_unchecked(old_table_id)
                    .clone_empty_if(|id, _| !bundle_component_ids.contains(id));
                let table_id = world.tables_mut().push(table);
                let arch_id = world
                    .archetypes_mut()
                    .push(Archetype::new(table_id, new_type_ids));
                world.archetypes_mut().get_mut_unchecked(arch_id)
            }
        };

        unsafe {
            world.insert_entity_into_world(self.entity, arch, |arch| {
                world.transfer_table_row_if(old_table_row, old_table_id, arch.table_id, |id, _| {
                    !bundle_component_ids.contains(id)
                });
            });
        }
    }

    pub fn despawn(self) {
        let Some(meta) = self.world.entities.meta(self.entity).cloned() else {
            trace!("entity uninitialized, noop");
            return;
        };

        let table_id = meta.location.table_id;
        let archetype_id = meta.location.archetype_id;
        let table_row = meta.location.table_row;
        let arch_row = meta.location.arch_row;

        let archetype = self
            .world
            .archetypes
            .get_mut(archetype_id)
            .expect("valid entity");
        let table = self.world.tables.get_mut(table_id).expect("valid entity");

        if SwapEntity::Swap == archetype.swap_remove_entity(arch_row) {
            table.swap_remove_row(table_row);
            let swapped_entity = archetype.get_entity(arch_row).unwrap();
            trace!(
                "swapping entities: {:?} & {:?}",
                self.entity,
                swapped_entity
            );
            let new_location = MetaLocation {
                table_id,
                archetype_id,
                table_row,
                arch_row,
            };
            self.world
                .entities
                .set_location(swapped_entity, new_location);
        } else {
            trace!("no swap: EntityPop");
        }

        self.world.entities.despawn(self.entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use ecs_macro::InternalComponent;
    use tracing_test::traced_test;
    use util::tracing;

    #[derive(Debug, InternalComponent)]
    struct Health(u32);

    #[derive(Debug, InternalComponent)]
    struct Weight(u32);

    #[derive(Debug, InternalComponent, PartialEq, Eq, Hash)]
    struct Size(u32);

    #[traced_test]
    #[test]
    fn it_works() {
        let mut world = World::default();
        let e2 = world.spawn((Health(0), Weight(0)));
        let e = world.spawn((Health(1), Weight(1)));

        let mut e = world.entity_mut(e);
        e.insert(Size(1));

        let mut e2 = world.entity_mut(e2);
        e2.insert(Size(0));
        e2.remove::<Weight>();
        e2.despawn();

        assert!(world.archetypes.len() == 3);
        assert!(world.entities.len() == 2);

        println!("{:#?}", world.entities);
        println!("{:#?}", world.archetypes);
    }
}
