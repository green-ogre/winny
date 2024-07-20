use super::*;

#[derive(Debug)]
struct SchedulerBuilder {
    schedules: Vec<ScheduleBuilder>,
}

impl Default for SchedulerBuilder {
    fn default() -> Self {
        let schedules = Schedule::VALUES().map(ScheduleBuilder::new).collect();

        Self { schedules }
    }
}

impl SchedulerBuilder {
    pub fn new() -> Self {
        Self {
            schedules: Vec::new(),
        }
    }

    pub fn add_systems<M, S: IntoSystemStorage<M>>(&mut self, schedule: Schedule, systems: S) {
        self.schedules[schedule as usize].push_set(systems.into_set());
    }

    pub fn build_schedules(&mut self, world: &mut World) -> Vec<ScheduleExecuter> {
        let mut executers = Vec::new();
        for schedule in self.schedules.drain(..) {
            executers.push(ScheduleExecuter::new(
                schedule.tag,
                schedule.build_schedule(world),
            ));
        }

        executers
    }
}

#[derive(Debug, Default)]
pub struct OneShotSystems {
    systems: SparseArray<usize, (StoredSystem, StoredCondition)>,
    // SparseArray does not track `alive` values
    num_systems: usize,
}

impl OneShotSystems {
    pub fn len(&self) -> usize {
        self.num_systems
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert<S, C>(
        &mut self,
        system: impl System<Out = ()>,
        condition: impl System<Out = bool>,
    ) {
        self.num_systems += 1;
        self.systems
            .insert_in_first_empty((Box::new(system), Box::new(condition)));
    }

    pub fn new_archetype(&mut self, arch: &Archetype) {
        for (_, condition) in self.systems.iter_mut() {
            condition.new_archetype(arch);
        }
    }

    pub fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (usize, (&mut StoredSystem, &mut StoredCondition))> {
        self.systems
            .iter_indexed_mut()
            .map(|(i, t)| (i, (&mut t.0, &mut t.1)))
    }

    pub fn remove_indexes(&mut self, indexes: impl Iterator<Item = usize>) {
        for index in indexes {
            self.num_systems -= 1;
            let _ = self.systems.take(index);
        }
    }

    pub fn append(&mut self, other: OneShotSystems) {
        for set in other.systems.into_iter() {
            self.systems.push(set);
        }
    }
}

// TODO: fix system tree
#[derive(Debug, Default)]
pub struct Scheduler {
    pub(crate) executers: Vec<ScheduleExecuter>,
    one_shot_systems: OneShotSystems,
    builder: SchedulerBuilder,
}

impl Scheduler {
    pub fn add_systems<M, S: IntoSystemStorage<M>>(&mut self, schedule: Schedule, systems: S) {
        self.builder.add_systems::<M, S>(schedule, systems);
    }

    pub fn build_schedule(&mut self, world: &mut World) {
        self.executers
            .append(&mut self.builder.build_schedules(world));

        self.executers
            .iter_mut()
            .for_each(|e| e.init_systems(world));
    }

    pub fn init_systems(&mut self, world: &mut World) {
        for executer in self.executers.iter_mut() {
            executer.init_systems(world);
        }
    }

    pub fn new_archetype(&mut self, archetype: &Archetype) {
        for executer in self.executers.iter_mut() {
            executer.new_archetype(archetype, &mut self.one_shot_systems);
        }
    }

    pub fn run(&mut self, world: &mut World) {
        self.run_schedule(world, Schedule::Platform);
        self.run_schedule(world, Schedule::PreUpdate);
        self.run_schedule(world, Schedule::Update);
        self.run_schedule(world, Schedule::PostUpdate);
    }

    pub fn startup(&mut self, world: &mut World) {
        self.run_schedule(world, Schedule::PreStartUp);
        self.run_schedule(world, Schedule::StartUp);
    }

    pub fn flush_events(&mut self, world: &mut World) {
        self.run_schedule(world, Schedule::FlushEvents);
    }

    pub fn resized(&mut self, world: &mut World) {
        self.run_schedule(world, Schedule::Resized);
    }

    pub fn render(&mut self, world: &mut World) {
        self.run_schedule(world, Schedule::PrepareRender);
        self.run_schedule(world, Schedule::PreRender);
        self.run_schedule(world, Schedule::Render);
        self.run_schedule(world, Schedule::PostRender);
        self.run_schedule(world, Schedule::Present);
    }

    pub fn exit(&mut self, world: &mut World) {
        self.run_schedule(world, Schedule::Exit);
    }

