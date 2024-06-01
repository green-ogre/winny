pub mod filter;
pub mod iter;
pub mod state;

use ecs_macro::all_tuples;
pub use filter::*;
pub use iter::*;
pub use state::*;

use std::{any::TypeId, borrow::Cow, fmt::Debug, marker::PhantomData};

use crate::{
    entity::Entity, unsafe_world::UnsafeWorldCell, ArchEntity, ArchId, Archetype, Component,
    ComponentId, Table, TableId,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AccessType {
    Immutable,
    Mutable,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ComponentAccess {
    access_type: AccessType,
    id: TypeId,
    name: Cow<'static, str>,
}

impl ComponentAccess {
    pub fn new<T: 'static>(access_type: AccessType) -> Self {
        Self {
            access_type,
            id: TypeId::of::<T>(),
            name: Cow::Borrowed(std::any::type_name::<T>()),
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ComponentAccessFilter {
    filter: AccessFilter,
    id: TypeId,
    name: Cow<'static, str>,
}

impl ComponentAccessFilter {
    pub fn new<T: Component>(filter: AccessFilter) -> Self {
        Self {
            id: TypeId::of::<T>(),
            filter,
            name: Cow::Borrowed(std::any::type_name::<T>()),
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

pub trait QueryData: Send + Sync {
    type ReadOnly<'d>;
    type Item<'d>;

    fn fetch<'d>(
        table: &'d Table,
        arch_entity: &ArchEntity,
        component_ids: impl Iterator<Item = ComponentId>,
    ) -> Self::Item<'d>;
    fn read_only<'d>(
        table: &'d Table,
        arch_entity: &ArchEntity,
        component_ids: impl Iterator<Item = ComponentId>,
    ) -> Self::ReadOnly<'d>;
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

            fn fetch<'d>(table: &'d Table, arch_entity: &ArchEntity, mut component_ids: impl Iterator<Item = ComponentId>) -> Self::Item<'d> {
                (
                    $($params::fetch(table, arch_entity, std::iter::once(component_ids.next().expect("is pre calculated and stored").clone())),)*
                )
            }

            fn read_only<'d>(table: &'d Table, arch_entity: &ArchEntity, mut component_ids: impl Iterator<Item = ComponentId>) -> Self::ReadOnly<'d> {
                (
                    $($params::read_only(table, arch_entity, std::iter::once(component_ids.next().unwrap())),)*
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

pub struct Mut<T>(PhantomData<T>);

impl<T: Component> QueryData for Mut<T> {
    type ReadOnly<'d> = &'d T;
    type Item<'d> = &'d mut T;

    fn fetch<'d>(
        table: &'d Table,
        arch_entity: &ArchEntity,
        mut component_ids: impl Iterator<Item = ComponentId>,
    ) -> Self::Item<'d> {
        unsafe { table.get_entity_mut::<T>(arch_entity.row, component_ids.next().unwrap()) }
    }

    fn read_only<'d>(
        table: &'d Table,
        arch_entity: &ArchEntity,
        mut component_ids: impl Iterator<Item = ComponentId>,
    ) -> Self::ReadOnly<'d> {
        unsafe { table.get_entity::<T>(arch_entity.row, component_ids.next().unwrap()) }
    }

    fn set_access() -> Vec<ComponentAccess> {
        vec![ComponentAccess::new::<T>(AccessType::Mutable)]
    }

    fn set_ids() -> Vec<TypeId> {
        vec![std::any::TypeId::of::<T>()]
    }
}

impl<T: Component> QueryData for T {
    type ReadOnly<'d> = &'d T;
    type Item<'d> = &'d T;

    fn fetch<'d>(
        table: &'d Table,
        arch_entity: &ArchEntity,
        mut component_ids: impl Iterator<Item = ComponentId>,
    ) -> Self::Item<'d> {
        unsafe { table.get_entity::<T>(arch_entity.row, component_ids.next().unwrap()) }
    }

    fn read_only<'d>(
        table: &'d Table,
        arch_entity: &ArchEntity,
        component_ids: impl Iterator<Item = ComponentId>,
    ) -> Self::ReadOnly<'d> {
        T::fetch(table, arch_entity, component_ids)
    }

    fn set_access() -> Vec<ComponentAccess> {
        vec![ComponentAccess::new::<T>(AccessType::Immutable)]
    }

    fn set_ids() -> Vec<TypeId> {
        vec![std::any::TypeId::of::<T>()]
    }
}

impl QueryData for Entity {
    type ReadOnly<'d> = Entity;
    type Item<'d> = Entity;

    fn fetch<'d>(
        _table: &'d Table,
        arch_entity: &ArchEntity,
        _component_ids: impl Iterator<Item = ComponentId>,
    ) -> Self::Item<'d> {
        arch_entity.entity
    }

    fn read_only<'d>(
        _table: &'d Table,
        arch_entity: &ArchEntity,
        _component_ids: impl Iterator<Item = ComponentId>,
    ) -> Self::ReadOnly<'d> {
        arch_entity.entity
    }

    fn set_access() -> Vec<ComponentAccess> {
        vec![ComponentAccess::new::<Entity>(AccessType::Immutable)]
    }

    fn set_ids() -> Vec<TypeId> {
        vec![std::any::TypeId::of::<Entity>()]
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

    pub fn get_single(&self) -> Result<T::ReadOnly<'_>, ()> {
        self.state.get_single(&self.world)
    }

    pub fn get_single_mut(&self) -> Result<T::Item<'_>, ()> {
        self.state.get_single_mut(&self.world)
    }
}
