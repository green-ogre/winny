use std::{fmt::Debug, marker::PhantomData};

use util::tracing::trace;

use access::SystemAccess;
use ecs_macro::all_tuples;
use system_param::SystemParam;

use crate::{unsafe_world::UnsafeWorldCell, Archetype, OneShotSystems, Schedule, World};

pub mod access;
pub mod system_param;

pub type StoredSystem = Box<dyn System<Out = ()>>;
pub type StoredCondition = Box<dyn System<Out = bool>>;

impl<O: 'static> Debug for dyn System<Out = O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("System")
            .field("name", &self.name())
            .finish()
    }
}

#[derive(Debug)]
pub struct ScheduleBuilder {
    pub sets: Vec<SystemSet>,
    pub tag: Schedule,
}

impl ScheduleBuilder {
    pub fn new(tag: Schedule) -> Self {
        Self {
            sets: Vec::new(),
            tag,
        }
    }

    pub fn push_set(&mut self, set: SystemSet) {
        self.sets.push(set);
    }

    pub fn build_schedule(self, world: &mut World) -> Vec<SystemSet> {
        optimize_schedule(world, self.sets)
    }
}

fn optimize_schedule(world: &mut World, sets: Vec<SystemSet>) -> Vec<SystemSet> {
    let mut schedule = Vec::new();

    for set in sets.iter() {
        set.validate_nodes_or_panic(world);
    }

    for set in sets.into_iter() {
        if set.is_invalid(world) {
            // TODO: pull apart sets?
            // TODO: not fullproof yet, platform for instance does not correctly see nested
            // invalid accesses
            schedule.push(set.chain());
        } else {
            // TODO: combine sets?
            schedule.push(set);
        }
    }

    schedule
}

#[derive(Debug)]
pub enum Node {
    Leaf(StoredSystem),
    Branch(SystemSet),
}

impl Node {
    pub fn access(&self, world: &mut World) -> Vec<SystemAccess> {
        match self {
            Self::Leaf(system) => vec![system.access(world)],
            Self::Branch(set) => set.access(world),
        }
    }

    pub fn init_state(&mut self, world: &mut World) {
        match self {
            Self::Leaf(system) => system.init_state(world),
            Self::Branch(set) => set.init_state(world),
        }
    }
}

#[derive(Debug)]
pub struct SystemSet {
    pub nodes: Vec<Node>,
    pub condition: Option<StoredCondition>,
    pub chain: bool,
}

impl SystemSet {
    pub fn join_disjoint(sets: Vec<Self>) -> Self {
        let mut nodes = Vec::new();

        for mut set in sets.into_iter() {
            if set.nodes.len() == 1 && set.condition.is_none() {
                match set.nodes.pop().unwrap() {
                    Node::Leaf(system) => nodes.push(Node::Leaf(system)),
                    Node::Branch(set) => panic!("{:#?}", set),
                }
            } else {
                nodes.push(Node::Branch(set));
            }
        }

        Self {
            chain: false,
            condition: None,
            nodes,
        }
    }

    pub fn new_system(system: StoredSystem) -> Self {
        Self {
            nodes: vec![Node::Leaf(system)],
            condition: None,
            chain: false,
        }
    }

    pub fn new_nodes(nodes: Vec<Node>) -> Self {
        Self {
            nodes,
            condition: None,
            chain: false,
        }
    }

    fn access(&self, world: &mut World) -> Vec<SystemAccess> {
        self.nodes
            .iter()
            .map(|s| s.access(world))
            .flatten()
            .collect::<Vec<_>>()
    }

    pub fn run_if<M, F: IntoCondition<M>>(mut self, condition: F) -> Self {
        self.condition = Some(Box::new(condition.into_system()));
        self
    }

    pub fn chain(mut self) -> Self {
        self.chain = true;
        self
    }

    pub fn validate_nodes_or_panic(&self, world: &mut World) {
        for system in self.access(world).iter() {
            system.validate_or_panic();
        }
    }

    pub fn is_invalid(&self, world: &mut World) -> bool {
        if self.chain {
            return false;
        }

        let access = self.access(world);

        for i in 0..access.len() - 1 {
            for j in i + 1..access.len() {
                if access[i].conflicts_with(&access[j]) {
                    return true;
                }
            }
        }

        false
    }

