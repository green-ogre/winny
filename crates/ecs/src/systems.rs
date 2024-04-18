use std::{
    cell::UnsafeCell,
    ffi::OsString,
    fmt::Debug,
    marker::PhantomData,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ecs_derive::all_tuples;
use libloading::{Error, Symbol};
use logger::{error, trace};

use crate::{
    unsafe_world::UnsafeWorldCell, AccessType, Commands, ComponentAccess, ComponentAccessFilter,
    Event, EventReader, EventWriter, Filter, Query, QueryData, QueryState, Res, ResMut, Resource,
    TypeGetter, TypeId, World,
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
            startup: vec![vec![], vec![]],
            pre_update: vec![vec![], vec![]],
            update: vec![vec![], vec![]],
            post_update: vec![vec![], vec![]],
            exit: vec![vec![], vec![]],
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

        let systems = systems.get();
        let system_sets = storage;
        let systems_access: Vec<_> = systems.iter().map(|s| s.access()).collect();

        for access in systems_access.iter() {
            access.validate_or_panic();
        }

        for (access, system) in systems_access.iter().zip(systems.into_iter()) {
            // The first set is reserved for read only queries
            if access.is_read_only() {
                system_sets[0].push(system);
                continue;
            }

            if !system_sets.last().unwrap().is_empty() {
                system_sets.push(vec![]);
            }

            for set in system_sets.iter_mut().skip(1) {
                // println!("{:#?}", access);
                if set.iter().all(|s| !s.access().conflicts_with(access)) {
                    set.push(system);
                    break;
                }
            }
        }

        system_sets.retain(|set| !set.is_empty());

        // for (i, set) in system_sets.iter().enumerate() {
        //     println!("SCHEDULE: {:?} -- SET {}", schedule, i + 1);
        //     println!();
        //     set.iter().for_each(|s| s.debug_print());
        //     println!();
        // }
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

            // TODO: apply deffered
        }
    }

    fn run_schedule_single_thread(&mut self, schedule: Schedule, world: &World) {
        let schedule = match schedule {
            Schedule::StartUp => &mut self.startup,
            Schedule::PreUpdate => &mut self.pre_update,
            Schedule::Update => &mut self.update,
            Schedule::PostUpdate => &mut self.post_update,
            Schedule::Exit => &mut self.exit,
        };

        for set in schedule.iter_mut() {
            for system in set.iter_mut() {
                let world = unsafe { world.as_unsafe_world() };
                let f = system.as_mut();

                f.run_unsafe(world);
            }

            // TODO: apply deffered
        }
    }

    pub fn startup(&mut self, world: &World) {
        self.run_schedule(Schedule::StartUp, &world);
    }

    pub fn run(&mut self, world: &World) {
        self.run_schedule(Schedule::PreUpdate, world);
        self.run_schedule(Schedule::Update, world);
        self.run_schedule(Schedule::PostUpdate, world);
    }

    pub fn exit(&mut self, world: &World) {
        self.run_schedule(Schedule::Exit, world);
    }

    pub fn startup_single_thread(&mut self, world: &World) {
        self.run_schedule_single_thread(Schedule::StartUp, world);
    }

    pub fn run_single_thread(&mut self, world: &World) {
        self.run_schedule_single_thread(Schedule::PreUpdate, world);
        self.run_schedule_single_thread(Schedule::Update, world);
        self.run_schedule_single_thread(Schedule::PostUpdate, world);
    }

    pub fn exit_single_thread(&mut self, world: &World) {
        self.run_schedule_single_thread(Schedule::Exit, world);
    }
}

#[derive(Debug)]
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

    fn access() -> Vec<Access>;
    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State;
    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's>;
}

pub struct Mut<T>(PhantomData<T>);

impl SystemParam for Commands {
    type State = TypeId;
    type Item<'world, 'state> = Commands;

    fn access() -> Vec<Access> {
        vec![Access::empty()]
    }

    // TODO: pass a reference to storage for this command to be cached
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

impl<E: Event + TypeGetter> SystemParam for EventReader<'_, E> {
    type State = TypeId;
    type Item<'world, 'state> = EventReader<'world, E>;

    fn access() -> Vec<Access> {
        vec![Access::new(
            vec![ComponentAccess::new::<E>(AccessType::Immutable)],
            vec![],
        )]
    }

    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State {
        E::type_id()
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        EventReader::new(world)
    }
}

impl<E: Event + TypeGetter> SystemParam for EventWriter<'_, E> {
    type State = TypeId;
    type Item<'world, 'state> = EventWriter<'world, E>;

    fn access() -> Vec<Access> {
        vec![Access::new(
            vec![ComponentAccess::new::<E>(AccessType::Mutable)],
            vec![],
        )]
    }

    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State {
        E::type_id()
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        EventWriter::new(world)
    }
}

