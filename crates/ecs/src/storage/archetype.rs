use super::*;

#[derive(Debug)]
pub struct Archetypes {
    archetypes: MutableSparseSet<ArchId, Archetype>,
    comp_set_index: MutableSparseSet<Box<[TypeId]>, ArchId>,
}

impl Archetypes {
    pub fn new() -> Self {
        Self {
            archetypes: MutableSparseSet::new(),
            comp_set_index: MutableSparseSet::new(),
        }
    }

    pub fn get(&self, id: ArchId) -> &Archetype {
        self.archetypes.get_value(&id).expect("valid arch id")
    }

    pub fn get_from_comps(&self, comp_set: &Box<[TypeId]>) -> Option<&Archetype> {
        let index = self.comp_set_index.get_value(comp_set)?;

        Some(self.archetypes.get_value(index)?)
    }

    pub fn get_mut(&mut self, id: ArchId) -> &mut Archetype {
        self.archetypes.get_value_mut(&id).expect("valid arch id")
    }

    pub fn new_archetype(&mut self, id: ArchId, arch: Archetype) {
        self.comp_set_index
            .insert(arch.component_ids.clone().into_boxed_slice(), id);
        self.archetypes.insert(id, arch);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Archetype> {
        self.archetypes.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Archetype> {
        self.archetypes.iter_mut()
    }

    pub fn new_id(&self) -> ArchId {
        ArchId(self.archetypes.len())
    }
}

impl SparseHash for Box<[TypeId]> {}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct ArchId(pub usize);
impl SparseHash for ArchId {}

#[derive(Debug)]
pub struct Archetype {
    pub id: ArchId,
    pub table_id: TableId,

    pub component_ids: Vec<TypeId>,
    pub component_desc: FxHashMap<TypeId, StorageType>,

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
        component_desc: FxHashMap<TypeId, StorageType>,
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

    pub fn contains<T: TypeGetter>(&self) -> bool {
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
