pub mod entity_query;
pub mod filter;
mod impl_macros;

use ecs_derive::all_tuples;
pub use entity_query::*;
pub use filter::*;

use std::{
    borrow::{Borrow, BorrowMut},
    cell::{Ref, RefCell, RefMut},
    fmt::Debug,
    marker::PhantomData,
    slice::Iter,
};

use itertools::*;
use logging::{error, trace, warn};

use crate::{
    entity::Entity,
    unsafe_world::{self, UnsafeWorldCell},
    world, ArchEntity, ArchId, Archetype, Component, EntityMeta, Storage, StorageType, Table,
    TableId, TableRow, TypeGetter, TypeId, World,
};

#[derive(Debug, Clone, Copy)]
pub enum AccessType {
    Immutable,
    Mutable,
}

#[derive(Debug, Clone, Copy)]
pub struct ComponentAccess {
    access_type: AccessType,
    id: TypeId,
}

impl ComponentAccess {
    pub fn new<T: TypeGetter>(access_type: AccessType) -> Self {
        Self {
            access_type,
            id: TypeId::of::<T>(),
        }
    }
}

#[derive(Debug)]
pub struct StorageId {
    table_id: TableId,
    archetype_id: ArchId,
}

pub trait QueryData {
    type Item<'d>;

    fn fetch<'d>(table: &'d Table, arch_entity: &ArchEntity) -> Self::Item<'d>;
    fn set_access() -> Vec<ComponentAccess>;
    fn set_ids() -> Vec<TypeId>;
}

macro_rules! impl_query_data {
    (
        $($params:ident),*
    ) => {
        impl<$($params: QueryData + 'static),*> QueryData for ($($params,)*) {
            type Item<'d> = ($($params::Item<'d>,)*);

            fn fetch<'d>(table: &'d Table, arch_entity: &ArchEntity) -> Self::Item<'d> {
                (
                    $($params::fetch(table, arch_entity),)*
                )
            }

            fn set_access() -> Vec<ComponentAccess> {
                let mut access = vec![];
                $(access.append(
                        &mut $params::set_access()
                        );
                    )*
                access
            }

            fn set_ids() -> Vec<TypeId> {
                let mut ids = vec![];
                $(ids.append(
                        &mut $params::set_ids()
                        );
                    )*
                ids
            }
        }
    }
}

all_tuples!(impl_query_data, 1, 10, D);

impl<T: Component + TypeGetter> QueryData for T {
    type Item<'d> = &'d T;

    fn fetch<'d>(table: &'d Table, arch_entity: &ArchEntity) -> Self::Item<'d> {
        unsafe { table.get_entity::<T>(arch_entity.row) }
    }

    fn set_access() -> Vec<ComponentAccess> {
        vec![ComponentAccess::new::<T>(AccessType::Immutable)]
    }

    fn set_ids() -> Vec<TypeId> {
        vec![T::type_id()]
    }
}

impl QueryData for Entity {
    type Item<'d> = Entity;

    fn fetch<'d>(_table: &'d Table, arch_entity: &ArchEntity) -> Self::Item<'d> {
        arch_entity.entity
    }

    fn set_access() -> Vec<ComponentAccess> {
        vec![ComponentAccess::new::<Entity>(AccessType::Immutable)]
    }

    fn set_ids() -> Vec<TypeId> {
        vec![Entity::type_id()]
    }
}

// TODO: this can easily be cached in the world with an id
pub struct QueryState<T, F> {
    storages: Vec<StorageId>,
    component_access: Vec<ComponentAccess>,
    query: PhantomData<T>,
    filter: PhantomData<F>,
}

impl<T, F> Debug for QueryState<T, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryState")
            .field("query", &self.query)
            .field("filter", &self.filter)
            .field("storages", &self.storages)
            .field("component_access", &self.component_access)
            .finish()
    }
}

impl<T: QueryData, F: Filter> QueryState<T, F> {
    pub fn from_world(world: &World) -> Self {
        let storages = unsafe {
            world
                .as_unsafe_world()
                .read_only()
                .archetypes
                .iter()
                .filter(|arch| arch.contains_query::<T>())
                .filter(|arch| F::condition(arch))
                .map(|arch| StorageId {
                    table_id: arch.table_id,
                    archetype_id: arch.id,
                })
                .collect()
        };

        Self::new(storages)
    }

