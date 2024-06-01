use std::{borrow::Cow, fmt::Debug, marker::PhantomData};

use ecs_macro::all_tuples;

use crate::{unsafe_world::UnsafeWorldCell, ComponentAccess, ComponentAccessFilter, SystemParam};

pub type StoredSystem = Box<dyn System<Out = ()>>;
pub type StoredCondition = Box<dyn System<Out = bool>>;

impl<O: 'static> Debug for dyn System<Out = O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("System")
            .field("name", &self.name())
            .field("access", &self.access())
            .finish()
    }
}

#[derive(Debug)]
enum SystemNode {
    Branch(SystemSet),
    Leaf {
        system: StoredSystem,
        condition: Option<StoredCondition>,
    },
}

impl SystemNode {
    pub fn system(system: StoredSystem) -> Self {
        Self::Leaf {
            system,
            condition: None,
        }
    }

    pub fn access(&self) -> Vec<SystemAccess> {
        match self {
            Self::Leaf { system, .. } => vec![system.access()],
            Self::Branch(s) => s.access(),
        }
    }
}

#[derive(Debug)]
pub struct SystemSet {
    nodes: Vec<SystemNode>,
    condition: Option<StoredCondition>,
    chain: bool,
}

impl SystemSet {
    pub fn new_system(system: StoredSystem) -> Self {
        Self {
            nodes: vec![SystemNode::system(system)],
            condition: None,
            chain: false,
        }
    }

    pub fn from_nodes(nodes: Vec<SystemNode>) -> Self {
        Self {
            nodes,
            condition: None,
            chain: false,
        }
    }

    fn access(&self) -> Vec<SystemAccess> {
        self.nodes.iter().map(|s| s.access()).flatten().collect()
    }

    fn try_into_nodes(&mut self) -> Result<Vec<SystemNode>, ()> {
        if self.chain || self.condition.is_some() {
            Err(())
        } else {
            Ok(self.nodes.drain(..).collect())
        }
    }

    pub fn join_disjoint(sets: Vec<Self>) -> Self {
        let mut parent_set = SystemSet {
            nodes: Vec::new(),
            condition: None,
            chain: false,
        };

        parent_set
            .nodes
            .extend(sets.into_iter().map(|s| SystemNode::Branch(s)));

        parent_set
    }

    pub fn run_if<M, F: IntoCondition<M>>(mut self, condition: F) -> Self {
        self.condition = Some(Box::new(condition.into_system()));
        self
    }

    pub fn chain(mut self) -> Self {
        self.chain = true;
        self
    }

    pub fn validate_or_panic(&self) {
        for node in self.nodes.iter() {
            match node {
                SystemNode::Branch(ss) => ss.validate_or_panic(),
                SystemNode::Leaf { system, condition } => {
                    system.access().validate_or_panic();
                    if let Some(condition) = condition {
                        condition.access().is_read_and_write().then(|| panic!());
                    }
                }
            }
        }
    }

    pub fn condense(&mut self) {
        if self.chain {
            return;
        }

        let mut parent_set = Vec::new();
        let mut empty_nodes = Vec::new();
        let mut new_nodes = Vec::new();

        for i in 0..self.nodes.len().saturating_sub(1) {
            for j in i + 1..self.nodes.len() {
                if !self.nodes[i].access().iter().any(|a| {
                    self.nodes[j]
                        .access()
                        .iter()
                        .any(|other| a.conflicts_with(other))
                }) {
                    let new_node = match &mut self.nodes[i] {
                        SystemNode::Leaf { .. } => panic!(),
                        SystemNode::Branch(s) => {
                            if let Ok(mut s_nodes) = s.try_into_nodes() {
                                let s_access = s_nodes
                                    .iter()
                                    .map(|n| n.access())
                                    .flatten()
                                    .collect::<Vec<_>>();

                                match &mut self.nodes[j] {
                                    SystemNode::Leaf { .. } => panic!(),
                                    SystemNode::Branch(b) => {
                                        if let Ok(mut b_nodes) = b.try_into_nodes() {
                                            s_nodes.append(&mut b_nodes);
                                            empty_nodes.push(i);
                                            empty_nodes.push(j);

                                            let b_access = b_nodes
                                                .iter()
                                                .map(|n| n.access())
                                                .flatten()
                                                .collect::<Vec<_>>();

                                            if b_access.iter().any(|a| {
                                                s_access.iter().any(|other| a.conflicts_with(other))
                                            }) {
                                                empty_nodes.push(i);
                                                Some(SystemNode::Branch(SystemSet {
                                                    nodes: s_nodes,
                                                    chain: true,
                                                    condition: None,
                                                }))
                                            } else {
                                                Some(SystemNode::Branch(SystemSet::from_nodes(
                                                    s_nodes,
                                                )))
                                            }
                                        } else {
                                            empty_nodes.push(i);
                                            Some(SystemNode::Branch(SystemSet::from_nodes(s_nodes)))
                                        }
                                    }
                                }
                            } else {
                                None
                            }
                        }
                    };

                    if let Some(node) = new_node {
                        new_nodes.push(node);
                    }
                }
            }
        }

        parent_set.extend(new_nodes.into_iter());
        for (i, node) in self.nodes.drain(..).enumerate() {
            if !empty_nodes.contains(&i) {
                parent_set.push(node);
            }
        }

        self.nodes.append(&mut parent_set);
    }

