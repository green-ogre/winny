use std::{fmt::Debug, sync::atomic::AtomicU32};

use crate::{
    ArchId, ArchRow, Bundle, ComponentMeta, SparseArray, SparseArrayIndex, SwapEntity, TableId,
    TableRow, UnsafeWorldCell, World,
};

use util::tracing::trace;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(u64);

impl Debug for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entity")
            .field("generation", &self.generation())
            .field("index", &self.index())
            .finish()
    }
}

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

    pub fn meta_maybe_free(&self, entity: Entity) -> Option<&EntityMeta> {
        self.entities.get(&entity)
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
        generation: u32,
        entity: Entity,
        table_id: TableId,
        arch_id: ArchId,
        table_row: TableRow,
        arch_row: ArchRow,
    ) {
        let location = MetaLocation::new(table_id, arch_id, table_row, arch_row);
        self.entities.insert(
            entity.index(),
            EntityMeta {
                free: false,
                generation,
                location,
            },
        );

        self.reserve_index.store(
            self.entities.len() as u32,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    // WARN: CANNOT MULTITHREAD
    pub fn reserve(&mut self) -> Entity {
        if let Some(free_entity) = self.free_entities.pop() {
            let meta = self.entities.get_mut(&Entity::new(0, free_entity)).unwrap();
            meta.generation += 1;
            Entity::new(meta.generation, free_entity)
        } else {
            let index = self
                .reserve_index
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Entity::new(0, index)
        }
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

    pub fn insert<B: Bundle>(&mut self, bundle: B) {
        if let Some(meta) = self.world.entities.meta_maybe_free(self.entity).cloned() {
            if meta.free {
                unsafe {
                    self.world
                        .as_unsafe_world()
                        .spawn_bundle_with_entity::<B>(self.entity, bundle)
                };
                return;
            }

            util::tracing::info!("meta found: {:?}", meta);

            let mut bundle_ids = Vec::new();
            B::component_meta(&mut self.world.components, &mut |meta| {
                bundle_ids.push(*meta);
            });
            self.verify_insert_action(meta, &bundle_ids);
            self.swap_remove_entity(meta);

            let old_arch = self
                .world
                .archetypes
                .get_mut(meta.location.archetype_id)
                .unwrap();

            let mut component_metas = bundle_ids.clone();
            component_metas.append(&mut old_arch.component_metas.clone().to_vec());
            component_metas.sort();
            let component_metas = component_metas.into_boxed_slice();

            let (arch_id, table_id) = unsafe {
                UnsafeWorldCell::find_or_create_storage::<B>(
                    meta,
                    component_metas,
                    &mut self.world.archetypes,
                    &mut self.world.tables,
                    &mut self.world.components,
                    &|_, _| true,
                    true,
                )
            };

            trace!("adding components: {:?}", bundle_ids);

            let table = unsafe { self.world.tables.get_mut_unchecked(table_id) };
            let mut bundle_ids = bundle_ids.iter();

            bundle.insert_components(&mut |component_ptr| {
                // bundle_components is the same order as bundle components
                if let Some(meta) = bundle_ids.next() {
                    unsafe {
                        table
                            .column_mut_unchecked(&meta.id)
                            .push_erased(component_ptr)
                    };
                }
            });

            let table_id = unsafe { self.world.archetypes.get_mut_unchecked(arch_id).table_id };
            trace!(
                "transfer table row: {:?}, {:?} - {:?}",
                meta.location.table_row,
                meta.location.table_id,
                table_id
            );
            unsafe {
                UnsafeWorldCell::transfer_table_row(
                    &mut self.world.tables,
                    meta.location.table_row,
                    meta.location.table_id,
                    table_id,
                )
            };

            trace!("update entity meta");
            unsafe {
                UnsafeWorldCell::insert_entity_into_world(
                    self.entity,
                    self.world.archetypes.get_mut_unchecked(arch_id),
                    &mut self.world.tables,
                    &mut self.world.entities,
                );
            }
        } else {
            util::tracing::info!("entity is reserved, spawning bundle");
            unsafe {
                self.world
                    .as_unsafe_world()
                    .spawn_bundle_with_entity::<B>(self.entity, bundle)
            };
        }
    }

    fn verify_insert_action(&self, meta: EntityMeta, bundle_metas: &[ComponentMeta]) {
        let arch = unsafe {
            self.world
                .archetypes
                .get_unchecked(meta.location.archetype_id)
        };

        if arch
            .component_metas
            .iter()
            .any(|m| bundle_metas.contains(m))
        {
            panic!(
                "Tried to insert component into entity already containing component:\
                            inserting bundle => {:#?}. Maybe this should be supported?",
                bundle_metas
            );
        }
    }

    pub fn remove<B: Bundle>(&mut self) {
        let Some(meta) = self.world.entities.meta(self.entity).cloned() else {
            panic!("cannot remove bundle from entity that does not exist");
        };
        trace!("meta found: {:?}", meta);

        assert!(!meta.free);

        let mut bundle_metas = Vec::new();
        B::component_meta(&mut self.world.components, &mut |meta| {
            bundle_metas.push(*meta);
        });
        self.verify_remove_action(meta, &bundle_metas);
        self.swap_remove_entity(meta);

        let mut new_metas = unsafe {
            self.world
                .archetypes
                .get_mut_unchecked(meta.location.archetype_id)
                .component_metas
                .clone()
                .to_vec()
        };
        new_metas.retain(|t| !bundle_metas.contains(t));
        let new_metas = new_metas.into_boxed_slice();

        let (arch_id, table_id) = unsafe {
            UnsafeWorldCell::find_or_create_storage::<B>(
                meta,
                new_metas,
                &mut self.world.archetypes,
                &mut self.world.tables,
                &mut self.world.components,
                &|id, _| !bundle_metas.iter().any(|m| m.id == *id),
                false,
            )
        };

        trace!("removing components: {:?}", bundle_metas);

        unsafe {
            UnsafeWorldCell::transfer_table_row_if(
                &mut self.world.tables,
                meta.location.table_row,
                meta.location.table_id,
                table_id,
                |id, _| !bundle_metas.iter().any(|m| m.id == *id),
            )
        };

        unsafe {
            UnsafeWorldCell::insert_entity_into_world(
                self.entity,
                self.world.archetypes.get_mut_unchecked(arch_id),
                &mut self.world.tables,
                &mut self.world.entities,
            );
        }
    }

    fn verify_remove_action(&self, meta: EntityMeta, bundle_metas: &[ComponentMeta]) {
        let arch = unsafe {
            self.world
                .archetypes
                .get_unchecked(meta.location.archetype_id)
        };

        if !bundle_metas
            .iter()
            .all(|m| arch.component_metas.contains(m))
        {
            panic!(
                "Tried to remove component from entity who does not contain component:\
                            remove bundle => {:#?}. Maybe this should be supported?",
                bundle_metas
            );
        }

        if arch
            .component_metas
            .len()
            .saturating_sub(bundle_metas.len())
            == 0
        {
            panic!(
                "Cannot remove last component(s) from entity: {:#?}. This should be implemented.",
                bundle_metas
            );
        }
    }

    // pub fn get_components(&self) ->

    pub fn despawn(mut self) {
        let Some(meta) = self.world.entities.meta(self.entity).cloned() else {
            trace!("entity uninitialized, noop");
            return;
        };

        let table = self
            .world
            .tables
            .get_mut(meta.location.table_id)
            .expect("valid entity");
        table.swap_remove_row(meta.location.table_row);
        self.swap_remove_entity(meta);
        self.world.entities.despawn(self.entity);
    }

    fn swap_remove_entity(&mut self, meta: EntityMeta) {
        let old_arch_row = meta.location.arch_row;
        let old_arch = unsafe {
            self.world
                .archetypes
                .get_mut_unchecked(meta.location.archetype_id)
        };

        match old_arch.swap_remove_entity(old_arch_row) {
            SwapEntity::Swap => unsafe {
                self.world
                    .entities
                    .set_location(old_arch.get_entity_unchecked(old_arch_row), meta.location);
                old_arch.entities[old_arch_row.0].row = meta.location.table_row;
            },
            SwapEntity::Pop => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use ecs_macro::InternalComponent;
    // use tracing_test::traced_test;
    // use util::tracing;

    #[derive(Debug, InternalComponent)]
    struct Health(u32);

    #[derive(Debug, InternalComponent)]
    struct Weight(u32);

    #[derive(Debug, InternalComponent, PartialEq, Eq, Hash)]
    struct Size(u32);

    macro_rules! impl_drop {
        ($s:ident) => {
            impl Drop for $s {
                fn drop(&mut self) {
                    println!("Dropping: {:?}", self);
                }
            }
        };
    }

    impl_drop!(Health);
    impl_drop!(Weight);
    impl_drop!(Size);

    // #[traced_test]
    #[test]
    fn all_entity_mutations() {
        let mut world = World::default();
        let e2 = world.spawn((Health(0), Weight(0)));
        let e = world.spawn((Health(1), Weight(1)));

        world.despawn(e2);

        // let mut e = world.entity_mut(e);
        // e.insert(Size(1));
        // // println!("{:#?}", world.entities);
        // e.despawn();
        //
        // let mut e2 = world.entity_mut(e2);
        // e2.insert(Size(0));
        // e2.remove::<(Health, Size)>();
        // e2.insert((Health(69), Size(69)));

        // println!("{:#?}", world.entities);

        // assert!(world.archetypes.len() == 3);
        // assert!(world.entities.len() == 2);

        // println!("{:#?}", world);

        // println!("exiting scope");
    }
}
