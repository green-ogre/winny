use std::{
    borrow::{Borrow, BorrowMut},
    cell::{Ref, RefCell, RefMut},
    collections::{btree_map::Range, VecDeque},
    marker::PhantomData,
    ops::Deref,
};

use dyn_clone::DynClone;
use fxhash::FxHashMap;

use crate::{
    any::{self, *},
    ecs_derive::*,
    entity::Entity,
    Res, ResourceStorage,
};

#[derive(Debug)]
pub struct Table {
    pub storage: Vec<Box<dyn ComponentVec>>,
    pub len: usize,
}

impl Table {
    pub fn new<T: Bundle>(bundle: T) -> Self {
        Self {
            storage: bundle.into_storage(),
            len: 1,
        }
    }

    pub fn new_dyn(bundle: Box<dyn Bundle>) -> Self {
        Self {
            storage: bundle.into_storage(),
            len: 1,
        }
    }

    pub fn new_entity<T: Bundle>(&mut self, bundle: T) {
        bundle.push_storage(self);
        self.len += 1;
    }

    pub fn remove_entity(&mut self, row: TableRow) -> Result<(), ()> {
        for component_vec in self.storage.iter_mut() {
            component_vec.swap_remove(row.0)?;
        }
        self.len -= 1;

        Ok(())
    }

    pub fn borrow_component_vec<T: Component + TypeGetter>(&self) -> Option<Ref<Vec<T>>> {
        let vec = self
            .storage
            .iter()
            .find(|vec| vec.stored_type_id() == T::type_id())?;

        Some(
            vec.as_any()
                .downcast_ref::<RefCell<Vec<T>>>()
                .unwrap()
                .borrow(),
        )
    }

