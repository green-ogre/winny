pub mod filter;
pub mod iter;
pub mod state;

use ecs_macro::all_tuples;
pub use filter::*;
pub use iter::*;
pub use state::*;

use std::{any::TypeId, cell::UnsafeCell, fmt::Debug, marker::PhantomData};

use crate::{
    access::{AccessType, ComponentAccess, SystemAccess},
    entity::Entity,
    unsafe_world::UnsafeWorldCell,
    ArchEntity, ArchId, Archetype, Component, ComponentId, Components, Table, TableId,
};

#[derive(Debug)]
pub struct StorageId {
    table_id: TableId,
    archetype_id: ArchId,
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not valid to request as data in a `Query`",
    label = "invalid `Query` data"
)]
pub trait QueryData: WorldQuery {
    type ReadOnly: ReadOnlyQueryData<State = <Self as WorldQuery>::State>;
}

pub trait ReadOnlyQueryData: QueryData<ReadOnly = Self> {}

pub trait WorldQuery: Send + Sync {
    type Fetch<'d>;
    type Item<'d>;
    type State;

    fn init_state(world: UnsafeWorldCell<'_>) -> Self::State;
    fn init_fetch<'d>(world: UnsafeWorldCell<'d>, state: &Self::State) -> Self::Fetch<'d>;
    fn set_table<'d>(fetch: &mut Self::Fetch<'d>, state: &Self::State, table: &'d Table);
    fn fetch<'d>(fetch: &mut Self::Fetch<'d>, arch_entity: &ArchEntity) -> Self::Item<'d>;

    fn system_access(components: &mut Components) -> SystemAccess;
    fn set_ids() -> Vec<TypeId>;
}

macro_rules! impl_query_data {
    (
        $(($params:ident, $idx:tt))*
    ) => {

        impl<$($params: QueryData + 'static),*> QueryData for ($($params,)*) {
            type ReadOnly = ($($params::ReadOnly,)*);
        }

        impl<$($params: ReadOnlyQueryData + 'static),*> ReadOnlyQueryData for ($($params,)*) {}

        impl<$($params: WorldQuery + 'static),*> WorldQuery for ($($params,)*) {
            type Item<'d> = ($($params::Item<'d>,)*);
            type Fetch<'d> = ($($params::Fetch<'d>,)*);

        type State = ($($params::State,)*);

        fn init_state(world: UnsafeWorldCell<'_>) -> Self::State {
            (
        $($params::init_state(world),)*
        )
        }

    fn init_fetch<'d>(
        world: UnsafeWorldCell<'d>,
        state: &Self::State,
    ) -> Self::Fetch<'d> {
            (
        $($params::init_fetch(world, &tuple_index!(state, $idx)),)*
        )

    }

    fn set_table<'d>(
        fetch: &mut Self::Fetch<'d>,
 state: &Self::State,
        table: &'d Table,
    )  {
        $($params::set_table(&mut tuple_index!(fetch, $idx),&tuple_index!(state, $idx), table);)*

    }

            fn fetch<'d>(fetch: &mut Self::Fetch<'d>, arch_entity: &ArchEntity) -> Self::Item<'d> {
                (
                    $($params::fetch(&mut tuple_index!(fetch, $idx), arch_entity),)*
                )
            }

    fn system_access(components: &mut Components) -> SystemAccess {
        let mut access = SystemAccess::default();

                $(access = access.with(
                        $params::system_access(components)
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

macro_rules! expr {
    ($x:expr) => {
        $x
    };
} // HACK
macro_rules! tuple_index {
    ($tuple:expr, $idx:tt) => {
        expr!($tuple.$idx)
    };
}

impl_query_data!((A, 0));
impl_query_data!((A, 0)(B, 1));
impl_query_data!((A, 0)(B, 1)(C, 2));
impl_query_data!((A, 0)(B, 1)(C, 2)(D, 3));
impl_query_data!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4));
impl_query_data!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5));
impl_query_data!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6));
impl_query_data!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7));
impl_query_data!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7)(J, 8));
impl_query_data!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7)(J, 8)(K, 9));

pub struct Mut<T>(PhantomData<T>);

impl<T: Component> QueryData for Mut<T> {
    type ReadOnly = T;
}

impl<T: Component> WorldQuery for Mut<T> {
    type Item<'d> = &'d mut T;
    type Fetch<'d> = Option<&'d mut [UnsafeCell<T>]>;
    type State = ComponentId;

    fn init_state(world: UnsafeWorldCell<'_>) -> Self::State {
        unsafe { world.components() }.id(&std::any::TypeId::of::<T>())
    }

