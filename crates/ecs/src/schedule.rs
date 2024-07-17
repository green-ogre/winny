use super::*;

#[derive(Debug)]
struct SchedulerBuilder {
    schedules: Vec<ScheduleBuilder>,
}

impl Default for SchedulerBuilder {
    fn default() -> Self {
        Self {
            schedules: vec![
                ScheduleBuilder::new(Schedule::StartUp),
                ScheduleBuilder::new(Schedule::Exit),
                ScheduleBuilder::new(Schedule::FlushEvents),
                ScheduleBuilder::new(Schedule::Platform),
                ScheduleBuilder::new(Schedule::PreUpdate),
                ScheduleBuilder::new(Schedule::Update),
                ScheduleBuilder::new(Schedule::PostUpdate),
            ],
        }
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

// TODO: fix system tree
#[derive(Debug, Default)]
pub struct Scheduler {
    builder: SchedulerBuilder,
    pub(crate) executers: Vec<ScheduleExecuter>,
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
            executer.new_archetype(archetype);
        }
    }

    pub fn run(&mut self, world: &mut World) {
        for executer in self.executers.iter_mut().skip(3) {
            let _span = util::tracing::trace_span!("schedule", name = ?executer.tag).entered();
            executer.run(world);
            executer.apply_deffered(world);
        }
    }

    pub fn startup(&mut self, world: &mut World) {
        let startup = &mut self.executers[Schedule::StartUp as usize];
        let _span = util::tracing::trace_span!("schedule", name = ?startup.tag).entered();
        startup.run(world);
        startup.apply_deffered(world);
    }

    pub fn flush_events(&mut self, world: &mut World) {
        let flush_events = &mut self.executers[Schedule::FlushEvents as usize];
        let _span = util::tracing::trace_span!("schedule", name = ?flush_events.tag).entered();
        flush_events.run(world);
        // NOTE: flush_events is a platform driven schedule that cannot be added to, meaning
        // that there is no need to apply deffered commands
    }

    pub fn exit(&mut self, world: &mut World) {
        let exit = &mut self.executers[Schedule::Exit as usize];
        let _span = util::tracing::trace_span!("schedule", name = ?exit.tag).entered();
        exit.run(world);
        exit.apply_deffered(world);
    }
}

// TODO: pull out backend schedules
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schedule {
    StartUp,
    Exit,
    FlushEvents,
    Platform,
    PreUpdate,
    Update,
    PostUpdate,
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

    pub fn apply_deffered(&mut self, world: &mut World) {
        for system in self.systems.iter_mut() {
            system.apply_deffered(world);
        }

        while self.archetypes_len < world.archetypes.len() {
            let arch = world
                .archetypes
                .get(ArchId::new(self.archetypes_len))
                .expect("valid id");
            self.new_archetype(arch);
            self.archetypes_len += 1;
        }
    }

    pub fn new_archetype(&mut self, arch: &Archetype) {
        for system in self.systems.iter_mut() {
            system.new_archetype(arch);
        }
    }

    pub fn run(&mut self, world: &mut World) {
        let world = unsafe { world.as_unsafe_world() };
        for (sys, cond) in self.systems.iter_mut().zip(self.conditions.iter_mut()) {
            if let Some(cond) = cond {
                cond.run_unsafe(world).then(|| sys.run_unsafe(world));
            } else {
                sys.run_unsafe(world);
            }
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
