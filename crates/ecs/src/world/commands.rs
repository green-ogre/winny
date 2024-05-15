use super::*;

#[derive(Debug)]
pub struct NewEntityCommands {
    data: Table,
}

impl NewEntityCommands {
    pub fn new<T: Bundle + 'static>(bundle: T) -> Self {
        Self {
            data: Table::from_bundle(bundle),
        }
    }

    pub fn commit<'w>(self, world: &mut UnsafeWorldCell<'w>) {
        // world.spawn_table(self.data);
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
        c.push(component);

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

    pub fn commit<'w>(self, world: &mut UnsafeWorldCell<'w>) {
        unsafe { world.apply_entity_commands() };
    }
}

#[derive(Debug)]
pub struct Commands<'w> {
    world: UnsafeWorldCell<'w>,
    entity_commands: VecDeque<EntityCommands>,
    new_entity_commands: VecDeque<NewEntityCommands>,
}

impl<'w> Commands<'w> {
    pub fn new(world: UnsafeWorldCell<'w>) -> Self {
        Self {
            world,
            entity_commands: VecDeque::new(),
            new_entity_commands: VecDeque::new(),
        }
    }

    pub fn spawn<T: Bundle + 'static>(&mut self, bundle: T) -> &NewEntityCommands {
        self.new_entity_commands
            .push_back(NewEntityCommands::new(bundle));

        self.new_entity_commands.back().unwrap()
    }

    pub fn get_entity(&mut self, entity: Entity) -> &mut EntityCommands {
        self.entity_commands.push_back(EntityCommands::new(entity));

        self.entity_commands.back_mut().unwrap()
    }
}

impl<'w> Drop for Commands<'w> {
    fn drop(&mut self) {
        self.new_entity_commands
            .drain(..)
            .for_each(|ec| ec.commit(&mut self.world));

        self.entity_commands
            .drain(..)
            .for_each(|ec| ec.commit(&mut self.world));

        drop(self)
    }
}
