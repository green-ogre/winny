use core::panic;

use super::*;

#[derive(Debug)]
pub struct Scheduler {
    startup: Vec<SystemSet>,
    platform: Vec<SystemSet>,
    pre_update: Vec<SystemSet>,
    update: Vec<SystemSet>,
    post_update: Vec<SystemSet>,
    render: Vec<SystemSet>,
    flush_events: Vec<SystemSet>,
    exit: Vec<SystemSet>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            startup: Vec::new(),
            platform: Vec::new(),
            pre_update: Vec::new(),
            update: Vec::new(),
            post_update: Vec::new(),
            render: Vec::new(),
            flush_events: Vec::new(),
            exit: Vec::new(),
        }
    }

    pub fn add_systems<M, S: IntoSystemStorage<M>>(&mut self, schedule: Schedule, systems: S) {
        let storage = match schedule {
            Schedule::StartUp => &mut self.startup,
            Schedule::Platform => &mut self.platform,
            Schedule::PreUpdate => &mut self.pre_update,
            Schedule::Update => &mut self.update,
            Schedule::PostUpdate => &mut self.post_update,
            Schedule::Render => &mut self.render,
            Schedule::FlushEvents => &mut self.flush_events,
            Schedule::Exit => &mut self.exit,
        };

        let mut set = systems.into_set();
        set.validate_or_panic();
        set.condense();

        storage.push(set);
    }

    pub fn optimize_schedule(&mut self) {
        let optimize = |storage: &mut Vec<SystemSet>| {
            let optimized_set = SystemSet::join_disjoint(storage.drain(..).collect());
            storage.push(optimized_set)
        };
        self.apply_to_all_schedules(optimize)
    }

    pub fn init_systems(&mut self, world: &World) {
        let init = |storage: &mut Vec<SystemSet>| {
            for set in storage.iter_mut() {
                set.init_systems(unsafe { world.as_unsafe_world() });
            }
        };
        self.apply_to_all_schedules(init)
    }

    fn apply_to_all_schedules(&mut self, mut f: impl FnMut(&mut Vec<SystemSet>)) {
        let mut apply = |storage: &mut Vec<SystemSet>| f(storage);

        apply(&mut self.startup);
        apply(&mut self.platform);
        apply(&mut self.pre_update);
        apply(&mut self.update);
        apply(&mut self.post_update);
        apply(&mut self.render);
        apply(&mut self.flush_events);
        apply(&mut self.exit);
    }

    pub fn run_schedule(&mut self, schedule: Schedule, world: &World) {
        let schedule = match schedule {
            Schedule::StartUp => &mut self.startup,
            Schedule::Platform => &mut self.platform,
            Schedule::PreUpdate => &mut self.pre_update,
            Schedule::Update => &mut self.update,
            Schedule::PostUpdate => &mut self.post_update,
            Schedule::Render => &mut self.render,
            Schedule::FlushEvents => &mut self.flush_events,
            Schedule::Exit => &mut self.exit,
        };

        for set in schedule.iter_mut() {
            unsafe { set.run_tree(world.as_unsafe_world()) };
        }
    }

    pub fn startup(&mut self, world: &World) {
        self.run_schedule(Schedule::StartUp, &world);
    }

    pub fn run(&mut self, world: &World) {
        self.run_schedule(Schedule::Platform, world);
        self.run_schedule(Schedule::PreUpdate, world);
        self.run_schedule(Schedule::Update, world);
        self.run_schedule(Schedule::PostUpdate, world);
        self.run_schedule(Schedule::Render, world);
        self.run_schedule(Schedule::FlushEvents, world);
    }

    pub fn exit(&mut self, world: &World) {
        self.run_schedule(Schedule::Exit, world);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Schedule {
    StartUp,
    Platform,
    PreUpdate,
    Update,
    PostUpdate,
    Render,
    FlushEvents,
    Exit,
}
