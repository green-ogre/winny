use super::*;

#[derive(Debug)]
struct SchedulerBuilder {
    pub startup: Option<ScheduleBuilder>,
    pub platform: Option<ScheduleBuilder>,
    pub pre_update: Option<ScheduleBuilder>,
    pub update: Option<ScheduleBuilder>,
    pub post_update: Option<ScheduleBuilder>,
    pub render: Option<ScheduleBuilder>,
    pub flush_events: Option<ScheduleBuilder>,
    pub exit: Option<ScheduleBuilder>,
}

impl SchedulerBuilder {
    pub fn new() -> Self {
        Self {
            startup: Some(ScheduleBuilder::new()),
            platform: Some(ScheduleBuilder::new()),
            pre_update: Some(ScheduleBuilder::new()),
            update: Some(ScheduleBuilder::new()),
            post_update: Some(ScheduleBuilder::new()),
            render: Some(ScheduleBuilder::new()),
            flush_events: Some(ScheduleBuilder::new()),
            exit: Some(ScheduleBuilder::new()),
        }
    }
}

// TODO: fix system tree
#[derive(Debug)]
pub struct Scheduler {
    builder: SchedulerBuilder,

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
            builder: SchedulerBuilder::new(),
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
        let builder = match schedule {
            Schedule::StartUp => &mut self.builder.startup,
            Schedule::Platform => &mut self.builder.platform,
            Schedule::PreUpdate => &mut self.builder.pre_update,
            Schedule::Update => &mut self.builder.update,
            Schedule::PostUpdate => &mut self.builder.post_update,
            Schedule::Render => &mut self.builder.render,
            Schedule::FlushEvents => &mut self.builder.flush_events,
            Schedule::Exit => &mut self.builder.exit,
        };
        let builder = builder.as_mut().unwrap();

        let set = systems.into_set();
        builder.push_set(set);
    }

    pub fn build_schedule(&mut self) {
        self.startup = self.builder.startup.take().unwrap().build_schedule();
        self.platform = self.builder.platform.take().unwrap().build_schedule();
        self.pre_update = self.builder.pre_update.take().unwrap().build_schedule();
        self.update = self.builder.update.take().unwrap().build_schedule();
        self.post_update = self.builder.post_update.take().unwrap().build_schedule();
        self.render = self.builder.render.take().unwrap().build_schedule();
        self.flush_events = self.builder.flush_events.take().unwrap().build_schedule();
        self.exit = self.builder.exit.take().unwrap().build_schedule();
    }

    pub fn init_systems(&mut self, world: &World) {
        let init = |storage: &mut Vec<SystemSet>| {
            for set in storage.iter_mut() {
                set.init(unsafe { world.as_unsafe_world() });
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
            unsafe { set.run(world.as_unsafe_world()) };
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