    fn init_fetch<'d>(_world: UnsafeWorldCell<'d>, _state: &Self::State) -> Self::Fetch<'d> {
        None
    }

    fn set_table<'d>(fetch: &mut Self::Fetch<'d>, state: &Self::State, table: &'d Table) {
        *fetch = Some(unsafe { table.column_slice::<T>(state) });
    }

    fn fetch<'d>(fetch: &mut Self::Fetch<'d>, arch_entity: &ArchEntity) -> Self::Item<'d> {
        unsafe {
            fetch.as_ref().unwrap()[arch_entity.row.0]
                .get()
                .as_mut()
                .unwrap()
        }
    }

    fn system_access(components: &mut Components) -> SystemAccess {
        let id = components.register::<T>();
        SystemAccess::default().with_component(ComponentAccess::new(AccessType::Mutable, id))
    }

    fn set_ids() -> Vec<TypeId> {
        vec![std::any::TypeId::of::<T>()]
    }
}

impl<T: Component> QueryData for T {
    type ReadOnly = Self;
}

impl<T: Component> ReadOnlyQueryData for T {}

impl<T: Component> WorldQuery for T {
    type Item<'d> = &'d T;
    type Fetch<'d> = Option<&'d mut [UnsafeCell<T>]>;
    type State = ComponentId;

    fn init_state(world: UnsafeWorldCell<'_>) -> Self::State {
        unsafe { world.components() }.id(&std::any::TypeId::of::<T>())
    }

    fn init_fetch<'d>(_world: UnsafeWorldCell<'d>, _state: &Self::State) -> Self::Fetch<'d> {
        None
    }

    fn set_table<'d>(fetch: &mut Self::Fetch<'d>, state: &Self::State, table: &'d Table) {
        *fetch = Some(unsafe { table.column_slice::<T>(state) });
    }

    fn fetch<'d>(fetch: &mut Self::Fetch<'d>, arch_entity: &ArchEntity) -> Self::Item<'d> {
        unsafe {
            fetch.as_ref().unwrap()[arch_entity.row.0]
                .get()
                .as_ref()
                .unwrap()
        }
    }

    fn system_access(components: &mut Components) -> SystemAccess {
        let id = components.register::<T>();
        SystemAccess::default().with_component(ComponentAccess::new(AccessType::Immutable, id))
    }

    fn set_ids() -> Vec<TypeId> {
        vec![std::any::TypeId::of::<T>()]
    }
}

impl QueryData for Entity {
    type ReadOnly = Self;
}

impl ReadOnlyQueryData for Entity {}

impl WorldQuery for Entity {
    type Item<'d> = Entity;
    type Fetch<'d> = ();
    type State = ();

    fn init_state(_world: UnsafeWorldCell<'_>) -> Self::State {}
    fn init_fetch<'d>(_world: UnsafeWorldCell<'d>, _state: &Self::State) -> Self::Fetch<'d> {}
    fn set_table<'d>(_fetch: &mut Self::Fetch<'d>, _state: &Self::State, _table: &'d Table) {}
    fn fetch<'d>(_fetch: &mut Self::Fetch<'d>, arch_entity: &ArchEntity) -> Self::Item<'d> {
        arch_entity.entity
    }

    fn system_access(_components: &mut Components) -> SystemAccess {
        SystemAccess::default()
    }

    fn set_ids() -> Vec<TypeId> {
        vec![std::any::TypeId::of::<Entity>()]
    }
}

pub struct Query<'w, 's, T: QueryData, F = ()> {
    state: &'s QueryState<T, F>,
    world: UnsafeWorldCell<'w>,
}

impl<'w, 's, T: QueryData, F: Filter> Debug for Query<'w, 's, T, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Query").field("state", &self.state).finish()
    }
}

impl<'w, 's, T: QueryData, F: Filter> Query<'w, 's, T, F> {
    pub fn new(world: UnsafeWorldCell<'w>, state: &'s mut QueryState<T, F>) -> Self {
        Self { state, world }
    }

    pub fn iter(&self) -> QueryIter<'_, '_, T::ReadOnly, F> {
        self.state.read_only().new_iter(self.world)
    }

    pub fn iter_mut(&mut self) -> QueryIter<'_, '_, T, F> {
        self.state.new_iter(self.world)
    }

    pub fn get_single(
        &self,
    ) -> Result<<<T as QueryData>::ReadOnly as WorldQuery>::Item<'_>, SingleQueryError> {
        self.state.read_only().get_single(self.world)
    }

    pub fn get_single_mut(&mut self) -> Result<T::Item<'_>, SingleQueryError> {
        self.state.get_single(self.world)
    }
}
