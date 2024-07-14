use std::{cell::UnsafeCell, hash::Hash};

use any_vec::AnyVec;
use util::tracing::error;

use super::*;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TableId(usize);

impl SparseArrayIndex for TableId {
    fn index(&self) -> usize {
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

#[derive(Debug)]
pub struct Tables {
    tables: SparseArray<TableId, Table>,
}

impl Default for Tables {
    fn default() -> Self {
        Self {
            tables: SparseArray::new(),
        }
    }
}

impl Tables {
    pub fn push(&mut self, table: Table) -> TableId {
        TableId(self.tables.insert_in_first_empty(table))
    }

    pub fn get(&self, id: TableId) -> &Table {
        // Safety:
        // Cannot obtain a ['TableId'] other than from Tables. Depends on the Immutability of ['Tables']
        unsafe { self.tables.get_unchecked(&id) }
    }

    pub fn get_mut(&mut self, id: TableId) -> &mut Table {
        // Safety:
        // Cannot obtain a ['TableId'] other than from Tables. Depends on the Immutability of ['Tables']
        unsafe { self.tables.get_mut_unchecked(&id) }
    }
}

#[derive(Debug)]
pub struct Table {
    storage: SparseSet<ComponentId, AnyVec<dyn Sync + Send>>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            storage: SparseSet::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            storage: SparseSet::with_capacity(cap),
        }
    }

    pub fn len(&self) -> usize {
        self.storage.sparse_len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn depth(&self) -> usize {
        if let Some(column) = self.storage.values().first() {
            column.len()
        } else {
            0
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ComponentId, &AnyVec<dyn Sync + Send>)> {
        self.storage.iter()
    }

    pub fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (&ComponentId, &mut AnyVec<dyn Sync + Send>)> {
        self.storage.iter_mut()
    }

    pub fn remove_entity(&mut self, row: TableRow) {
        for column in self.storage.iter_mut() {
            column.1.swap_remove(row.0);
        }
    }

    pub fn get_entity<T: Component>(&self, row: TableRow, component_id: ComponentId) -> &T {
        unsafe {
            let Some(column) = self.storage.get(&component_id) else {
                error!("Could not get entity from table: {:?}", row);
                panic!();
            };

            column.get_unchecked(row.0).downcast_ref_unchecked::<T>()
        }
    }

    pub fn get_entity_mut<T: Component>(
        &mut self,
        row: TableRow,
        component_id: ComponentId,
    ) -> &mut T {
        let Some(column) = self.storage.get_mut(&component_id) else {
            error!("Could not get entity from table: {:?}", row);
            panic!();
        };

        unsafe {
            column
                .get_unchecked_mut(row.0)
                .downcast_mut_unchecked::<T>()
        }
    }

    // Safety:
    //     This should only ever be called in the construction of a new table. Archetype and Query
    //     Metadata relies on the immutability of a Table.
    pub unsafe fn insert_column(
        &mut self,
        column: AnyVec<dyn Sync + Send>,
        component_id: ComponentId,
    ) {
        self.storage.insert(component_id, column);
    }

    pub fn push_column<T: Component>(&mut self, val: T, component_id: ComponentId) {
        let Some(column) = self.storage.get_mut(&component_id) else {
            error!("Could not push component to table");
            panic!();
        };

        unsafe {
            let mut vec = column.downcast_mut_unchecked::<T>();
            vec.push(val);
        }
    }

    pub fn column_mut(
        &mut self,
        component_id: &ComponentId,
    ) -> Option<&mut AnyVec<dyn Sync + Send>> {
        self.storage.get_mut(component_id)
    }

    pub unsafe fn column_slice<T: Component>(
        &self,
        component_id: &ComponentId,
    ) -> &mut [UnsafeCell<T>] {
        unsafe {
            let column = self
                .storage
                .get(component_id)
                .unwrap()
                .downcast_ref_unchecked::<T>();
            std::slice::from_raw_parts_mut(column.as_ptr() as *mut UnsafeCell<T>, column.len())
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TableRow(pub usize);
