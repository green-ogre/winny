use std::{hash::Hash, ptr::NonNull};

use logger::{error, info};

use crate::unsafe_world::UnsafeWorldCell;

use self::dumb_vec::DumbVec;

use super::*;

#[derive(Debug)]
pub struct Column {
    storage: DumbVec,
}

impl Column {
    pub fn new(storage: DumbVec) -> Self {
        Self { storage }
    }

    pub unsafe fn storage_mut(&mut self) -> &mut DumbVec {
        &mut self.storage
    }

    pub unsafe fn storage(&self) -> &DumbVec {
        &self.storage
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }

    // TODO: error
    pub fn swap_remove(&mut self, row: TableRow) -> Result<(), ()> {
        if self.len() <= row.0 {
            return Err(());
        }

        self.storage.swap_remove_drop_unchecked(row.0);

        Ok(())
    }

    pub unsafe fn get_row(&self, row: TableRow) -> NonNull<u8> {
        debug_assert!(row.0 < self.storage.len());

        self.storage.get_unchecked(row.0)
    }

    pub fn push<T>(&mut self, val: T) -> Result<(), IntoStorageError> {
        self.storage.push(val)
    }
}

pub trait SparseArrayIndex: Copy {
    fn to_index(&self) -> usize;
}

impl SparseArrayIndex for usize {
    fn to_index(&self) -> usize {
        *self
    }
}

#[derive(Debug)]
pub struct Tables {
    tables: SparseSet<TableId, Table>,
}

unsafe impl Sync for Tables {}
unsafe impl Send for Tables {}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TableId(usize);

impl SparseArrayIndex for TableId {
    fn to_index(&self) -> usize {
        self.id()
    }
}

impl TableId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn id(&self) -> usize {
        self.0
    }
}

impl Tables {
    pub fn new() -> Self {
        Self {
            tables: SparseSet::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.tables.len()
    }

    pub fn new_table(&mut self, id: TableId, table: Table) {
        self.tables.insert(id, table);
    }

    pub fn get(&self, id: TableId) -> Option<&Table> {
        self.tables.get(&id)
    }

    pub fn get_mut(&mut self, id: TableId) -> Option<&mut Table> {
        self.tables.get_mut(&id)
    }

    pub fn new_id(&self) -> TableId {
        TableId::new(self.tables.len())
    }
}

#[derive(Debug)]
pub struct Table {
    pub storage: SparseSet<ComponentId, Column>,
}

impl Table {
    pub fn new(storages: Vec<(ComponentId, DumbVec)>) -> Self {
        let mut storage = SparseSet::new();

        for (component_id, dumb_vec) in storages.into_iter() {
            storage.insert(component_id, Column::new(dumb_vec));
        }

        Self { storage }
    }

    pub fn from_bundle<'w, T: Bundle>(bundle: T, world: UnsafeWorldCell<'w>) -> Self {
        let mut table = Self::new(bundle.new_storages(world));
        table.new_entity(bundle, world);

        table
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }

    pub fn depth(&self) -> usize {
        if self.storage.len() == 0 {
            0
        } else {
            let index = self.storage.indexes().first().expect("cannot be empty");
            self.storage.get(index).expect("must be occupied").len()
        }
    }

    pub fn new_entity<'w, T: Bundle>(&mut self, bundle: T, world: UnsafeWorldCell<'w>) {
        let _ = bundle.push_storage(world, self).map_err(|err| {
            error!("Could not push bundle into table storage: {:?}", err);
            // TODO: REMOVE
            info!("{:#?}", unsafe { world.read_only() });
            panic!();
        });
    }

    pub fn remove_entity(&mut self, row: TableRow) -> Result<(), ()> {
        for column in self.storage.values_mut() {
            column.swap_remove(row)?;
        }

        Ok(())
    }

    pub unsafe fn get_entity<T: Component>(&self, row: TableRow, component_id: ComponentId) -> &T {
        if let Some(column) = self.storage.get(&component_id) {
            column.get_row(row).cast::<T>().as_ref()
        } else {
            error!("Could not get entity from table: {:?}", row);
            panic!();
        }
    }

    pub unsafe fn get_entity_mut<T: Component>(
        &self,
        row: TableRow,
        component_id: ComponentId,
    ) -> &mut T {
        if let Some(column) = self.storage.get(&component_id) {
            column.get_row(row).cast::<T>().as_mut()
        } else {
            error!("Could not get entity from table: {:?}", row);
            panic!();
        }
    }

    // Caller needs to ensure that all elements of entity are present and successfully pushed
    // so that there are not miss-shapen columns
    pub fn push_column<T: Component>(
        &mut self,
        val: T,
        component_id: ComponentId,
    ) -> Result<(), IntoStorageError> {
        Ok(self
            .storage
            .get_mut(&component_id)
            .ok_or_else(|| IntoStorageError::IncorrectSparseIndex)?
            .push(val)?)
    }

    pub fn column_mut(&mut self, component_id: ComponentId) -> Option<&mut Column> {
        self.storage.get_mut(&component_id)
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum StorageType {
    Table,
    SparseSet,
}

impl StorageType {
    pub fn of<T: Storage>() -> StorageType {
        T::storage_type()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TableRow(pub usize);
