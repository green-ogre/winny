use std::any::Any;

use super::*;

pub trait SystemParam {
    type State: Send + Sync + 'static;
    type Item<'world, 'state>;

    fn access() -> Vec<Access>;
    fn init<'w>(_world: UnsafeWorldCell<'w>) {}
    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State;
    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's>;
}

impl SystemParam for Commands<'_> {
    type State = ();
    type Item<'world, 'state> = Commands<'world>;

    fn access() -> Vec<Access> {
        vec![Access::empty()]
    }

    fn init_state<'w>(_world: UnsafeWorldCell<'w>) -> Self::State {
        ()
    }

    fn to_param<'w, 's>(
        _state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        Commands::new(world)
    }
}

impl<E: Event> SystemParam for EventReader<'_, E> {
    type State = ResourceId;
    type Item<'world, 'state> = EventReader<'world, E>;

    fn access() -> Vec<Access> {
        vec![Access::new(
            vec![ComponentAccess::new::<E>(AccessType::Mutable)],
            vec![],
        )]
    }

    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State {
        unsafe { world.read_and_write().get_resource_id::<Events<E>>() }
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

    fn access() -> Vec<Access> {
        vec![Access::new(
            vec![ComponentAccess::new::<E>(AccessType::Mutable)],
            vec![],
        )]
    }

    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State {
        unsafe { world.read_and_write().get_resource_id::<Events<E>>() }
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        EventWriter::new(world, *state)
    }
}

impl<'a, T: 'static + Resource> SystemParam for Option<Res<'a, T>> {
    type State = ResourceId;
    type Item<'world, 'state> = Option<Res<'state, T>>;

    fn access() -> Vec<Access> {
        vec![Access::new(
            vec![ComponentAccess::new::<T>(AccessType::Immutable)],
            vec![],
        )]
    }

    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State {
        unsafe { world.read_and_write().get_resource_id::<T>() }
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        Res::try_new(world, *state)
    }
}

impl<T: 'static + Resource> SystemParam for Option<ResMut<'_, T>> {
    type State = ResourceId;
    type Item<'world, 'state> = Option<ResMut<'state, T>>;

    fn access() -> Vec<Access> {
        vec![Access::new(
            vec![ComponentAccess::new::<T>(AccessType::Mutable)],
            vec![],
        )]
    }

    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State {
        unsafe { world.read_and_write().get_resource_id::<T>() }
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        ResMut::try_new(world, *state)
    }
}

impl<'a, T: 'static + Resource> SystemParam for Res<'a, T> {
    type State = ResourceId;
    type Item<'world, 'state> = Res<'state, T>;

    fn access() -> Vec<Access> {
        vec![Access::new(
            vec![ComponentAccess::new::<T>(AccessType::Immutable)],
            vec![],
        )]
    }

    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State {
        unsafe { world.read_and_write().get_resource_id::<T>() }
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        Res::new(world, *state)
    }
}

impl<T: 'static + Resource> SystemParam for ResMut<'_, T> {
    type State = ResourceId;
    type Item<'world, 'state> = ResMut<'state, T>;

    fn access() -> Vec<Access> {
        vec![Access::new(
            vec![ComponentAccess::new::<T>(AccessType::Mutable)],
            vec![],
        )]
    }

    fn init_state<'w>(world: UnsafeWorldCell<'w>) -> Self::State {
        unsafe { world.read_and_write().get_resource_id::<T>() }
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        ResMut::new(world, *state)
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

    fn init<'w>(world: UnsafeWorldCell<'w>) {
        let mut component_ids = T::set_ids();
        component_ids.extend(F::set_access().iter().map(|a| a.type_id()));
        unsafe { world.read_and_write() }.register_components(&component_ids);
    }

    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's> {
        Query::new(world, state)
    }
}
