use std::{
    cell::UnsafeCell,
    ffi::OsString,
    marker::PhantomData,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ecs_derive::all_tuples;
use libloading::{Error, Symbol};
use logging::{error, trace};

use crate::{
    unsafe_world::UnsafeWorldCell, AccessType, Commands, ComponentAccess, Filter, Query, QueryData,
    QueryState, Res, ResMut, Resource, TypeGetter, TypeId, World,
};

pub type StoredSystem = Box<dyn System>;
// A vec of systems that are run in parallel
pub type SystemSet = Vec<StoredSystem>;
// A vec of systems that are NOT run in parallel
pub type ChainedSystemSet = Vec<StoredSystem>;

pub struct Scheduler {
    startup: Vec<SystemSet>,
    pre_update: Vec<SystemSet>,
    update: Vec<SystemSet>,
    post_update: Vec<SystemSet>,
    exit: Vec<SystemSet>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            startup: vec![],
            pre_update: vec![],
            update: vec![],
            post_update: vec![],
            exit: vec![],
        }
    }

    pub fn add_systems<M, S: IntoSystemStorage<M>>(&mut self, schedule: Schedule, systems: S) {
        let storage = match schedule {
            Schedule::StartUp => &mut self.startup,
            Schedule::PreUpdate => &mut self.pre_update,
            Schedule::Update => &mut self.update,
            Schedule::PostUpdate => &mut self.post_update,
            Schedule::Exit => &mut self.exit,
        };

        storage.push(systems.get());
    }

    fn run_schedule(&mut self, schedule: Schedule, world: &World) {
        let schedule = match schedule {
            Schedule::StartUp => &mut self.startup,
            Schedule::PreUpdate => &mut self.pre_update,
            Schedule::Update => &mut self.update,
            Schedule::PostUpdate => &mut self.post_update,
            Schedule::Exit => &mut self.exit,
        };

        for set in schedule.iter_mut() {
            std::thread::scope(|s| {
                for system in set.iter_mut() {
                    let world = unsafe { world.as_unsafe_world() };
                    let f = system.as_mut();

                    s.spawn(move || {
                        f.run_unsafe(world);
                    });
                }
            })
        }
    }

    pub fn startup(&mut self, world: &World) {
        self.run_schedule(Schedule::StartUp, world);
    }

    pub fn run(&mut self, world: &World) {
        self.run_schedule(Schedule::PreUpdate, world);
        self.run_schedule(Schedule::Update, world);
        self.run_schedule(Schedule::PostUpdate, world);
    }

    pub fn exit(&mut self, world: &World) {
        self.run_schedule(Schedule::Exit, world);
    }
}

pub enum Schedule {
    StartUp,
    PreUpdate,
    Update,
    PostUpdate,
    Exit,
}

pub trait SystemParam {
    type State;
    type Item<'world, 'state>;

    fn access() -> Vec<ComponentAccess>;
    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State;
    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's>;
}

impl SystemParam for Commands {
    type State = TypeId;
    type Item<'world, 'state> = Commands;

    fn access() -> Vec<ComponentAccess> {
        vec![]
    }

    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State {
        TypeId::new(0)
    }

    fn to_param<'w, 's>(
        _state: &'s mut Self::State,
        _world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        Commands::new()
    }
}

impl<'a, T: 'static + Resource + TypeGetter> SystemParam for Res<'a, T> {
    type State = TypeId;
    type Item<'world, 'state> = Res<'state, T>;

    fn access() -> Vec<ComponentAccess> {
        vec![ComponentAccess::new::<T>(AccessType::Immutable)]
    }

    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State {
        T::type_id()
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        unsafe { Res::from_ref(world.resource::<T>()) }
    }
}

impl<T: 'static + QueryData, F: 'static + Filter> SystemParam for Query<'_, '_, T, F> {
    type State = QueryState<T, F>;
    type Item<'world, 'state> = Query<'world, 'state, T, F>;

    fn access() -> Vec<ComponentAccess> {
        T::set_access()
    }

    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State {
        QueryState::from_world_unsafe(world)
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        Query::new(world, state)
    }
}

pub trait System: Send + Sync {
    fn access(&self) -> Vec<ComponentAccess>;
    fn run_unsafe<'w>(&mut self, world: UnsafeWorldCell<'w>);
}

