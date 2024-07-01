use std::any::TypeId;

use logger::error;

use crate::EntityMeta;

use super::*;

#[derive(Debug)]
pub struct Archetypes {
    archetypes: SparseSet<ArchId, Archetype>,
}

impl Archetypes {
    pub fn new() -> Self {
        Self {
            archetypes: SparseSet::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.archetypes.len()
    }

    pub fn get_from_type_ids(&mut self, ids: &mut [TypeId]) -> Option<&mut Archetype> {
        // type_ids sorted on creation
        ids.sort();
        self.archetypes
            .values_mut()
            .iter_mut()
            .find(|arch| arch.type_ids.as_slice() == ids)
    }

    pub fn get(&self, id: ArchId) -> Option<&Archetype> {
        self.archetypes.get(&id)
    }

    pub fn get_mut(&mut self, id: ArchId) -> &mut Archetype {
        self.archetypes.get_mut(&id).unwrap_or_else(|| {
            error!("Could not index Archetypes at {:?}", id);
            panic!()
        })
    }

    pub fn new_archetype(&mut self, id: ArchId, arch: Archetype) {
        self.archetypes.insert(id, arch);
    }

    pub fn new_id(&self) -> ArchId {
        ArchId::new(self.archetypes.len())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Archetype> {
        self.archetypes.values().iter()
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
    fn to_index(&self) -> usize {
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
    fn to_index(&self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub struct Archetype {
    pub id: ArchId,
    pub table_id: TableId,

    pub type_ids: Vec<TypeId>,
    pub component_desc: SparseSet<ComponentId, StorageType>,

    pub entities: SparseSet<ArchIndex, ArchEntity>,
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

impl Archetype {
    pub fn new(
        id: ArchId,
        table_id: TableId,
        mut type_ids: Vec<TypeId>,
        component_desc: SparseSet<ComponentId, StorageType>,
    ) -> Self {
        type_ids.sort();
        Self {
            id,
            table_id,
            type_ids,
            component_desc,
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