    pub fn from_world_unsafe<'w>(world: UnsafeWorldCell<'w>) -> Self {
        let storages = unsafe {
            world
                .read_only()
                .archetypes
                .iter()
                .filter(|arch| arch.contains_query::<T>())
                .filter(|arch| F::condition(arch))
                .map(|arch| StorageId {
                    table_id: arch.table_id,
                    archetype_id: arch.id,
                })
                .collect()
        };

        Self::new(storages)
    }

    pub fn new(storages: Vec<StorageId>) -> Self {
        Self {
            storages,
            component_access: T::set_access(),
            query: PhantomData,
            filter: PhantomData,
        }
    }

    pub fn component_access(&self) -> Vec<ComponentAccess> {
        self.component_access.clone()
    }

    pub fn new_iter<'w>(&self, world: &'w UnsafeWorldCell<'w>) -> QueryIter<'_, T, F> {
        let storage: Vec<_> = self
            .storages
            .iter()
            .map(|id| unsafe {
                (
                    world.read_only().archetypes.get(id.archetype_id),
                    world.read_only().tables.get(id.table_id),
                )
            })
            .collect();

        QueryIter::new(storage)
    }
}

pub struct QueryIterStorage<'s> {
    storage: Vec<(&'s Archetype, &'s Table)>,
    table: &'s Table,
    archetype: &'s Archetype,
}

impl<'s> QueryIterStorage<'s> {
    pub fn new(storage: Vec<(&'s Archetype, &'s Table)>) -> Self {
        let (archetype, table) = storage.first().expect("cannot be empty");

        Self {
            table,
            archetype,
            storage,
        }
    }
}

pub struct QueryIter<'s, T, F> {
    cursor: std::slice::Iter<'s, ArchEntity>,
    storage: Option<QueryIterStorage<'s>>,
    next_storage: usize,
    query: PhantomData<T>,
    filter: PhantomData<F>,
}

impl<'s, T, F> QueryIter<'s, T, F> {
    fn new(storage: Vec<(&'s Archetype, &'s Table)>) -> Self {
        if storage.first().is_none() {
            return Self::empty();
        }

        let storage = QueryIterStorage::new(storage);

        Self {
            cursor: storage.archetype.entities.iter(),
            next_storage: 1,
            storage: Some(storage),
            query: PhantomData,
            filter: PhantomData,
        }
    }

    fn empty() -> Self {
        Self {
            cursor: [].iter(),
            next_storage: 0,
            storage: None,
            query: PhantomData,
            filter: PhantomData,
        }
    }
}

impl<'s, T: QueryData, F> Iterator for QueryIter<'s, T, F> {
    type Item = T::Item<'s>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.storage.is_none() {
            return None;
        }

        let Some(arch_entity) = self.cursor.next().or_else(|| {
            let storage = self.storage.as_mut().unwrap();

            let Some(next_storage) = storage.storage.get(self.next_storage) else {
                return None;
            };

            self.next_storage += 1;
            storage.archetype = &next_storage.0;
            storage.table = &next_storage.1;

            self.cursor = storage.archetype.entities.iter();
            let Some(next) = self.cursor.next() else {
                return None;
            };

            Some(next)
        }) else {
            return None;
        };

        Some(T::fetch(self.storage.as_ref().unwrap().table, arch_entity))
    }
}

pub struct Query<'w, 's, T, F = ()> {
    world: UnsafeWorldCell<'w>,
    pub state: &'s QueryState<T, F>,
}

impl<'w, 's, T, F> Debug for Query<'w, 's, T, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Query").field("state", &self.state).finish()
    }
}

impl<'w, 's, T: QueryData, F: Filter> Query<'w, 's, T, F> {
    pub fn new(unsafe_world: UnsafeWorldCell<'w>, state: &'s mut QueryState<T, F>) -> Self {
        // TODO: This is really something that should be cached. I'm not sure where to put it though

        Self {
            state,
            world: unsafe_world,
        }
    }

    pub fn iter(&self) -> QueryIter<'s, T, F> {
        self.state.new_iter(&self.world)
    }
}

// pub trait WorldQuery {
//     type Output<'w>;
//
//     // fn iter<'w>(self) -> impl Iterator<Item = Self::Output<'w>>;
//     // fn get(&self, id: Entity) -> Result<Self::Output, ()>;
//     // fn get_single(&self) -> Result<Self::Output, ()>;
// }
//
// pub trait WorldQueryMut {
//     type Output;
//
//     // fn iter_mut(&mut self) -> impl Iterator<Item = Self::Output>;
//     // fn get_mut(&self, id: Entity) -> Result<Self::Output, ()>;
//     // fn get_single_mut(&self) -> Result<Self::Output, ()>;
// }

// fn log_failed_query(
//     archetype: &Archetype,
//     table: &Table,
//     type_name: String,
//     type_id: TypeId,
//     storage_type: StorageType,
// ) {
//     let buf = format!(
//         "Type Name: {}, Type Id: {:?}, Storage Type: {:?},\n\n{:#?}, {:#?}",
//         type_name, type_id, storage_type, archetype, table
//     );
//     std::fs::write("temp/invalid_query.txt", &buf).unwrap()
// }

