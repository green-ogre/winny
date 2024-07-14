use super::*;

pub struct Commands<'w, 's> {
    entities: &'w Entities,
    queue: &'s mut CommandQueue,
}

impl<'w, 's> Commands<'w, 's> {
    pub fn new(entities: &'w Entities, queue: &'s mut CommandQueue) -> Self {
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

    pub fn insert_resource<R: Resource>(&mut self, res: R) {
        self.push(insert_resource(res));
    }

    fn push<C: Command>(&mut self, command: C) {
        self.queue.push(|world| {
            command.apply(world);
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
        self.queue.push(move |world| {
            command.apply(entity, world);
        });
    }
}

trait Command: 'static + Send + Sync {
    fn apply(self, world: &mut World);
}

impl<F> Command for F
where
    F: FnOnce(&mut World) + 'static + Send + Sync,
{
    fn apply(self, world: &mut World) {
        self(world)
    }
}

fn insert_resource<R: Resource>(res: R) -> impl Command {
    move |world: &mut World| {
        util::tracing::trace_span!("inserting resource", name = %std::any::type_name::<R>());
        world.insert_resource(res);
    }
}

trait EntityCommand: 'static + Send + Sync {
    fn apply(self, entity: Entity, world: &mut World);
}

impl<F> EntityCommand for F
where
    F: FnOnce(Entity, &mut World) + 'static + Send + Sync,
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
    queue: Vec<Box<dyn FnOnce(&mut World) + Send + Sync>>,
}

impl CommandQueue {
    pub fn push<F>(&mut self, f: F)
    where
        F: FnOnce(&mut World) + 'static + Send + Sync,
    {
        self.queue.push(Box::new(f));
    }

    pub fn pop(&mut self) -> Option<impl FnOnce(&mut World)> {
        self.queue.pop()
    }

    pub fn apply_deffered(&mut self, world: &mut World) {
        while let Some(command) = self.queue.pop() {
            command(world)
        }
    }
}