    fn run_schedule(&mut self, world: &mut World, schedule: Schedule) {
        let executer = &mut self.executers[schedule as usize];
        let _span = util::tracing::trace_span!("schedule", name = ?executer.tag).entered();
        executer.run(world, &mut self.one_shot_systems);
        executer.apply_deffered(world, &mut self.one_shot_systems);
    }
}

// TODO: pull out backend schedules
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum Schedule {
    Platform,
    PreUpdate,
    Update,
    PostUpdate,
    PreStartUp,
    StartUp,
    Exit,
    FlushEvents,
    Resized,
    SubmitEncoder,
    PrepareRender,
    PreRender,
    Render,
    PostRender,
    Present,
}

#[derive(Debug)]
pub(crate) struct ScheduleExecuter {
    pub(crate) tag: Schedule,
    pub(crate) systems: Vec<StoredSystem>,
    pub(crate) conditions: Vec<Option<StoredCondition>>,
    pub(crate) archetypes_len: usize,
}

impl ScheduleExecuter {
    pub fn new(tag: Schedule, sets: Vec<SystemSet>) -> Self {
        let mut systems = Vec::new();
        let mut conditions = Vec::new();

        for mut set in sets.into_iter() {
            for node in set.nodes.into_iter() {
                match node {
                    Node::Leaf(system) => {
                        systems.push(system);
                        conditions.push(set.condition.take());
                    }
                    _ => panic!(),
                }
            }
        }

        Self {
            tag,
            systems,
            conditions,
            archetypes_len: 0,
        }
    }

    pub fn init_systems(&mut self, world: &mut World) {
        for system in self.systems.iter_mut() {
            system.init_state(world);
        }
    }

    pub fn apply_deffered(&mut self, world: &mut World, one_shot_systems: &mut OneShotSystems) {
        // for system in self.systems.iter_mut() {
        //     system.apply_deffered(world);
        // }

        while self.archetypes_len < world.archetypes.len() {
            let arch = world
                .archetypes
                .get(ArchId::new(self.archetypes_len))
                .expect("valid id");
            self.new_archetype(arch, one_shot_systems);
            self.archetypes_len += 1;
        }

        if one_shot_systems.is_empty() {
            return;
        }

        let mut indexes = Vec::with_capacity(one_shot_systems.len());
        let mut temp = OneShotSystems::default();
        for (index, (one_shot, condition)) in one_shot_systems.iter_mut() {
            if condition.run_unsafe(unsafe { world.as_unsafe_world() }) {
                indexes.push(index);
                one_shot.init_state(world);
                one_shot.run_unsafe(unsafe { world.as_unsafe_world() });
                one_shot.apply_deffered(world, &mut temp);
            }
        }
        one_shot_systems.remove_indexes(indexes.into_iter());
        one_shot_systems.append(temp);
    }

    pub fn new_archetype(&mut self, arch: &Archetype, one_shot_systems: &mut OneShotSystems) {
        for system in self.systems.iter_mut() {
            system.new_archetype(arch);
        }
        one_shot_systems.new_archetype(arch);
    }

    pub fn run(&mut self, world: &mut World, one_shot_systems: &mut OneShotSystems) {
        for (sys, cond) in self.systems.iter_mut().zip(self.conditions.iter_mut()) {
            if let Some(cond) = cond {
                cond.run_unsafe(unsafe { world.as_unsafe_world() })
                    .then(|| sys.run_unsafe(unsafe { world.as_unsafe_world() }));
            } else {
                sys.run_unsafe(unsafe { world.as_unsafe_world() });
            }

            // TODO: either redo the render system so that is does not rely on commands or change
            // the deffered system. It could be more efficient to only apply deffered if necessary
            sys.apply_deffered(world, one_shot_systems);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, InternalComponent)]
    struct Health(u32);

    #[derive(Debug, InternalResource)]
    struct Weight(u32);

    #[derive(Debug, InternalComponent)]
    struct Size(u32);

    fn test_q(q: Query<(Health, Size)>) {
        if q.get_single().is_ok() {
            println!("match single");
        } else {
            println!("mismatch single");
        }
    }

    fn test_r(_r: Res<Weight>) {}

    #[test]
    fn miri() {
        let mut world = World::default();
        let mut scheduler = Scheduler::default();
        world.insert_resource(Weight(0));
        scheduler.add_systems(Schedule::Update, (test_q, test_r));
        scheduler.add_systems(Schedule::PostUpdate, test_r);
        scheduler.build_schedule(&mut world);
        scheduler.startup(&mut world);
        scheduler.run(&mut world);
        world.spawn((Health(0), Size(0)));
        scheduler.new_archetype(world.archetypes.get(ArchId::new(0)).unwrap());
        scheduler.run(&mut world);
        scheduler.flush_events(&mut world);
        scheduler.exit(&mut world);
    }
}
