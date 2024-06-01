use std::any::TypeId;

use logger::error;

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

    pub fn get(&self, id: ArchId) -> Option<&Archetype> {
        self.archetypes.get(&id)
    }

    pub fn get_from_type_ids(&self, ids: &mut [TypeId]) -> Option<&Archetype> {
        self.archetypes
            .iter()
            .find(|(_, arch)| arch.component_ids.clone().sort() == ids.sort())
            .map(|(_, arch)| arch)
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

#[derive(Debug)]
pub struct Archetype {
    pub id: ArchId,
    pub table_id: TableId,

    pub component_ids: Vec<TypeId>,
    pub component_desc: SparseSet<ComponentId, StorageType>,

    pub entities: Vec<ArchEntity>,
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
        component_ids: Vec<TypeId>,
        component_desc: SparseSet<ComponentId, StorageType>,
        entities: Vec<ArchEntity>,
    ) -> Self {
        Self {
            id,
            table_id,
            component_ids,
            component_desc,
            entities,
        }
    }

    pub fn contains_type_id<T: Component>(&self) -> bool {
        self.component_ids.contains(&TypeId::of::<T>())
    }

    pub fn contains_query<T: QueryData>(&self) -> bool {
        self.contains_id_set(&T::set_ids())
    }

    pub fn contains_id(&self, id: &TypeId) -> bool {
        self.component_ids.contains(id)
    }

    pub fn contains_id_set(&self, components: &[TypeId]) -> bool {
        components.iter().all(|c| self.component_ids.contains(c))
    }

    pub fn comp_set_eq(&self, components: &[TypeId]) -> bool {
        components.iter().all(|c| self.component_ids.contains(c))
            && components.len() == self.component_ids.len()
    }
}
