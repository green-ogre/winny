use std::{cell::UnsafeCell, hash::Hash};

use std::ptr::NonNull;

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
    pub(crate) fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn id(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TableRow(pub usize);

#[derive(Debug, Default)]
pub struct Tables {
    tables: Vec<Table>,
}

#[allow(clippy::missing_safety_doc)]
impl Tables {
    pub fn push(&mut self, table: Table) -> TableId {
        self.tables.push(table);
        TableId(self.tables.len() - 1)
    }

    pub unsafe fn get_unchecked(&self, id: TableId) -> &Table {
        // Safety:
        // Cannot obtain a ['TableId'] other than from Tables. Depends on the Immutability of ['Tables']
        unsafe { self.tables.get_unchecked(id.index()) }
    }

    pub unsafe fn get_mut_unchecked(&mut self, id: TableId) -> &mut Table {
        // Safety:
        // Cannot obtain a ['TableId'] other than from Tables. Depends on the Immutability of ['Tables']
        unsafe { self.tables.get_unchecked_mut(id.index()) }
    }

    pub fn get(&self, id: TableId) -> Option<&Table> {
        self.tables.get(id.index())
    }

    pub fn get_mut(&mut self, id: TableId) -> Option<&mut Table> {
        self.tables.get_mut(id.index())
    }
}

pub struct Table {
    storage: SparseSet<ComponentId, Column>,
}

impl Debug for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let storage = self.storage.iter().collect::<Vec<_>>();
        f.debug_struct("Table").field("storage", &storage).finish()
    }
}

impl Default for Table {
    fn default() -> Self {
        Self {
            storage: SparseSet::new(),
        }
    }
}

#[allow(clippy::missing_safety_doc)]
impl Table {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            storage: SparseSet::with_capacity(cap),
        }
    }

    pub fn clone_empty(&self) -> Self {
        let mut storage = SparseSet::new();
        for (component_id, column) in self.storage.iter() {
            storage.insert(*component_id, column.clone_empty());
        }

        Self { storage }
    }

    pub fn clone_empty_if<F>(&self, f: F) -> Self
    where
        F: Fn(&ComponentId, &Column) -> bool,
    {
        let mut storage = SparseSet::new();
        for (component_id, column) in self.storage.iter() {
            if f(component_id, column) {
                storage.insert(*component_id, column.clone_empty());
            }
        }

        Self { storage }
    }

    // This can only be used immediately following clone_empty, the world relies on immutable
    // tables!
    pub unsafe fn remove_column(&mut self, id: ComponentId) {
        self.storage.remove(&id);
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

    pub fn swap_remove_row(&mut self, row: TableRow) {
        for column in self.storage.iter_mut() {
            column.1.swap_remove_row_drop(row);
        }
    }

    pub fn clear(&mut self) {
        for column in self.storage.iter_mut() {
            column.1.clear();
        }
    }

    pub fn get_entity<T: Component>(&self, row: TableRow, component_id: ComponentId) -> &T {
        let Some(column) = self.storage.get(&component_id) else {
            error!("Could not get entity from table: {:?}", row);
            panic!();
        };

        assert!(row.0 < column.len());

        unsafe { column.get_row_unchecked::<T>(row) }
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

        assert!(row.0 < column.len());

        unsafe { column.get_row_mut_unchecked::<T>(row) }
    }

    // Safety:
    //     This should only ever be called in the construction of a new table. Archetype and Query
    //     Metadata relies on the immutability of a Table.
    pub unsafe fn insert_column(&mut self, column: Column, component_id: ComponentId) {
        self.storage.insert(component_id, column);
    }

    // Safety:
    //     This should only ever be called in the construction of a new table. Archetype and Query
    //     Metadata relies on the immutability of a Table.
    pub unsafe fn new_column_from_meta(&mut self, meta: &ComponentMeta) {
        let column = Column::new_from_meta(meta);
        self.storage.insert(meta.id, column);
    }

    pub fn push_column<T: Component>(&mut self, val: T, component_id: ComponentId) {
        let Some(column) = self.storage.get_mut(&component_id) else {
            error!("Could not push component to table");
            panic!();
        };

        // caller promises that component_id and component match
        unsafe { column.push::<T>(val) }
    }

    pub fn push_column_unchecked(&mut self, component_id: ComponentId, val: OwnedPtr) {
        let Some(column) = self.storage.get_mut(&component_id) else {
            error!("Could not append column to table");
            panic!();
        };

        // caller promises that component_id and component match
        unsafe { column.push_erased(val) }
    }

    pub fn column_mut(&mut self, component_id: &ComponentId) -> Option<&mut Column> {
        self.storage.get_mut(component_id)
    }

    pub unsafe fn column_mut_unchecked(&mut self, component_id: &ComponentId) -> &mut Column {
        self.storage.get_mut_unchecked(component_id)
    }

    pub unsafe fn column_slice<T: Component>(
        &self,
        component_id: &ComponentId,
    ) -> &[UnsafeCell<T>] {
        self.storage.get(component_id).unwrap().as_slice()
    }

    pub unsafe fn try_column_slice<T: Component>(
        &self,
        component_id: &ComponentId,
    ) -> Option<&[UnsafeCell<T>]> {
        self.storage.get(component_id).map(|c| c.as_slice())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ComponentId, &mut Column)> {
        self.storage.iter_mut()
    }
}

#[derive(Debug)]
pub struct Column {
    components: DumbVec,
}

#[allow(clippy::missing_safety_doc)]
impl Column {
    pub fn new<T>() -> Self {
        Self {
            components: DumbVec::new::<T>(),
        }
    }

    pub fn new_from_meta(meta: &ComponentMeta) -> Self {
        Self {
            components: DumbVec::new_from(meta.layout(), 0, meta.drop),
        }
    }

    pub fn clone_empty(&self) -> Self {
        Self {
            components: self.components.clone_empty(),
        }
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn swap_remove_row_drop(&mut self, row: TableRow) {
        unsafe { self.components.swap_remove_drop(row.0) };
    }

    pub fn swap_remove_row_no_drop(&mut self, row: TableRow) {
        unsafe { self.components.swap_remove_no_drop(row.0) };
    }

    pub fn clear(&mut self) {
        self.components.clear();
    }

    pub unsafe fn push<T>(&mut self, val: T) {
        self.components.push(val)
    }

    pub unsafe fn push_erased(&mut self, val: OwnedPtr) {
        self.components.push_erased(val)
    }

    pub unsafe fn get_row_unchecked<T>(&self, row: TableRow) -> &T {
        self.components.get_unchecked(row.0).cast::<T>().as_ref()
    }

    pub unsafe fn get_row_mut_unchecked<T>(&mut self, row: TableRow) -> &mut T {
        self.components.get_unchecked(row.0).cast::<T>().as_mut()
    }

    pub unsafe fn get_row_ptr_unchecked(&self, row: TableRow) -> NonNull<u8> {
        self.components.get_unchecked(row.0)
    }

    pub unsafe fn as_slice<T: Component>(&self) -> &[UnsafeCell<T>] {
        unsafe { self.components.as_slice::<T>() }
    }
}