    pub fn run_tree(&mut self, world: UnsafeWorldCell<'_>) {
        if let Some(condition) = &mut self.condition {
            if !condition.run_unsafe(world) {
                return;
            }
        }

        std::thread::scope(|s| {
            let mut handles = Vec::new();
            for node in self.nodes.iter_mut() {
                match node {
                    SystemNode::Branch(ss) => ss.run_tree(world),
                    SystemNode::Leaf { system, condition } => {
                        if let Some(condition) = condition {
                            if !condition.run_unsafe(world) {
                                return;
                            }
                        }
                        if self.chain {
                            system.run_unsafe(world);
                        } else {
                            let h = s.spawn(|| system.run_unsafe(world));
                            handles.push(h);
                        }
                    }
                }
            }

            handles.into_iter().all(|h| {
                h.join()
                    .map_err(|err| logger::error!("problem joining system handles: {:?}", err))
                    .is_ok()
            });
        });
    }

    pub fn init_systems(&mut self, world: UnsafeWorldCell<'_>) {
        for node in self.nodes.iter_mut() {
            match node {
                SystemNode::Branch(ss) => ss.init_systems(world),
                SystemNode::Leaf { system, .. } => system.init(world),
            }
        }
    }
}

pub trait System: Send + Sync + 'static {
    type Out;

    fn access(&self) -> SystemAccess;
    fn name(&self) -> &str;
    fn init<'w>(&self, world: UnsafeWorldCell<'w>);
    fn run_unsafe<'w>(&mut self, world: UnsafeWorldCell<'w>) -> Self::Out;
}

mod sealed {
    pub struct Locked;
}

pub trait SystemParamFunc<Marker, Out>: 'static + Send + Sync {
    type Param: SystemParam;

    fn access() -> Vec<Access> {
        Self::Param::access()
    }

    fn run<'w, 's>(&mut self, params: <Self::Param as SystemParam>::Item<'w, 's>) -> Out;
}

pub struct SystemFunc<Marker, F, Out>
where
    F: SystemParamFunc<Marker, Out>,
{
    f: F,
    name: Cow<'static, str>,
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
            name: name.into(),
            param_state: None,
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

    pub fn is_read_and_write(&self) -> bool {
        self.filtered_set.iter().any(|a| a.is_mut())
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

        let res = mutable_access
            .iter()
            .any(|s| other_immutable_access.iter().any(|o| s.id() == o.id()))
            || other_mutable_access
                .iter()
                .any(|o| immutable_access.iter().any(|s| s.id() == o.id()))
            || other_mutable_access
                .iter()
                .any(|o| mutable_access.iter().any(|s| s.id() == o.id()));

        // println!("{:#?} && {:#?} => {}", self, other, res);
        res
    }
}

impl<Marker, F, Out> System for SystemFunc<Marker, F, Out>
where
    Marker: 'static,
    Out: 'static,
    F: SystemParamFunc<Marker, Out>,
{
    type Out = Out;

    fn access(&self) -> SystemAccess {
        SystemAccess::new(F::access())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn init(&self, world: UnsafeWorldCell<'_>) {
        <F::Param as SystemParam>::init(world);
    }

    fn run_unsafe<'w>(&mut self, world: UnsafeWorldCell<'w>) -> Self::Out {
        let state = self
            .param_state
            .get_or_insert(<F::Param as SystemParam>::init_state(world));
        self.f.run(F::Param::to_param(state, world))
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

            fn run<'w, 's>(&mut self, params: <($($params,)*) as SystemParam>::Item<'w, 's>) -> Out {
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
            name: name.into(),
            param_state: None,
            _phantom: PhantomData,
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

            fn init<'w>(world: UnsafeWorldCell<'w>) {
                $($params::init(world);)*
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