    pub fn borrow_mut_component_vec<T: Component + TypeGetter>(&self) -> Option<RefMut<Vec<T>>> {
        let vec = self
            .storage
            .iter()
            .find(|vec| vec.stored_type_id() == T::type_id())?;

        Some(
            vec.as_any()
                .downcast_ref::<RefCell<Vec<T>>>()
                .unwrap()
                .borrow_mut(),
        )
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

#[derive(Debug)]
pub struct Archetype {
    pub id: usize,
    pub table_id: usize,

    pub component_ids: Vec<TypeId>,
    pub component_desc: FxHashMap<TypeId, StorageType>,

    pub entities: Vec<(Entity, TableRow)>,
}

impl Archetype {
    pub fn new(
        id: usize,
        table_id: usize,
        component_ids: Vec<TypeId>,
        component_desc: FxHashMap<TypeId, StorageType>,
        entities: Vec<(Entity, TableRow)>,
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

    pub fn contains_id(&self, id: &TypeId) -> bool {
        self.component_ids.contains(id)
    }

    pub fn contains_id_set(&self, components: &[TypeId]) -> bool {
        self.component_ids.eq(components)
    }
}

pub trait Component: 'static + DynClone {}
dyn_clone::clone_trait_object!(Component);

pub trait ComponentStorageType {
    fn storage_type(&self) -> StorageType;
}

pub trait Storage {
    fn storage_type() -> StorageType;
}

impl<T: Storage + 'static> ComponentStorageType for T {
    fn storage_type(&self) -> StorageType {
        T::storage_type()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComponentSet {
    pub ids: Vec<TypeId>,
}

impl ComponentSet {
    pub fn new(mut ids: Vec<TypeId>) -> Self {
        ids.insert(0, any::ENTITY);
        Self { ids }
    }

    pub fn contains<T: TypeGetter>(&self) -> bool {
        self.ids.contains(&TypeId::of::<T>())
    }

    pub fn contains_id(&self, id: &TypeId) -> bool {
        self.ids.contains(id)
    }

    pub fn equivalent(&self, components: &[TypeId]) -> bool {
        self.ids.eq(components)
    }
}

pub trait ComponentVec: std::fmt::Debug + DynClone {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn stored_type_id(&self) -> TypeId;
    fn storage_type(&self) -> StorageType;
    fn swap_remove(&mut self, index: usize) -> Result<(), ()>;
    fn try_push(&mut self, value: &dyn Any) -> Result<(), ()>;
    fn try_append(&mut self, value: &dyn Any) -> Result<(), ()>;
    fn try_remove(&mut self, value: &dyn Any) -> Result<(), ()>;
    fn len(&self) -> usize;
    fn duplicate(&self, index: usize) -> Option<Box<dyn ComponentVec>>;
}

dyn_clone::clone_trait_object!(ComponentVec);

impl<T: TypeGetter> TypeGetter for RefCell<T> {
    fn type_id() -> TypeId {
        TypeId::new(T::type_id().consume() + 1)
    }

    fn type_name() -> TypeName {
        T::type_name()
    }
}

impl<T: TypeGetter> TypeGetter for Vec<T> {
    fn type_id() -> TypeId {
        TypeId::new(T::type_id().consume() + 11)
    }

    fn type_name() -> TypeName {
        T::type_name()
    }
}

impl<T: TypeGetter> TypeGetter for VecDeque<T> {
    fn type_id() -> TypeId {
        TypeId::new(T::type_id().consume() + 7)
    }

    fn type_name() -> TypeName {
        T::type_name()
    }
}

impl<T: Storage + TypeGetter + std::fmt::Debug + Clone> ComponentVec for RefCell<Vec<T>> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn stored_type_id(&self) -> TypeId {
        T::type_id()
    }

    fn storage_type(&self) -> StorageType {
        T::storage_type()
    }

    fn swap_remove(&mut self, index: usize) -> Result<(), ()> {
        if index >= self.len() {
            return Err(());
        }
        let _ = self.get_mut().swap_remove(index);

        Ok(())
    }

    fn try_push(&mut self, value: &dyn Any) -> Result<(), ()> {
        let value = value.downcast_ref::<T>().ok_or(())?.clone();
        self.get_mut().push(value);

        Ok(())
    }

    fn try_append(&mut self, value: &dyn Any) -> Result<(), ()> {
        let value = value.downcast_ref::<RefCell<Vec<T>>>().ok_or(())?.clone();
        self.get_mut().append(&mut value.borrow_mut());

        Ok(())
    }

    fn try_remove(&mut self, value: &dyn Any) -> Result<(), ()> {
        let value = value.downcast_ref::<RefCell<Vec<T>>>().ok_or(())?.clone();
        self.get_mut().append(&mut value.borrow_mut());

        Ok(())
    }

    fn len(&self) -> usize {
        self.borrow().len()
    }

    fn duplicate(&self, index: usize) -> Option<Box<dyn ComponentVec>> {
        println!("{:?}, {:?}", index, self);

        Some(Box::new(RefCell::new(vec![self
            .borrow()
            .get(index)?
            .clone()])))
    }
}

#[derive(Debug)]
pub enum IntoStorageError {
    MismatchedShape,
}

pub trait Bundle: BundleBoxed {
    fn into_storage(self) -> Vec<Box<dyn ComponentVec>>;
    fn push_storage(self, table: &mut Table) -> Result<(), IntoStorageError>;
    fn ids(&self) -> Vec<TypeId>;
    fn storage_locations(&self) -> Vec<StorageType>;
}

pub trait BundleBoxed {
    fn into_storage_boxed(self: Box<Self>) -> Vec<Box<dyn ComponentVec>>;
    fn push_storage_boxed(self: Box<Self>, table: &mut Table) -> Result<(), IntoStorageError>;
    fn ids_boxed(&self) -> Vec<TypeId>;
    fn storage_locations_boxed(&self) -> Vec<StorageType>;
}

impl<T: Bundle> BundleBoxed for T {
    fn into_storage_boxed(self: Box<Self>) -> Vec<Box<dyn ComponentVec>> {
        self.into_storage()
    }

    fn push_storage_boxed(self: Box<Self>, table: &mut Table) -> Result<(), IntoStorageError> {
        self.push_storage(table)
    }

    fn ids_boxed(&self) -> Vec<TypeId> {
        self.ids()
    }

    fn storage_locations_boxed(&self) -> Vec<StorageType> {
        self.storage_locations()
    }
}

impl Bundle for Box<dyn Bundle> {
    fn into_storage(self) -> Vec<Box<dyn ComponentVec>> {
        self.into_storage_boxed()
    }

    fn push_storage(self, table: &mut Table) -> Result<(), IntoStorageError> {
        self.push_storage_boxed(table)
    }

    fn ids(&self) -> Vec<TypeId> {
        self.ids_boxed()
    }

    fn storage_locations(&self) -> Vec<StorageType> {
        self.storage_locations_boxed()
    }
}

impl Bundle for Vec<Box<dyn ComponentVec>> {
    fn into_storage(self) -> Vec<Box<dyn ComponentVec>> {
        self
    }

    fn push_storage(self, table: &mut Table) -> Result<(), IntoStorageError> {
        for vec in self.iter() {
            let id = vec.stored_type_id();
            let index = self
                .ids()
                .iter()
                .enumerate()
                .find(|(_, other)| **other == id)
                .ok_or(IntoStorageError::MismatchedShape)?
                .0;
            table.storage[index]
                .try_append(vec.as_any())
                .map_err(|_| IntoStorageError::MismatchedShape)?;
        }

        Ok(())
    }

    fn ids(&self) -> Vec<TypeId> {
        self.iter().map(|vec| vec.stored_type_id()).collect()
    }

    fn storage_locations(&self) -> Vec<StorageType> {
        self.iter().map(|vec| vec.storage_type()).collect()
    }
}

macro_rules! bundle {
    ($($t:ident)*) => {
        #[allow(non_snake_case)]
        impl<$($t: std::fmt::Debug + Storage + Component + ComponentStorageType + TypeGetter + Clone + 'static),*> Bundle for ($($t,)*) {
            fn into_storage(self) -> Vec<Box<dyn ComponentVec>>  {
               let ($($t,)*) = self;
                vec![
                    $(Box::new(RefCell::new(vec![$t])),)*
                ]
            }

            fn push_storage(self, table: &mut Table) -> Result<(), IntoStorageError> {
               let ($(ref $t,)*) = self;
                let ids = self.ids();
                $(
                    let id = TypeId::of::<$t>();
                    let index = ids
                        .iter()
                        .enumerate()
                        .find(|(_, other)| **other == id)
                        .ok_or(IntoStorageError::MismatchedShape)?.0;
                    table.storage[index].try_push($t as &dyn Any).expect("sad");
                )*

                Ok(())
            }

            fn ids(&self) -> Vec<TypeId>  {
                vec![
                    $(TypeId::of::<$t>(),)*
                ]
            }

            fn storage_locations(&self) -> Vec<StorageType> {
                vec![
                    $(StorageType::of::<$t>(),)*
                ]
            }
        }
    };

    ($(($t:ident)),*, $next:ident) => {
        bundle!($(($t)),*);
        bundle!($(($t)),*, $next);
    }
}

bundle!(A);
bundle!(A B);
bundle!(A B C);
bundle!(A B C D);
bundle!(A B C D E);
bundle!(A B C D E F);
bundle!(A B C D E F G);
bundle!(A B C D E F G H);
bundle!(A B C D E F G H I);
bundle!(A B C D E F G H I J);
bundle!(A B C D E F G H I J K);
bundle!(A B C D E F G H I J K L);
