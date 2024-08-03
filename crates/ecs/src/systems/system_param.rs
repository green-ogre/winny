use crate::{
    access::{AccessType, ResourceAccess, SystemAccess},
    Archetype, CommandQueue, Commands, Event, EventReader, EventWriter, Events, Filter,
    OneShotSystems, Query, QueryData, QueryState, Res, ResMut, Resource, ResourceId, Take,
    UnsafeWorldCell, World,
};

pub trait SystemParam {
    #[cfg(not(target_arch = "wasm32"))]
    type State: Send + Sync + 'static;
    #[cfg(target_arch = "wasm32")]
    type State: 'static;
    type Item<'world, 'state>;

    fn access(world: &mut World) -> SystemAccess;
    fn init_state(world: &mut World) -> Self::State;
    fn new_archetype(_archetype: &Archetype, _state: &mut Self::State) {}
    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's>;
    fn apply_deffered(
        _world: &'_ mut World,
        _state: &'_ mut Self::State,
        _one_shot_systems: &mut OneShotSystems,
    ) {
    }
}

impl SystemParam for Commands<'_, '_> {
    type State = CommandQueue;
    type Item<'world, 'state> = Commands<'world, 'state>;

    fn access(_world: &mut World) -> SystemAccess {
        SystemAccess::default()
    }

    fn init_state<'w>(_world: &mut World) -> Self::State {
        CommandQueue::default()
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        unsafe { Commands::new(world.entities_mut(), state) }
    }

    fn apply_deffered(
        world: &mut World,
        state: &mut Self::State,
        one_shot_systems: &mut OneShotSystems,
    ) {
        state.apply_deffered(world, one_shot_systems);
    }
}

impl<E: Event> SystemParam for EventReader<'_, E> {
    type State = ResourceId;
    type Item<'world, 'state> = EventReader<'world, E>;

    fn access(world: &mut World) -> SystemAccess {
        let id = world.get_resource_id::<Events<E>>();
        SystemAccess::default().with_resource(ResourceAccess::new(AccessType::Mutable, id))
    }

    fn init_state<'w>(world: &mut World) -> Self::State {
        world.get_resource_id::<Events<E>>()
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        EventReader::new(world, *state)
    }
}

impl<E: Event> SystemParam for EventWriter<'_, E> {
    type State = ResourceId;
    type Item<'world, 'state> = EventWriter<'world, E>;

    fn access(world: &mut World) -> SystemAccess {
        let id = world.get_resource_id::<Events<E>>();
        SystemAccess::default().with_resource(ResourceAccess::new(AccessType::Mutable, id))
    }

    fn init_state<'w>(world: &mut World) -> Self::State {
        world.get_resource_id::<Events<E>>()
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        EventWriter::new(world, *state)
    }
}

impl<'a, R: 'static + Resource> SystemParam for Option<Res<'a, R>> {
    type State = ResourceId;
    type Item<'world, 'state> = Option<Res<'world, R>>;

    fn access(world: &mut World) -> SystemAccess {
        let id = world.get_resource_id::<R>();
        SystemAccess::default().with_resource(ResourceAccess::new(AccessType::Immutable, id))
    }

    fn init_state<'w>(world: &mut World) -> Self::State {
        world.get_resource_id::<R>()
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        unsafe { world.try_get_resource_ref_by_id(*state) }
    }
}

impl<R: 'static + Resource> SystemParam for Option<ResMut<'_, R>> {
    type State = ResourceId;
    type Item<'world, 'state> = Option<ResMut<'world, R>>;

    fn access(world: &mut World) -> SystemAccess {
        let id = world.get_resource_id::<R>();
        SystemAccess::default().with_resource(ResourceAccess::new(AccessType::Mutable, id))
    }

    fn init_state<'w>(world: &mut World) -> Self::State {
        world.get_resource_id::<R>()
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        unsafe { world.try_get_resource_mut_ref_by_id::<R>(*state) }
    }
}

impl<'a, R: 'static + Resource> SystemParam for Res<'a, R> {
    type State = ResourceId;
    type Item<'world, 'state> = Res<'world, R>;

    fn access(world: &mut World) -> SystemAccess {
        let id = world.get_resource_id::<R>();
        SystemAccess::default().with_resource(ResourceAccess::new(AccessType::Immutable, id))
    }

    fn init_state<'w>(world: &mut World) -> Self::State {
        world.get_resource_id::<R>()
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        unsafe { world.get_resource_ref_by_id(*state) }
    }
}

impl<R: 'static + Resource> SystemParam for ResMut<'_, R> {
    type State = ResourceId;
    type Item<'world, 'state> = ResMut<'world, R>;

