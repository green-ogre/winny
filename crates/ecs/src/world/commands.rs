use crate::{IntoCondition, IntoSystem, OneShotSystems, System};

use super::*;

// WARN: CANNOT MULTITHREAD
pub struct Commands<'w, 's> {
    entities: &'w mut Entities,
    queue: &'s mut CommandQueue,
}

impl<'w, 's> Commands<'w, 's> {
    pub fn new(entities: &'w mut Entities, queue: &'s mut CommandQueue) -> Self {
        Self { entities, queue }
    }

    pub fn spawn<T: Bundle>(&mut self, bundle: T) -> EntityCommands {
        let entity = self.entities.reserve();
        let mut ec = EntityCommands::new(entity, self.queue);
        ec.insert(bundle);

        ec
    }

    pub fn get_entity(&mut self, entity: Entity) -> EntityCommands {
        EntityCommands::new(entity, self.queue)
    }

    pub fn insert_resource<R: Resource>(&mut self, res: R) -> &mut Self {
        self.push(insert_resource(res));
        self
    }

    pub fn run_system_once_when<S, C>(
        &mut self,
        system: impl IntoSystem<S>,
        condition: impl IntoCondition<C>,
    ) -> &mut Self {
        self.push(run_system_when::<S, C>(system, condition));
        self
    }

    fn push<C: Command>(&mut self, command: C) {
        self.queue.push(|world, one_shot_systems| {
            command.apply(world, one_shot_systems);
        });
    }
}

pub struct EntityCommands<'c> {
    entity: Entity,
    queue: &'c mut CommandQueue,
}

impl<'c> EntityCommands<'c> {
    pub fn new(entity: Entity, queue: &'c mut CommandQueue) -> Self {
        Self { entity, queue }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn insert<B: Bundle>(&mut self, bundle: B) -> &mut Self {
        self.push(insert::<B>(bundle));
        self
    }

    pub fn remove<B: Bundle>(&mut self) -> &mut Self {
        self.push(remove::<B>());
        self
    }

    pub fn despawn(mut self) {
        // TODO: maybe clear the previous commands? is there any reason not to
        self.push(despawn());
    }

    fn push<C: EntityCommand>(&mut self, command: C) {
        let entity = self.entity;
        self.queue.push(move |world, _| {
            command.apply(entity, world);
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
trait Command: 'static + Send + Sync {
    fn apply(self, world: &mut World, one_shot_systems: &mut OneShotSystems);
}
#[cfg(target_arch = "wasm32")]
trait Command: 'static {
    fn apply(self, world: &mut World, one_shot_systems: &mut OneShotSystems);
}

#[cfg(not(target_arch = "wasm32"))]
impl<F> Command for F
where
    F: FnOnce(&mut World, &mut OneShotSystems) + 'static + Send + Sync,
{
    fn apply(self, world: &mut World, one_shot_systems: &mut OneShotSystems) {
        self(world, one_shot_systems)
    }
}
#[cfg(target_arch = "wasm32")]
impl<F> Command for F
where
    F: FnOnce(&mut World, &mut OneShotSystems) + 'static,
{
    fn apply(self, world: &mut World, one_shot_systems: &mut OneShotSystems) {
        self(world, one_shot_systems)
    }
}

fn insert_resource<R: Resource>(res: R) -> impl Command {
    move |world: &mut World, _: &mut OneShotSystems| {
        util::tracing::trace_span!("inserting resource", name = %std::any::type_name::<R>());
        world.insert_resource(res);
    }
}

fn run_system_when<S, C>(
    system: impl IntoSystem<S>,
    condition: impl IntoCondition<C>,
) -> impl Command {
    let system = system.into_system();
    let mut condition = condition.into_system();
    move |world: &mut World, one_shot_systems: &mut OneShotSystems| {
        if !system.access(world).is_valid() {
            util::tracing::error!("run_system_when: Invalid system access, ignoring");
        } else if !condition.access(world).is_valid() {
            util::tracing::error!("run_system_when: Invalid condition access, ignoring");
        } else {
            condition.init_state(world);
            one_shot_systems.insert::<S, C>(system, condition);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
trait EntityCommand: 'static + Send + Sync {
    fn apply(self, entity: Entity, world: &mut World);
}
#[cfg(target_arch = "wasm32")]
trait EntityCommand: 'static {
    fn apply(self, entity: Entity, world: &mut World);
}

#[cfg(not(target_arch = "wasm32"))]
impl<F> EntityCommand for F
where
    F: FnOnce(Entity, &mut World) + 'static + Send + Sync,
{
    fn apply(self, entity: Entity, world: &mut World) {
        self(entity, world)
    }
}
#[cfg(target_arch = "wasm32")]
impl<F> EntityCommand for F
where
    F: FnOnce(Entity, &mut World) + 'static,
{
    fn apply(self, entity: Entity, world: &mut World) {
        self(entity, world)
    }
}

fn insert<B: Bundle>(bundle: B) -> impl EntityCommand {
    move |entity: Entity, world: &mut World| {
        util::tracing::trace_span!("insert", entity = ?entity);
        world.entity_mut(entity).insert::<B>(bundle)
    }
}

fn remove<B: Bundle>() -> impl EntityCommand {
    |entity: Entity, world: &mut World| {
        util::tracing::trace_span!("remove", entity = ?entity);
        world.entity_mut(entity).remove::<B>()
    }
}

fn despawn() -> impl EntityCommand {
    |entity: Entity, world: &mut World| {
        util::tracing::trace_span!("despawn", entity = ?entity);
        world.entity_mut(entity).despawn()
    }
}

#[derive(Default)]
pub struct CommandQueue {
    #[allow(clippy::type_complexity)]
    #[cfg(not(target_arch = "wasm32"))]
    queue: Vec<Box<dyn FnOnce(&mut World, &mut OneShotSystems) + Send + Sync>>,
    #[cfg(target_arch = "wasm32")]
    queue: Vec<Box<dyn FnOnce(&mut World, &mut OneShotSystems)>>,
}

impl CommandQueue {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn push<F>(&mut self, f: F)
    where
        F: FnOnce(&mut World, &mut OneShotSystems) + 'static + Send + Sync,
    {
        self.queue.push(Box::new(f));
    }
    #[cfg(target_arch = "wasm32")]
    pub fn push<F>(&mut self, f: F)
    where
        F: FnOnce(&mut World, &mut OneShotSystems) + 'static,
    {
        self.queue.push(Box::new(f));
    }

    pub fn apply_deffered(&mut self, world: &mut World, one_shot_systems: &mut OneShotSystems) {
        while let Some(command) = self.queue.pop() {
            command(world, one_shot_systems)
        }
    }
}
