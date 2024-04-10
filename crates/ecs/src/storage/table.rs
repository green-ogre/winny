use std::{alloc::Layout, hash::Hash, ptr::NonNull};

use self::dumb_vec::DumbVec;

use super::*;

#[derive(Debug)]
pub struct Column {
    storage: DumbVec,
}

impl Column {
    pub fn new(desc: ComponentDescription) -> Self {
        Self {
            storage: DumbVec::from_description(desc),
        }
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }

    pub fn swap_remove(&mut self, row: TableRow) {
        self.storage.swap_remove(row.0);
    }

    pub unsafe fn get_row(&self, row: TableRow) -> NonNull<u8> {
        debug_assert!(row.0 < self.storage.len());

        self.storage.get_unchecked(row.0)
    }

    pub fn push<T>(&mut self, val: T) -> Result<(), ()> {
        self.storage.push(val)
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct MutableSparseSet<I: SparseHash, V> {
    set: FxHashMap<I, V>,
}

impl<I: SparseHash, V> MutableSparseSet<I, V> {
    pub fn new() -> Self {
        Self {
            set: FxHashMap::default(),
        }
    }

    pub fn get_value(&self, index: &I) -> Option<&V> {
        self.set.get(index)
    }

    pub fn get_value_mut(&mut self, index: &I) -> Option<&mut V> {
        self.set.get_mut(index)
    }

    pub fn insert(&mut self, index: I, val: V) {
        self.set.insert(index, val);
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.set.values().into_iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.set.values_mut().into_iter()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ImmutableSparseSet<I: SparseHash, V> {
    set: FxHashMap<I, V>,
}

impl<I: SparseHash, V> ImmutableSparseSet<I, V> {
    pub unsafe fn get_value(&self, index: &I) -> &V {
        self.set
            .get(index)
            .expect("Invalid index into ImmutableSparseSet")
    }

    pub fn get_value_mut(&mut self, index: &I) -> &mut V {
        self.set
            .get_mut(index)
            .expect("Invalid index into ImmutableSparseSet")
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.set.values().into_iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.set.values_mut().into_iter()
    }
}

#[derive(Debug)]
pub struct SparseSetBuilder<I, V> {
    set: FxHashMap<I, V>,
}

impl<I: SparseHash, V> From<SparseSetBuilder<I, V>> for ImmutableSparseSet<I, V> {
    fn from(value: SparseSetBuilder<I, V>) -> Self {
        Self { set: value.set }
    }
}

impl<I: SparseHash, V> SparseSetBuilder<I, V> {
    pub fn new() -> Self {
        Self {
            set: FxHashMap::default(),
        }
    }

    pub fn insert(&mut self, index: I, value: V) {
        self.set.insert(index, value);
    }

    pub fn build(self) -> ImmutableSparseSet<I, V> {
        ImmutableSparseSet::from(self)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ComponentDescription {
    pub layout: Layout,
    pub type_id: TypeId,
    pub drop: Option<DumbDrop>,
}

pub trait SparseHash: Hash + PartialEq + Eq {}

impl SparseHash for Box<[ComponentDescription]> {}

#[derive(Debug)]
pub struct Tables {
    tables: MutableSparseSet<TableId, Table>,
    descriptor_index: MutableSparseSet<Box<[ComponentDescription]>, TableId>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TableId(pub usize);
impl SparseHash for TableId {}

impl Tables {
    pub fn new() -> Self {
        Self {
            tables: MutableSparseSet::new(),
            descriptor_index: MutableSparseSet::new(),
        }
    }

    pub fn new_table(
        &mut self,
        id: TableId,
        table: Table,
        descriptions: Vec<ComponentDescription>,
    ) {
        self.descriptor_index
            .insert(descriptions.into_boxed_slice(), id);
        self.tables.insert(id, table);
    }

    pub fn get(&self, id: TableId) -> &Table {
        self.tables.get_value(&id).expect("valid table id")
    }

    pub fn get_from_descriptions(descriptions: Box<[ComponentDescription]>) -> Option<Table> {
        None
    }

    pub fn new_id(&self) -> TableId {
        TableId(self.tables.len())
    }
}

#[derive(Debug)]
pub struct Table {
    pub storage: ImmutableSparseSet<TypeId, Column>,
}

impl Table {
    pub fn from_bundle<T: Bundle>(bundle: T) -> Self {
        let mut table = Self::new(&bundle.descriptions());
        table.new_entity(bundle);

        table
    }

    fn new(descriptions: &[ComponentDescription]) -> Self {
        let mut storage = SparseSetBuilder::new();

        for desc in descriptions.iter() {
            storage.insert(desc.type_id, Column::new(desc.clone()));
        }

        Self {
            storage: storage.into(),
        }
    }

    pub fn len(&self) -> usize {
        if self.storage.is_empty() {
            return 0;
        }

        self.storage.iter().nth(0).expect("cannot be empty").len()
    }

    pub fn new_entity<T: Bundle>(&mut self, bundle: T) {
        assert!(bundle.push_storage(self).is_ok());
    }

    pub fn remove_entity(&mut self, row: TableRow) -> Result<(), ()> {
        for column in self.storage.iter_mut() {
            column.swap_remove(row);
        }

        Ok(())
    }

    pub unsafe fn get_entity<T: TypeGetter>(&self, row: TableRow) -> &T {
        self.storage
            .get_value(&T::type_id())
            .get_row(row)
            .cast::<T>()
            .as_ref()
    }

    pub unsafe fn get_mut<T: TypeGetter>(&self, row: TableRow) -> &mut T {
        self.storage
            .get_value(&T::type_id())
            .get_row(row)
            .cast::<T>()
            .as_mut()
    }

    // Caller needs to ensure that all elements of entity are present and successfully pushed
    // so that there are not miss-shapen columns
    pub fn push_column<T: TypeGetter>(&mut self, val: T) -> Result<(), ()> {
        self.storage.get_value_mut(&T::type_id()).push(val)
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
