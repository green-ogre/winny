pub mod entity_query;
pub mod filter;
mod impl_macros;
pub mod iter;
pub mod state;

use ecs_derive::all_tuples;
pub use entity_query::*;
pub use filter::*;
pub use iter::*;
pub use state::*;

use std::{
    borrow::{Borrow, BorrowMut},
    cell::{Ref, RefCell, RefMut},
    fmt::Debug,
    marker::PhantomData,
    slice::Iter,
};

use itertools::*;
use logger::{error, trace, warn};

use crate::{
    entity::Entity,
    unsafe_world::{self, UnsafeWorldCell},
    world, ArchEntity, ArchId, Archetype, Component, EntityMeta, Mut, Storage, StorageType, Table,
    TableId, TableRow, TypeGetter, TypeId, TypeName, World,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AccessType {
    Immutable,
    Mutable,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ComponentAccess {
    access_type: AccessType,
    id: TypeId,

    #[cfg(debug_assertions)]
    name: TypeName,
}

impl ComponentAccess {
    pub fn new<T: TypeGetter>(access_type: AccessType) -> Self {
        Self {
            access_type,
            id: TypeId::of::<T>(),

            #[cfg(debug_assertions)]
            name: TypeName::of::<T>(),
        }
    }

    pub fn id(&self) -> TypeId {
        self.id
    }

    pub fn is_read_only(&self) -> bool {
        self.access_type == AccessType::Immutable
    }

    pub fn is_mut(&self) -> bool {
        self.access_type == AccessType::Mutable
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AccessFilter {
    With,
    Without,
    Or,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ComponentAccessFilter {
    filter: AccessFilter,
    id: TypeId,

    #[cfg(debug_assertions)]
    name: TypeName,
}

impl ComponentAccessFilter {
    pub fn new<T: TypeGetter>(filter: AccessFilter) -> Self {
        Self {
            id: TypeId::of::<T>(),
            filter,

            #[cfg(debug_assertions)]
            name: TypeName::of::<T>(),
        }
    }

    pub fn with(&self) -> bool {
        self.filter == AccessFilter::With
    }

    pub fn without(&self) -> bool {
        self.filter == AccessFilter::Without
    }

    pub fn or(&self) -> bool {
        self.filter == AccessFilter::Or
    }
}

#[derive(Debug)]
pub struct StorageId {
    table_id: TableId,
    archetype_id: ArchId,
}

pub trait QueryData {
    type ReadOnly<'d>;
    type Item<'d>;

    fn fetch<'d>(table: &'d Table, arch_entity: &ArchEntity) -> Self::Item<'d>;
    fn read_only<'d>(table: &'d Table, arch_entity: &ArchEntity) -> Self::ReadOnly<'d>;
    fn set_access() -> Vec<ComponentAccess>;
    fn set_ids() -> Vec<TypeId>;
}

macro_rules! impl_query_data {
    (
        $($params:ident),*
    ) => {
        impl<$($params: QueryData + 'static),*> QueryData for ($($params,)*) {
            type ReadOnly<'d> = ($($params::ReadOnly<'d>,)*);
            type Item<'d> = ($($params::Item<'d>,)*);

            fn fetch<'d>(table: &'d Table, arch_entity: &ArchEntity) -> Self::Item<'d> {
                (
                    $($params::fetch(table, arch_entity),)*
                )
            }

            fn read_only<'d>(table: &'d Table, arch_entity: &ArchEntity) -> Self::ReadOnly<'d> {
                (
                    $($params::read_only(table, arch_entity),)*
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

impl<T: Component + TypeGetter> QueryData for Mut<T> {
    type ReadOnly<'d> = &'d T;
    type Item<'d> = &'d mut T;

    fn fetch<'d>(table: &'d Table, arch_entity: &ArchEntity) -> Self::Item<'d> {
        unsafe { table.get_entity_mut::<T>(arch_entity.row) }
    }

    fn read_only<'d>(table: &'d Table, arch_entity: &ArchEntity) -> Self::ReadOnly<'d> {
        unsafe { table.get_entity::<T>(arch_entity.row) }
    }

    fn set_access() -> Vec<ComponentAccess> {
        vec![ComponentAccess::new::<T>(AccessType::Mutable)]
    }

    fn set_ids() -> Vec<TypeId> {
        vec![T::type_id()]
    }
}

impl<T: Component + TypeGetter> QueryData for T {
    type ReadOnly<'d> = &'d T;
    type Item<'d> = &'d T;

    fn fetch<'d>(table: &'d Table, arch_entity: &ArchEntity) -> Self::Item<'d> {
        unsafe { table.get_entity::<T>(arch_entity.row) }
    }

    fn read_only<'d>(table: &'d Table, arch_entity: &ArchEntity) -> Self::ReadOnly<'d> {
        T::fetch(table, arch_entity)
    }

    fn set_access() -> Vec<ComponentAccess> {
        vec![ComponentAccess::new::<T>(AccessType::Immutable)]
    }

    fn set_ids() -> Vec<TypeId> {
        vec![T::type_id()]
    }
}

impl QueryData for Entity {
    type ReadOnly<'d> = Entity;
    type Item<'d> = Entity;

    fn fetch<'d>(_table: &'d Table, arch_entity: &ArchEntity) -> Self::Item<'d> {
        arch_entity.entity
    }

    fn read_only<'d>(table: &'d Table, arch_entity: &ArchEntity) -> Self::ReadOnly<'d> {
        arch_entity.entity
    }

    fn set_access() -> Vec<ComponentAccess> {
        vec![ComponentAccess::new::<Entity>(AccessType::Immutable)]
    }

    fn set_ids() -> Vec<TypeId> {
        vec![Entity::type_id()]
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
        Self {
            state,
            world: unsafe_world,
        }
    }

    pub fn iter(&self) -> QueryIter<'s, T, F> {
        self.state.new_iter(&self.world)
    }

    pub fn iter_mut(&self) -> QueryIterMut<'s, T, F> {
        self.state.new_iter_mut(&self.world)
    }

    pub fn get_single(&self) -> anyhow::Result<T::ReadOnly<'_>> {
        self.state.get_single(&self.world)
    }

    pub fn get_single_mut(&self) -> anyhow::Result<T::Item<'_>> {
        self.state.get_single_mut(&self.world)
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
