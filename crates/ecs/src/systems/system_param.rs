use crate::{
    access::{AccessType, ResourceAccess, SystemAccess},
    Archetype, CommandQueue, Commands, Event, EventReader, EventWriter, Events, Filter, Query,
    QueryData, QueryState, Res, ResMut, Resource, ResourceId, UnsafeWorldCell, World,
};

pub trait SystemParam {
    type State: Send + Sync + 'static;
    type Item<'world, 'state>;

    fn access(world: &mut World) -> SystemAccess;
    fn init_state(world: &mut World) -> Self::State;
    fn new_archetype(_archetype: &Archetype, _state: &mut Self::State) {}
    fn to_param<'w, 's>(
        state: &'s mut Self::State,
        world: UnsafeWorldCell<'w>,
    ) -> Self::Item<'w, 's>;
    fn apply_deffered(_world: &'_ mut World, _state: &'_ mut Self::State) {}
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
        unsafe { Commands::new(world.entities(), state) }
    }

    fn apply_deffered(world: &mut World, state: &mut Self::State) {
        state.apply_deffered(world);
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