impl<'a, T: 'static + Resource + TypeGetter> SystemParam for Res<'a, T> {
    type State = TypeId;
    type Item<'world, 'state> = Res<'state, T>;

    fn access() -> Vec<Access> {
        vec![Access::new(
            vec![ComponentAccess::new::<T>(AccessType::Immutable)],
            vec![],
        )]
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

impl<'a, T: 'static + Resource + TypeGetter> SystemParam for ResMut<'a, T> {
    type State = TypeId;
    type Item<'world, 'state> = ResMut<'state, T>;

    fn access() -> Vec<Access> {
        vec![Access::new(
            vec![ComponentAccess::new::<T>(AccessType::Mutable)],
            vec![],
        )]
    }

    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State {
        T::type_id()
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        unsafe { ResMut::from_ref_mut(world.resource_mut::<T>()) }
    }
}

impl<T: 'static + QueryData, F: 'static + Filter> SystemParam for Query<'_, '_, T, F> {
    type State = QueryState<T, F>;
    type Item<'world, 'state> = Query<'world, 'state, T, F>;

    fn access() -> Vec<Access> {
        vec![Access::new(T::set_access(), F::set_access())]
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
    fn access(&self) -> SystemAccess;
    fn run_unsafe<'w>(&mut self, world: UnsafeWorldCell<'w>);
    fn debug_print(&self);
}

pub trait SystemParamFunc<Marker>: 'static + Send + Sync {
    type Param: SystemParam;

    fn access() -> Vec<Access> {
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

pub struct Access {
    data: Vec<ComponentAccess>,
    filter: Vec<ComponentAccessFilter>,
    filtered_set: Vec<ComponentAccess>,
}

impl Access {
    pub fn empty() -> Self {
        Self {
            data: vec![],
            filter: vec![],
            filtered_set: vec![],
        }
    }

    pub fn new(data: Vec<ComponentAccess>, filter: Vec<ComponentAccessFilter>) -> Self {
        // TODO: what is filter doing here exactly?

        Self {
            filtered_set: data.clone(),
            data,
            filter,
        }
    }

    pub fn conflicts_with(&self, other: Access) -> bool {
        let mutable_access: Vec<_> = self.filtered_set.iter().filter(|a| a.is_mut()).collect();
        let other_mutable_access: Vec<_> =
            other.filtered_set.iter().filter(|a| a.is_mut()).collect();

        mutable_access
            .iter()
            .any(|a| other_mutable_access.iter().any(|o| a == o))
    }
}

#[derive(Debug)]
pub struct SystemAccess {
    filtered_set: Vec<ComponentAccess>,
}

impl SystemAccess {
    pub fn new(access: Vec<Access>) -> Self {
        let mut filtered_set = vec![];
        for mut a in access.into_iter() {
            filtered_set.append(&mut a.filtered_set);
        }

        Self { filtered_set }
    }

    pub fn validate_or_panic(&self) {
        let mutable_access: Vec<_> = self.filtered_set.iter().filter(|a| a.is_mut()).collect();
        let immutable_access: Vec<_> = self
            .filtered_set
            .iter()
            .filter(|a| a.is_read_only())
            .collect();

        for m in mutable_access.iter() {
            for i in immutable_access.iter() {
                if i.id() == m.id() {
                    panic!(
                        "Query attemps to access the same memory mutably and immutably: {:#?}, {:#?}",
                        i, m
                    );
                }
            }
        }
    }

    pub fn is_read_only(&self) -> bool {
        !self.filtered_set.iter().any(|a| a.is_mut())
    }

    pub fn conflicts_with(&self, other: &SystemAccess) -> bool {
        let mutable_access: Vec<_> = self.filtered_set.iter().filter(|a| a.is_mut()).collect();
        let immutable_access: Vec<_> = self
            .filtered_set
            .iter()
            .filter(|a| a.is_read_only())
            .collect();

        let other_mutable_access: Vec<_> =
            other.filtered_set.iter().filter(|a| a.is_mut()).collect();
        let other_immutable_access: Vec<_> = other
            .filtered_set
            .iter()
            .filter(|a| a.is_read_only())
            .collect();

        mutable_access
            .iter()
            .any(|s| other_immutable_access.iter().any(|o| s.id() == o.id()))
            || other_mutable_access
                .iter()
                .any(|o| immutable_access.iter().any(|s| s.id() == o.id()))
            || other_mutable_access
                .iter()
                .any(|o| mutable_access.iter().any(|s| s.id() == o.id()))
    }
}

impl<Marker, F> System for SystemFunc<Marker, F>
where
    Marker: 'static,
    F: SystemParamFunc<Marker>,
{
    fn access(&self) -> SystemAccess {
        SystemAccess::new(F::access())
    }

    fn run_unsafe<'w>(&mut self, world: UnsafeWorldCell<'w>) {
        self.f
            .run(F::Param::to_param(&mut F::Param::init_state(world), world))
    }

    fn debug_print(&self) {
        println!("{:?}", self._phantom);
        // println!("{:#?}", self.access());
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

            fn access() -> Vec<Access> {
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

impl<F: 'static + Send + Sync, P: 'static> IntoSystemStorage<P> for F
where
    F: SystemParamFunc<P>,
{
    fn get(self) -> Vec<Box<dyn System>> {
        vec![Box::new(self.into_system())]
    }
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

all_tuples!(impl_into_system_tuple, 2, 10, F, P);
