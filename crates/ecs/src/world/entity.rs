use crate::{ArchId, TableId, TableRow};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entity(u64);

impl Entity {
    pub fn new(generation: u32, storage_index: u32) -> Self {
        Self(((generation as u64) << 32) | storage_index as u64)
    }

    pub fn generation(&self) -> u32 {
        (self.0 >> 32) as u32
    }

    pub fn index(&self) -> u32 {
        self.0 as u32
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
    pub table_row: TableRow,
    pub archetype_id: ArchId,
    pub archetype_index: usize,
}

impl MetaLocation {
    pub fn new(
        table_id: TableId,
        table_row: TableRow,
        archetype_id: ArchId,
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
