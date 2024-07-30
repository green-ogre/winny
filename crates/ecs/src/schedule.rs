use fxhash::FxHashMap;

use self::sets::{IntoSystemStorage, LabelId, SystemSet};

use super::*;

pub trait ScheduleLabel: LabelId {}

#[derive(Debug, Default)]
pub struct Scheduler {
    executers: FxHashMap<usize, ScheduleExecuter>,
    one_shot_systems: OneShotSystems,
}

impl Scheduler {
    pub fn add_systems<M, S: IntoSystemStorage<M>>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: S,
    ) {
        let systems = systems.into_set();
        self.executers
            .entry(schedule.id())
            .or_insert_with(|| ScheduleExecuter::default())
            .add_systems(systems);
    }

    pub fn init_schedule(&mut self, world: &mut World) {
        self.executers
            .values_mut()
            .for_each(|e| e.init_systems(world));
    }

    pub fn run_schedule(&mut self, world: &mut World, schedule: impl ScheduleLabel) {
        if let Some(executer) = &mut self.executers.get_mut(&schedule.id()) {
            let _span = util::tracing::trace_span!("schedule", name = ?schedule).entered();
            executer.run(world);
            executer.apply_deffered(world, &mut self.one_shot_systems);
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ScheduleExecuter {
    system_sets: Vec<SystemSet>,
}

impl ScheduleExecuter {
    pub fn new(system_sets: Vec<SystemSet>) -> Self {
        Self { system_sets }
    }

    pub fn add_systems(&mut self, system_set: SystemSet) {
        self.system_sets.push(system_set);
    }

    pub fn init_systems(&mut self, world: &mut World) {
        for set in self.system_sets.iter_mut() {
            set.validate_systems_and_conditions_or_panic(world);
            set.init_state(world);
        }
    }

    pub fn apply_deffered(&mut self, world: &mut World, one_shot_systems: &mut OneShotSystems) {
        for set in self.system_sets.iter_mut() {
            set.apply_deffered(world, one_shot_systems);
        }
    }

    // pub fn new_archetype(&mut self, arch: &Archetype, one_shot_systems: &mut OneShotSystems) {
    //     for set in self.system_sets.iter_mut() {
    //         set.new_archetype(arch, one_shot_systems);
    //     }
    // }

    pub fn run(&mut self, world: &mut World) {
        for set in self.system_sets.iter_mut() {
            set.run(world);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, InternalComponent)]
    struct Health(u32);

    #[derive(Debug, InternalResource)]
    struct Weight(u32);

    #[derive(Debug, InternalComponent)]
    struct Size(u32);

    fn test_q(_q: Query<(Health, Size)>) {
        println!("Hello, ");
    }

    fn startup(mut _commands: Commands) {
        println!("AHHHH");
    }

    fn test_r(_r: Res<Weight>) {
        println!("world!");
    }

    fn should_run(_r: Res<Weight>) -> bool {
        true
    }

    fn should_run_inner(_r: Res<Weight>) -> bool {
        false
    }

    fn test_run(_r: Res<Weight>) -> bool {
        true
    }

    #[test]
    fn run_if() {
        let mut world = World::default();
        world.insert_resource(Weight(0));
        let mut scheduler = Scheduler::default();
        scheduler.add_systems(TestLabel::Render, (startup,));
        scheduler.init_schedule(&mut world);
        scheduler.run_schedule(&mut world, TestLabel::Render);
        // println!("{scheduler:#?}");
    }

    #[derive(InternalScheduleLabel, Debug)]
    pub enum TestLabel {
        Render,
    }

    // #[test]
    // fn schedule_labels() {
    //     let render = Schedule::Render;
    //     let test_render = TestLabel::Render;
    //     // println!("{}, {}", render.id(), test_render.id());
    //     assert!(render.id() != test_render.id());
    // }
    //
    // #[test]
    // fn miri() {
    //     // let mut world = World::default();
    //     // let mut scheduler = Scheduler::default();
    //     // world.insert_resource(Weight(0));
    //     // scheduler.add_systems(Schedule::StartUp, startup);
    //     // scheduler.add_systems(Schedule::Update, (test_q, test_r));
    //     // scheduler.add_systems(Schedule::PostUpdate, test_r);
    //     // scheduler.init_systems(&mut world);
    //     // scheduler.build_schedule(&mut world);
    //     // println!("GGGGGG {:#?}", scheduler);
    //     // scheduler.startup(&mut world);
    //     // println!("SDFASDF {:#?}", scheduler);
    //     // scheduler.run(&mut world);
    //     // // scheduler.new_archetype(world.archetypes.get(ArchId::new(0)).unwrap());
    //     // scheduler.run(&mut world);
    //     // scheduler.flush_events(&mut world);
    //     // scheduler.exit(&mut world);
    // }
}