    fn access(world: &mut World) -> SystemAccess {
        let id = world.get_resource_id::<R>();
        SystemAccess::default().with_resource(ResourceAccess::new(AccessType::Mutable, id))
    }

    fn init_state<'w>(world: &mut World) -> Self::State {
        world.get_resource_id::<R>()
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        unsafe { world.get_resource_mut_ref_by_id(*state) }
    }
}

impl<R: Resource> SystemParam for Take<R> {
    type State = ResourceId;
    type Item<'world, 'state> = Take<R>;

    fn access(world: &mut World) -> SystemAccess {
        let id = world.get_resource_id::<R>();
        SystemAccess::default().with_resource(ResourceAccess::new(AccessType::Mutable, id))
    }

    fn init_state(world: &mut World) -> Self::State {
        world.get_resource_id::<R>()
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        unsafe {
            world
                .take_resource_by_id::<R>(*state)
                .map(|res| Take::new(res))
                .expect("resource is is storage")
        }
    }
}

impl<R: Resource> SystemParam for Option<Take<R>> {
    type State = ResourceId;
    type Item<'world, 'state> = Option<Take<R>>;

    fn access(world: &mut World) -> SystemAccess {
        let id = world.get_resource_id::<R>();
        SystemAccess::default().with_resource(ResourceAccess::new(AccessType::Mutable, id))
    }

    fn init_state(world: &mut World) -> Self::State {
        world.get_resource_id::<R>()
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        unsafe {
            world
                .take_resource_by_id::<R>(*state)
                .map(|res| Take::new(res))
        }
    }
}

impl<T: 'static + QueryData, F: 'static + Filter> SystemParam for Query<'_, '_, T, F> {
    type State = QueryState<T, F>;
    type Item<'world, 'state> = Query<'world, 'state, T, F>;

    fn access(world: &mut World) -> SystemAccess {
        let components = unsafe { world.as_unsafe_world().components_mut() };
        T::system_access(components).with(F::system_access(components))
    }

    fn init_state<'w>(world: &mut World) -> Self::State {
        QueryState::from_world(world)
    }

    fn new_archetype(archetype: &Archetype, state: &mut Self::State) {
        state.new_archetype(archetype);
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        Query::new(world, state)
    }
}

macro_rules! expr {
    ($x:expr) => {
        $x
    };
}
macro_rules! tuple_index {
    ($tuple:expr, $idx:tt) => {
        expr!($tuple.$idx)
    };
}

macro_rules! impl_system_param {
    ($(($params:ident, $idx:tt))*) => {
        impl<$($params: SystemParam),*> SystemParam for ($($params,)*) {
            type State = ($($params::State,)*);
            type Item<'w, 's> = ($($params::Item<'w, 's>,)*);

            fn access(world: &mut World) -> SystemAccess {
                let mut access = SystemAccess::default();
                $(
                    access = access.with($params::access(world));
                )*
                access
            }

            fn init_state(world: &mut World) -> Self::State {
                (
                    $($params::init_state(world),)*
                )
            }

            fn new_archetype(archetype: &crate::storage::Archetype, state: &mut Self::State) {
                $($params::new_archetype(archetype, &mut tuple_index!(state, $idx));)*
            }

            fn to_param<'w, 's>(state: &'s mut Self::State, world: UnsafeWorldCell<'w>) -> Self::Item<'w, 's> {
                (
                    $($params::to_param(&mut tuple_index!(state, $idx), world),)*
                )
            }

            fn apply_deffered(world: &'_ mut World, state: &'_ mut Self::State, one_shot_systems: &mut OneShotSystems) {
                $(
                    $params::apply_deffered(world, &mut tuple_index!(state, $idx), one_shot_systems);
                )*
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
impl_system_param!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7)(J, 8)(K, 9)(L, 10));
impl_system_param!((A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7)(J, 8)(K, 9)(L, 10)(M, 11));
impl_system_param!(
    (A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7)(J, 8)(K, 9)(L, 10)(M, 11)(N, 12)
);
impl_system_param!(
    (A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7)(J, 8)(K, 9)(L, 10)(M, 11)(N, 12)(O, 13)
);
impl_system_param!(
    (A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7)(J, 8)(K, 9)(L, 10)(M, 11)(N, 12)(O, 13)(P, 14)
);
impl_system_param!(
    (A, 0)(B, 1)(C, 2)(D, 3)(E, 4)(F, 5)(G, 6)(H, 7)(J, 8)(K, 9)(L, 10)(M, 11)(N, 12)(O, 13)(P, 14)(
        Q, 15
    )
);