    pub fn init_state(&mut self, world: &mut World) {
        for node in self.nodes.iter_mut() {
            node.init_state(world);
        }
    }

    pub fn run(&mut self, _world: &mut World) {
        // if let Some(condition) = &mut self.condition {
        //     if !condition.run_unsafe(unsafe { world.as_unsafe_world() }) {
        //         return;
        //     }
        // }

        // std::thread::scope(|s| {
        //     let world = unsafe { world.as_unsafe_world() };
        //     let mut handles = Vec::new();
        //     for node in self.nodes.iter_mut() {
        //         match node {
        //             Node::Leaf(system) => {
        //                 if self.chain {
        //                     system.run_unsafe(world);
        //                 } else {
        //                     let h = s.spawn(|| system.run_unsafe(world));
        //                     handles.push(h);
        //                 }
        //             }
        //             Node::Branch(set) => {
        //                 if self.chain {
        //                     set.run(world);
        //                 } else {
        //                     let h = s.spawn(|| set.run(world));
        //                     handles.push(h);
        //                 }
        //             }
        //         }
        //     }

        //     handles.into_iter().all(|h| {
        //         h.join()
        //             .map_err(|_| {
        //                 exit(1);
        //             })
        //             .is_ok()
        //     });
        // });

        // for node in self.nodes.iter_mut() {
        //     match node {
        //         Node::Leaf(system) => system.apply_deffered(world),
        //         Node::Branch(set) => set.apply_deffered(world),
        //     }
        // }
    }

    pub fn apply_deffered(&mut self, _world: &mut World) {
        // for node in self.nodes.iter_mut() {
        //     match node {
        //         Node::Leaf(system) => // system.apply_deffered(world),
        //         Node::Branch(set) => set.apply_deffered(world),
        //     }
        // }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub trait System: Send + Sync + 'static {
    type Out;

    fn access(&self, world: &mut World) -> SystemAccess;
    fn name(&self) -> &str;
    fn init_state(&mut self, world: &mut World);
    fn new_archetype(&mut self, archetype: &Archetype);
    fn run_unsafe(&mut self, world: UnsafeWorldCell<'_>) -> Self::Out;
    fn apply_deffered(&mut self, world: &mut World, one_shot_systems: &mut OneShotSystems);
}

#[cfg(target_arch = "wasm32")]
pub trait System: 'static {
    type Out;

    fn access(&self, world: &mut World) -> SystemAccess;
    fn name(&self) -> &str;
    fn init_state(&mut self, world: &mut World);
    fn new_archetype(&mut self, archetype: &Archetype);
    fn run_unsafe(&mut self, world: UnsafeWorldCell<'_>) -> Self::Out;
    fn apply_deffered(&mut self, world: &mut World, one_shot_systems: &mut OneShotSystems);
}

pub trait SystemParamFunc<Marker, Out>: 'static + Send + Sync {
    type Param: SystemParam;

    fn access(world: &mut World) -> SystemAccess {
        Self::Param::access(world)
    }
    fn run(&mut self, params: <Self::Param as SystemParam>::Item<'_, '_>) -> Out;
}

pub struct SystemFunc<Marker, F, Out>
where
    F: SystemParamFunc<Marker, Out>,
{
    f: F,
    name: &'static str,
    param_state: Option<<F::Param as SystemParam>::State>,
    _phantom: PhantomData<fn() -> Out>,
}

impl<Marker, F> IntoSystem<Marker> for F
where
    Marker: 'static,
    F: SystemParamFunc<Marker, ()>,
{
    type Sys = SystemFunc<Marker, F, ()>;

    fn into_system(self) -> Self::Sys {
        let name = std::any::type_name::<F>();

        SystemFunc {
            f: self,
            name,
            param_state: None,
            _phantom: PhantomData,
        }
    }
}

impl<Marker, F, Out> System for SystemFunc<Marker, F, Out>
where
    Marker: 'static,
    Out: 'static,
    F: SystemParamFunc<Marker, Out>,
{
    type Out = Out;

    fn access(&self, world: &mut World) -> SystemAccess {
        SystemAccess::default().with(F::access(world))
    }

    fn name(&self) -> &str {
        self.name
    }

    fn init_state(&mut self, world: &mut World) {
        let state = <F::Param as SystemParam>::init_state(world);
        trace!("Initializing ['System'] state: {}", self.name);
        let _ = self.param_state.insert(state);
    }

    fn new_archetype(&mut self, archetype: &Archetype) {
        trace!("new_archetype");
        F::Param::new_archetype(archetype, self.param_state.as_mut().unwrap());
    }

    fn run_unsafe(&mut self, world: UnsafeWorldCell<'_>) -> Self::Out {
        let state = self.param_state.as_mut().unwrap();
        let _span = util::tracing::trace_span!("system", name = %self.name).entered();
        let out = self.f.run(F::Param::to_param(state, world));
        trace!("exiting");

        out
    }

    fn apply_deffered(&mut self, world: &mut World, one_shot_systems: &mut OneShotSystems) {
        let _span = util::tracing::trace_span!("apply_deffered", name = %self.name).entered();
        F::Param::apply_deffered(world, self.param_state.as_mut().unwrap(), one_shot_systems);
        trace!("exiting");
    }
}

pub trait IntoSystem<Input> {
    type Sys: System<Out = ()>;

