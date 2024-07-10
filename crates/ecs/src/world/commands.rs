use super::*;
use util::tracing::error;

#[derive(Debug)]
pub struct NewEntityCommands {
    data: Table,
}

impl NewEntityCommands {
    pub fn new<T: Bundle + 'static>(bundle: T, world: UnsafeWorldCell<'_>) -> Self {
        Self {
            data: Table::from_bundle(bundle, world),
        }
    }
}

#[derive(Debug)]
pub struct InsertComponent {
    pub component: DumbVec,
    pub type_id: TypeId,
    pub storage_type: StorageType,
}

impl InsertComponent {
    pub fn new<T: Send + Component + Storage>(component: T) -> Self {
        let type_id = std::any::TypeId::of::<T>();
        let storage_type = component.storage_type();

        let mut c = DumbVec::new(std::alloc::Layout::new::<T>(), 1, new_dumb_drop::<T>());
        c.push(component).unwrap();

        Self {
            type_id,
            storage_type,
            component: c,
        }
    }
}

#[derive(Debug)]
pub struct EntityCommands {
    pub entity: Entity,
    pub insert: Vec<InsertComponent>,
    pub remove: Vec<TypeId>,
    pub despawn: bool,
}

impl EntityCommands {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            insert: vec![],
            remove: vec![],
            despawn: false,
        }
    }

    pub fn insert<T: Send + Component + Storage>(&mut self, component: T) -> &mut Self {
        self.insert.push(InsertComponent::new(component));
        self
    }

    pub fn remove<T: Component>(&mut self) -> &mut Self {
        self.remove.push(TypeId::of::<T>());
        self
    }

    pub fn despawn(&mut self) {
        self.despawn = true;
    }

    pub fn commit<'w>(self, world: &mut UnsafeWorldCell<'w>) {
        let world = unsafe { world.read_and_write() };

        if self.despawn {
            world.despawn(self.entity);
            return;
        }

        let Some(entity) = world.get_entity(self.entity) else {
            error!("[EntityCommands] points to invalid [EntityMeta]");
            return;
        };

        let insert_ids = self
            .insert
            .iter()
            .map(|i| i.type_id.clone())
            .collect::<Vec<_>>();
        let remove_ids = self.remove.clone();
        let current_ids = world
            .archetypes
            .get(entity.location.archetype_id)
            .unwrap()
            .type_ids
            .clone();

        let mut new_ids = current_ids
            .iter()
            .filter(|id| !remove_ids.contains(id))
            .collect::<Vec<_>>();
        new_ids.extend(insert_ids.iter().filter(|id| !current_ids.contains(id)));
        let new_ids = new_ids.into_iter().cloned().collect::<Vec<_>>();

        world.apply_entity_commands(self.entity, new_ids, remove_ids, insert_ids, self.insert);
    }
}

#[derive(Debug)]
pub struct NewResourceCommands {
    pub resource: DumbVec,
    pub type_id: TypeId,
}

impl NewResourceCommands {
    pub fn new<R: Resource>(res: R) -> Self {
        let mut storage = DumbVec::new(std::alloc::Layout::new::<R>(), 1, new_dumb_drop::<R>());
        storage.push(res).unwrap();

        Self {
            resource: storage,
            type_id: std::any::TypeId::of::<R>(),
        }
    }
}

#[derive(Debug)]
// TODO: this should be deffered... maybe? At some point, will have to do in sequence anyway unless
// sorted, which may be slower
pub struct Commands<'w> {
    world: UnsafeWorldCell<'w>,
    entity_commands: VecDeque<EntityCommands>,
    new_entity_commands: VecDeque<NewEntityCommands>,
    new_resource_commands: VecDeque<NewResourceCommands>,
}

impl<'w> Commands<'w> {
    pub fn new(world: UnsafeWorldCell<'w>) -> Self {
        Self {
            world,
            entity_commands: VecDeque::new(),
            new_entity_commands: VecDeque::new(),
            new_resource_commands: VecDeque::new(),
        }
    }

    pub fn spawn<T: Bundle + 'static>(&mut self, bundle: T) {
        unsafe { self.world.read_and_write().spawn(bundle) };

        // self.new_entity_commands
        //     .push_back(NewEntityCommands::new(bundle));

        // self.new_entity_commands.back().unwrap()
    }

    pub fn get_entity(&mut self, entity: Entity) -> &mut EntityCommands {
        self.entity_commands.push_back(EntityCommands::new(entity));
        self.entity_commands.back_mut().unwrap()
    }

    pub fn insert_resource<R: Resource>(&mut self, res: R) {
        self.new_resource_commands
            .push_back(NewResourceCommands::new(res));
    }
}

impl<'w> Drop for Commands<'w> {
    fn drop(&mut self) {
        // self.new_entity_commands
        //     .drain(..)
        //     .for_each(|ec| unsafe {self.world.read_and_write().spawn_table(ec.data)};);

        self.entity_commands
            .drain(..)
            .for_each(|ec| ec.commit(&mut self.world));

        self.new_resource_commands
            .drain(..)
            .for_each(|nrc| unsafe { self.world.insert_stored_resource(nrc.resource, nrc.type_id) })
    }
}
