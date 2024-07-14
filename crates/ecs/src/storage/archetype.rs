use std::{any::TypeId, marker::PhantomData};

use util::tracing::error;

use crate::EntityMeta;

use super::*;

#[derive(Debug)]
pub struct Archetypes {
    archetypes: SparseSet<ArchId, Archetype>,
}

impl Default for Archetypes {
    fn default() -> Self {
        Self {
            archetypes: SparseSet::new(),
        }
    }
}

impl Archetypes {
    pub fn push(&mut self, archetype: Archetype) -> ArchId {
        ArchId(self.archetypes.insert_in_first_empty(archetype))
    }

    pub fn get(&self, id: &ArchId) -> Option<&Archetype> {
        self.archetypes.get(&id)
    }

    pub fn get_mut(&mut self, id: &ArchId) -> &mut Archetype {
        self.archetypes.get_mut(&id).unwrap_or_else(|| {
            error!("Could not index Archetypes at {:?}", id);
            panic!()
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ArchId, &Archetype)> {
        self.archetypes.iter()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ArchId(usize);

impl ArchId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn id(&self) -> usize {
        self.0
    }
}

impl SparseArrayIndex for ArchId {
    fn index(&self) -> usize {
        self.id()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ArchIndex(usize);

impl ArchIndex {
    pub fn new(index: usize) -> Self {
        Self(index)
    }
}

impl SparseArrayIndex for ArchIndex {
    fn index(&self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub struct ArchEntity {
    pub entity: Entity,
    pub row: TableRow,
}

impl ArchEntity {
    pub fn new(entity: Entity, row: TableRow) -> Self {
        Self { entity, row }
    }
}

#[derive(Debug)]
pub struct Archetype {
    pub table_id: TableId,
    pub type_ids: Vec<TypeId>,
    pub entities: SparseSet<ArchIndex, ArchEntity>,
}

#[derive(Clone, Copy)]
pub struct ReservedArchIndex<'r>(ArchIndex, PhantomData<&'r ArchIndex>);

impl<'r> Into<ArchIndex> for ReservedArchIndex<'r> {
    fn into(self) -> ArchIndex {
        self.0
    }
}

impl Archetype {
    pub fn new(table_id: TableId, mut type_ids: Vec<TypeId>) -> Self {
        type_ids.sort();
        Self {
            table_id,
            type_ids,
            entities: SparseSet::new(),
        }
    }

    pub fn new_entity(&mut self, arch_entity: ArchEntity) -> ArchIndex {
        let index = self.entities.insert_in_first_empty(arch_entity);
        ArchIndex::new(index)
    }

    pub fn remove_entity(&mut self, index: ArchIndex) {
        self.entities.remove(&index);
    }

    pub fn new_entity_with<F>(&mut self, table_row: TableRow, f: F) -> Entity
    where
        F: FnOnce(ArchIndex) -> Entity,
    {
        let index = self.reserve_entity();
        let entity = f(index.into());
        self.new_entity_reserved(ArchEntity::new(entity, table_row), index.into());
        entity
    }

    pub fn new_entity_from<F>(&mut self, entity: Entity, table_row: TableRow, f: F)
    where
        F: FnOnce(ArchIndex),
    {
        let index = self.reserve_entity();
        f(index.into());
        self.new_entity_reserved(ArchEntity::new(entity, table_row), index.into());
    }

    // prevents having to call new_entity without first inserting entity into reserved index
    pub fn reserve_entity(&self) -> ReservedArchIndex<'_> {
        ReservedArchIndex(ArchIndex::new(self.entities.sparse_len()), PhantomData)
    }

    pub fn new_entity_reserved(&mut self, arch_entity: ArchEntity, index: ArchIndex) -> ArchIndex {
        let index = index.into();
        self.entities.insert(index, arch_entity);

        index
    }

    pub fn get_entity_table_row(&self, entity: EntityMeta) -> TableRow {
        self.entities
            .get(&entity.location.archetype_index)
            .unwrap()
            .row
    }

    pub fn contains_type_id<T: Component>(&self) -> bool {
        self.type_ids.contains(&TypeId::of::<T>())
    }

    pub fn contains_query<T: QueryData>(&self) -> bool {
        self.contains_id_set(&T::set_ids())
    }

    pub fn contains_id(&self, id: &TypeId) -> bool {
        self.type_ids.contains(id)
    }

    pub fn contains_id_set(&self, components: &[TypeId]) -> bool {
        components.iter().all(|c| {
            if *c == std::any::TypeId::of::<Entity>() {
                true
            } else {
                self.type_ids.contains(c)
            }
        })
    }

    pub fn comp_set_eq(&self, components: &[TypeId]) -> bool {
        components.iter().all(|c| self.type_ids.contains(c))
            && components.len() == self.type_ids.len()
    }
}