    fn into_system(self) -> Self::Sys;
}

macro_rules! impl_system {
    (
        $($params:ident),*
    ) => {
        #[allow(non_snake_case)]
        impl<F: 'static + Send + Sync, $($params: SystemParam),*, Out> SystemParamFunc<fn($($params,)*) -> Out, Out> for F
            where
                for<'a, 'b> &'a mut F:
                    FnMut( $($params),* ) -> Out +
                    FnMut( $(<$params as SystemParam>::Item<'_, '_>),* ) -> Out,
        {
            type Param = ($($params,)*);

#[allow(clippy::too_many_arguments)]
            fn run(&mut self, params: <($($params,)*) as SystemParam>::Item<'_, '_>) -> Out {
                fn call_inner<$($params),*, Out>(
                    mut f: impl FnMut($($params),*) -> Out,
                    $($params: $params),*
                ) -> Out {
                    f($($params),*)
                }

                let ($($params,)*) = params;
                call_inner(self, $($params),*)
            }
        }
    }
}

all_tuples!(impl_system, 1, 10, P);

pub trait IntoCondition<Input> {
    type Sys: System<Out = bool>;

    fn into_system(self) -> Self::Sys;
}

impl<Marker, F> IntoCondition<Marker> for F
where
    Marker: 'static,
    F: SystemParamFunc<Marker, bool>,
{
    type Sys = SystemFunc<Marker, F, bool>;

    fn into_system(self) -> Self::Sys {
        let name = std::any::type_name::<F>();

        SystemFunc {
            f: self,
            name,
            param_state: None,
            _phantom: PhantomData,
        }
    }
}

pub trait IntoSystemStorage<Marker>
where
    Self: Sized,
{
    fn into_set(self) -> SystemSet;
    fn chain(self) -> SystemSet {
        self.into_set().chain()
    }
    fn run_if<M, F: IntoCondition<M>>(self, condition: F) -> SystemSet {
        self.into_set().run_if(condition)
    }
}

impl IntoSystemStorage<()> for SystemSet {
    fn into_set(self) -> SystemSet {
        self
    }
}

impl<M, F> IntoSystemStorage<M> for F
where
    F: IntoSystem<M>,
{
    fn into_set(self) -> SystemSet {
        SystemSet::new_system(Box::new(self.into_system()))
    }
}

macro_rules! impl_into_system_tuple {
    ($(($t:ident, $p:ident)),*) => {
        #[allow(non_snake_case)]
        impl<$($t: 'static + Send + Sync, $p: 'static),*> IntoSystemStorage<($($p,)*)> for ($($t,)*)
            where
                $($t: IntoSystemStorage<$p>,)*
                {
                    fn into_set(self) -> SystemSet {
                        let ($($t,)*) = self;

                        SystemSet::join_disjoint(vec![$($t.into_set(),)*])
                    }
                }
    }
}

all_tuples!(impl_into_system_tuple, 1, 10, F, P);