pub trait SystemParamFunc<Marker>: 'static + Send + Sync {
    type Param: SystemParam;

    fn access() -> Vec<ComponentAccess> {
        Self::Param::access()
    }

    fn run<'w, 's>(&mut self, params: <Self::Param as SystemParam>::Item<'w, 's>);
}

pub struct SystemFunc<Marker, F>
where
    F: SystemParamFunc<Marker>,
{
    f: F,
    _phantom: PhantomData<fn() -> Marker>,
}

impl<Marker, F> IntoSystem<Marker> for F
where
    Marker: 'static,
    F: SystemParamFunc<Marker>,
{
    type Sys = SystemFunc<Marker, F>;

    fn into_system(self) -> Self::Sys {
        SystemFunc {
            f: self,
            _phantom: PhantomData,
        }
    }
}

impl<Marker, F> System for SystemFunc<Marker, F>
where
    Marker: 'static,
    F: SystemParamFunc<Marker>,
{
    fn access(&self) -> Vec<ComponentAccess> {
        self.f.access()
    }

    fn run_unsafe<'w>(&mut self, world: UnsafeWorldCell<'w>) {
        unsafe {
            self.f
                .run(F::Param::to_param(&mut F::Param::init_state(world), world))
        };
    }
}

pub trait IntoSystem<Input> {
    type Sys: System;

    fn into_system(self) -> Self::Sys;
}

macro_rules! impl_system {
    (
        $($params:ident),*
    ) => {
        #[allow(non_snake_case)]
        impl<F: 'static + Send + Sync, $($params: SystemParam),*> SystemParamFunc<fn($($params,)*)> for F
            where
                for<'a, 'b> &'a mut F:
                    FnMut( $($params),* ) +
                    FnMut( $(<$params as SystemParam>::Item<'_, '_>),* ),
        {
            type Param = ($($params,)*);

            fn run<'w, 's>(&mut self, params: <($($params,)*) as SystemParam>::Item<'w, 's>) {
                fn call_inner<$($params),*>(
                    mut f: impl FnMut($($params),*),
                    $($params: $params),*
                ) {
                    f($($params),*)
                }

                let ($($params,)*) = params;
                call_inner(self, $($params),*);
            }
        }
    }
}

all_tuples!(impl_system, 1, 10, P);

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

macro_rules! impl_system_param {
    (
        $(($params:ident, $idx:tt))*
    ) => {
        impl<$($params: SystemParam),*> SystemParam for ($($params,)*) {
            type State = ($($params::State,)*);
            type Item<'w, 's> = ($($params::Item<'w, 's>,)*);

            fn access() -> Vec<ComponentAccess> {
                let mut vec = vec![];
                $(
                    vec.append(&mut $params::access());
                )*

                vec
            }

            fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State {
                (
                    $($params::init_state(world),)*
                )
            }

            fn to_param<'w, 's>(state: &'s mut Self::State, world: UnsafeWorldCell<'w>) -> Self::Item<'w, 's> {
                (
                    $($params::to_param(&mut tuple_index!(state, $idx), world),)*
                )
            }
        }
    }
}

impl_system_param!((A, 0));
impl_system_param!((A, 0)(B, 1));
impl_system_param!((A, 0)(B, 1)(C, 2));
impl_system_param!((A, 0)(B, 1)(C, 2)(D, 3));
impl_system_param!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4));
impl_system_param!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5));
impl_system_param!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6));
impl_system_param!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7));
impl_system_param!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7)(J, 8));
impl_system_param!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7)(J, 8)(K, 9));

pub trait IntoSystemStorage<M> {
    fn get(self) -> Vec<Box<dyn System>>;
}

macro_rules! impl_into_system_tuple {
    ($(($t:ident, $p:ident)),*) => {
        #[allow(non_snake_case)]
        impl<$($t: 'static + Send + Sync, $p: 'static),*> IntoSystemStorage<($($p,)*)> for ($($t,)*)
            where
                $($t: SystemParamFunc<$p>,)*
                {
                    fn get(self) -> Vec<Box<dyn System>> {
                        let ($($t,)*) = self;

                        vec![
                            $(Box::new($t.into_system()),)*
                        ]
                    }
                }
    }
}

all_tuples!(impl_into_system_tuple, 1, 10, F, P);