// fn map_vec<'a, T: Storage + TypeGetter + Component>(
//     archetype: &'a Archetype,
//     table: &'a Table,
// ) -> impl Iterator<Item = Ref<'a, T>> {
//     let len = archetype.entities.len();
//
//     (0..len).map(|i| match T::storage_type() {
//         StorageType::SparseSet => todo!(),
//         StorageType::Table => {
//             let vec = table.borrow_component_vec::<T>().unwrap_or_else(|| {
//                 log_failed_query(
//                     archetype,
//                     table,
//                     T::type_name().as_string(),
//                     T::type_id(),
//                     T::storage_type(),
//                 );
//                 panic!("Logged failed query");
//             });
//
//             Ref::map(vec, |v| &v[archetype.entities[i].1 .0])
//         }
//     })
// }

// fn map_vec_mut<'a, T: Storage + TypeGetter + Component>(
//     archetype: &'a Archetype,
//     table: &'a Table,
// ) -> impl Iterator<Item = RefMut<'a, T>> {
//     let len = archetype.entities.len();
//
//     (0..len).map(|i| match T::storage_type() {
//         StorageType::SparseSet => todo!(),
//         StorageType::Table => {
//             let vec = table.borrow_mut_component_vec::<T>().unwrap_or_else(|| {
//                 log_failed_query(
//                     archetype,
//                     table,
//                     T::type_name().as_string(),
//                     T::type_id(),
//                     T::storage_type(),
//                 );
//                 panic!("Logged failed query");
//             });
//             RefMut::map(vec, |v| &mut v[archetype.entities[i].1 .0])
//         }
//     })
// }

// impl<T: QueryData, F: Filter> WorldQuery for Query<'_, T, F> {
// type Output<'w> = T::Item<'w>;

// fn iter(&self) -> impl Iterator<Item = Self::Output<'w>> {
//     unsafe { self.state.new_iter() }
// }

// fn get(&self, id: Entity) -> Result<Self::Output, ()> {
//     let meta = self.world.get_entity(id).ok_or(())?;
//     let len = self.world.archetypes[meta.location.archetype_id]
//         .entities
//         .len();

//     let id_set = vec![T::type_id()];
//     if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
//         return Err(());
//     }

//     (0..len)
//         .map(|_| {
//             map_vec::<T>(
//                 &self.world.archetypes[meta.location.archetype_id],
//                 &self.world.tables[meta.location.table_id],
//             )
//         })
//         .flatten()
//         .nth(meta.location.table_row.0)
//         .ok_or(())
// }

// fn get_single(&self) -> Result<Self::Output, ()> {
//     self.world
//         .archetypes
//         .iter()
//         .filter(|arch| arch.contains::<T>())
//         .filter(|arch| F::condition(arch))
//         .map(|arch| map_vec::<T>(arch, &self.world.tables[arch.table_id]))
//         .flatten()
//         .exactly_one()
//         .map_err(|_| ())
// }
// }

// impl<'b, T: TypeGetter + Component + Storage, F: Filter> WorldQueryMut for Query<'b, T, F> {
// type Output = RefMut<'b, T>;

// fn iter_mut(&mut self) -> impl Iterator<Item = Self::Output> {
//     self.world
//         .archetypes
//         .iter()
//         .filter(|arch| arch.contains::<T>())
//         .filter(|arch| F::condition(arch))
//         .map(|arch| map_vec_mut::<T>(arch, &self.world.tables[arch.table_id]))
//         .flatten()
// }

// fn get_mut(&self, id: Entity) -> Result<Self::Output, ()> {
//     let meta = self.world.get_entity(id).ok_or(())?;
//     let len = self.world.tables[meta.location.table_id].len;

//     let id_set = vec![T::type_id()];
//     if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
//         return Err(());
//     }

//     (0..len)
//         .map(|_| {
//             map_vec_mut::<T>(
//                 &self.world.archetypes[meta.location.archetype_id],
//                 &self.world.tables[meta.location.table_id],
//             )
//         })
//         .flatten()
//         .nth(meta.location.archetype_index)
//         .ok_or(())
// }

// fn get_single_mut(&self) -> Result<Self::Output, ()> {
//     self.world
//         .archetypes
//         .iter()
//         .filter(|arch| arch.contains::<T>())
//         .filter(|arch| F::condition(arch))
//         .map(|arch| map_vec_mut::<T>(arch, &self.world.tables[arch.table_id]))
//         .flatten()
//         .exactly_one()
//         .map_err(|_| ())
// }
//}
