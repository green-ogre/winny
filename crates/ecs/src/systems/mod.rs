use std::fmt::Debug;

use util::tracing::trace;

use access::SystemAccess;
use ecs_macro::all_tuples;
use system_param::SystemParam;

use crate::{unsafe_world::UnsafeWorldCell, Archetype, OneShotSystems, World};

pub mod access;
pub mod sets;
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
    // _phantom: PhantomData<fn(Marker) -> Out>,
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
            // _phantom: PhantomData,
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

all_tuples!(impl_system, 1, 15, P);

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
            // _phantom: PhantomData,
        }
    }
}
