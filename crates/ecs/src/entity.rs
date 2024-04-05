use crate::{any::*, Component, Storage, TableRow};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entity(usize);

impl Entity {
    pub fn new(generation: u32, storage_index: u32) -> Self {
        Self(((generation as usize) << 32) | storage_index as usize)
    }

    pub fn generation(&self) -> u32 {
        (self.0 >> 32) as u32
    }

    pub fn index(&self) -> u32 {
        (self.0 & 0x00000000ffffffff) as u32
    }
}

impl TypeGetter for Entity {
    fn type_id() -> TypeId {
        ENTITY
    }

    fn type_name() -> TypeName {
        TypeName::new("Entity")
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
    pub table_id: usize,
    pub table_row: TableRow,
    pub archetype_id: usize,
    pub archetype_index: usize,
}

impl MetaLocation {
    pub fn new(
        table_id: usize,
        table_row: TableRow,
        archetype_id: usize,
        archetype_index: usize,
    ) -> Self {
        Self {
            table_id,
            table_row,
            archetype_id,
            archetype_index,
        }
    }
}
