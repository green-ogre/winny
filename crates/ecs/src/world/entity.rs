use std::sync::atomic::AtomicU32;

use crate::{ArchId, ArchIndex, Bundle, SparseArray, SparseArrayIndex, TableId, World};

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

#[derive(Debug, Clone, Copy)]
pub struct MetaLocation {
    pub table_id: TableId,
    pub archetype_id: ArchId,
    pub archetype_index: ArchIndex,
}

impl MetaLocation {
    pub fn new(table_id: TableId, archetype_id: ArchId, archetype_index: ArchIndex) -> Self {
        Self {
            table_id,
            archetype_id,
            archetype_index,
        }
    }
}

pub struct Entities {
    entities: SparseArray<Entity, EntityMeta>,
    reserve_index: AtomicU32,
    free_entities: Vec<u32>,
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

    pub fn meta(&self, entity: Entity) -> Option<&EntityMeta> {
        self.generation_is_valid(entity)
            .then(|| self.entities.get(&entity))?
    }

    pub fn meta_mut(&mut self, entity: Entity) -> Option<&mut EntityMeta> {
        self.generation_is_valid(entity)
            .then(|| self.entities.get_mut(&entity))?
    }

    pub fn spawn(&mut self, table_id: TableId, arch_id: ArchId, arch_index: ArchIndex) -> Entity {
        let location = MetaLocation::new(table_id, arch_id, arch_index);
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
        arch_index: ArchIndex,
    ) {
        let location = MetaLocation::new(table_id, arch_id, arch_index);
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

    fn generation_is_valid(&self, entity: Entity) -> bool {
        self.entities
            .get(&entity)
            .map(|e| e.generation)
            .unwrap_or_else(|| u32::MAX)
            == entity.generation()
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
        if let Some(meta) = self.world.entities.meta(self.entity) {
            todo!();
        } else {
            self.world.spawn_with_entity::<B>(self.entity, bundle);
        }
    }

    pub fn remove<B: Bundle>(&self) {
        todo!();
    }

    pub fn despawn(self) {
        todo!();
    }
}
