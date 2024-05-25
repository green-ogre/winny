use super::*;

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
    pub component: Box<DumbVec>,
    pub type_id: TypeId,
    pub storage_type: StorageType,
}

impl InsertComponent {
    pub fn new<T: Send + Component + TypeGetter + Storage + Clone + Debug>(component: T) -> Self {
        let type_id = component.type_id();
        let storage_type = component.storage_type();

        let mut c = Box::new(DumbVec::new(
            std::alloc::Layout::new::<T>(),
            1,
            new_dumb_drop::<T>(),
        ));
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

    pub fn insert<T: Send + Component + TypeGetter + Storage + Debug + Clone>(
        &mut self,
        component: T,
    ) -> &mut Self {
        self.insert.push(InsertComponent::new(component));
        self
    }

    pub fn remove<T: Component + TypeGetter>(&mut self) -> &mut Self {
        self.remove.push(TypeId::of::<T>());
        self
    }

    pub fn despawn(&mut self) {
        self.despawn = true;
    }

    pub fn commit<'w>(self, _world: &mut UnsafeWorldCell<'w>) {}
}

#[derive(Debug)]
pub struct NewResourceCommands {
    pub resource: DumbVec,
    pub type_id: TypeId,
}

impl NewResourceCommands {
    pub fn new<R: Resource + TypeGetter>(res: R) -> Self {
        let mut storage = DumbVec::new(std::alloc::Layout::new::<R>(), 1, new_dumb_drop::<R>());
        storage.push(res).unwrap();

        Self {
            resource: storage,
            type_id: R::type_id(),
        }
    }
}

#[derive(Debug)]
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

    // TODO: this should be deffered
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

    pub fn insert_resource<R: Resource + TypeGetter>(&mut self, res: R) {
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
