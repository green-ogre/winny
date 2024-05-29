use std::{
    collections::VecDeque,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ecs::{prelude::*, Scheduler, TypeGetter, WinnyEvent, WinnyResource, World};
use logger::{error, info};

use crate::plugins::{Plugin, PluginSet};

#[derive(Debug, WinnyEvent)]
pub struct AppExit;

pub struct App {
    world: Option<World>,
    scheduler: Option<Scheduler>,
    plugins: Option<Vec<Box<dyn Plugin>>>,
    window_event_loop: Option<Box<dyn FnOnce()>>,
}

impl Default for App {
    fn default() -> Self {
        App {
            world: Some(World::default()),
            scheduler: Some(Scheduler::new()),
            plugins: Some(vec![]),
            window_event_loop: None,
        }
    }
}

impl App {
    pub(crate) fn add_plugin_boxed(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.as_mut().unwrap().push(plugin);
    }

    pub fn add_plugins<T: PluginSet>(&mut self, plugins: T) -> &mut Self {
        for p in plugins.get().into_iter() {
            self.add_plugin_boxed(p);
        }

        self
    }

    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.as_mut().unwrap().insert_resource(resource);
        self
    }

    pub fn register_event<E: Event>(&mut self) -> &mut Self {
        self.world.as_mut().unwrap().register_event::<E>();
        self
    }

    pub fn add_systems<M, B: IntoSystemStorage<M>>(
        &mut self,
        schedule: Schedule,
        systems: B,
    ) -> &mut Self {
        self.scheduler
            .as_mut()
            .unwrap()
            .add_systems(schedule, systems);
        self
    }

    pub fn set_window_callback(&mut self, callback: Box<dyn FnOnce()>) -> &mut Self {
        self.window_event_loop = Some(callback);
        self
    }

    pub fn run(&mut self) {
        logger::init();
        set_panic_hook();
        self.world.as_mut().unwrap().register_event::<AppExit>();
        self.world.as_mut().unwrap().insert_resource(DeltaT(0.0));

        loop {
            let mut plugins = self.plugins.replace(vec![]).unwrap();
            for plugin in plugins.iter_mut() {
                plugin.build(self);
            }
            if plugins.is_empty() {
                break;
            }
        }

        let mut world = self.world.take().unwrap();
        let mut scheduler = self.scheduler.take().unwrap();

        let h = std::thread::spawn(move || {
            scheduler.startup(&world);

            let mut start = SystemTime::now();
            let mut end = SystemTime::now();
            loop {
                let dt = DeltaT(end.duration_since(start).unwrap_or_default().as_secs_f64());
                start = SystemTime::now();
                update_ecs(dt, &mut world, &mut scheduler);
                end = SystemTime::now();
            }
        });

        if let Some(window_event_loop) = self.window_event_loop.take() {
            window_event_loop();
        } else {
            let _ = h.join();
        }
    }
}

// TODO: better panics => this is useful for exiting if non main scope panics
fn set_panic_hook() {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let line = line!();
        let column = column!();
        let file = file!();
        error!("[{}:{}:{}] Panic => Exiting...", file, line, column);
        orig_hook(panic_info);
        std::process::exit(1);
    }));
}

#[derive(Debug, WinnyResource)]
pub struct DeltaT(pub f64);

fn update_ecs(delta_t: DeltaT, world: &mut World, scheduler: &mut Scheduler) {
    update_delta_t(world, delta_t);

    if world.resource_ids.contains_key(&PerfCounter::type_id()) {
        // TODO: fix me
        let mut perf = world.resource_mut::<PerfCounter>().clone();
        perf.start();
        run_schedule_and_log(scheduler, &mut perf, world, Schedule::Platform);
        run_schedule_and_log(scheduler, &mut perf, world, Schedule::PreUpdate);
        run_schedule_and_log(scheduler, &mut perf, world, Schedule::Update);
        run_schedule_and_log(scheduler, &mut perf, world, Schedule::PostUpdate);
        run_schedule_and_log(scheduler, &mut perf, world, Schedule::Render);
        check_for_exit(world, scheduler);
        world.flush_events();
        perf.stop();
        *world.resource_mut::<PerfCounter>() = perf;
    } else {
        scheduler.run(world);
        check_for_exit(world, scheduler);
        world.flush_events();
    }
}

fn run_schedule_and_log(
    scheduler: &mut Scheduler,
    perf: &mut PerfCounter,
    world: &mut World,
    schedule: Schedule,
) {
    perf.start_debug_event();
    scheduler.run_schedule(schedule, world);
    perf.stop_debug_event();
    perf.log_last_debug_event(format!("ECS: {:?}", schedule).as_str());
}

fn update_delta_t(world: &mut World, delta_t: DeltaT) {
    let mut dt = world.resource_mut::<DeltaT>();
    *dt = delta_t;
}

fn check_for_exit(world: &mut World, scheduler: &mut Scheduler) {
    if world
        .events
        .get_mut(world.get_event_id::<AppExit>())
        .is_some_and(|exit| exit.read::<AppExit>().next().is_some())
    {
        scheduler.run_schedule(ecs::Schedule::Exit, world);
    }
}

#[derive(Debug, WinnyResource, Clone)]
struct PerfCounter {
    begin: Option<SystemTime>,
    begin_debug_event: Option<SystemTime>,
    end: Option<SystemTime>,
    end_debug_event: Option<SystemTime>,
    last_fram_duration: Option<Duration>,
    frames: usize,
    total_frames: usize,
    lost_frames: usize,
    lost_frames_sum: usize,
    highest_lost_frames: usize,
    frames_sum: f64,
    iterations: usize,
    // target_frame_len: Option<f64>,
    duration: f64,
    start_of_second: Duration,
    debug_events: VecDeque<(String, Duration)>,
}

#[allow(unused)]
impl PerfCounter {
    pub fn new(target_fps: Option<f64>) -> Self {
        let _target_frame_len = target_fps.map(|target| 1.0 / target);

        Self {
            begin: None,
            begin_debug_event: None,
            end: None,
            end_debug_event: None,
            last_fram_duration: None,
            frames: 0,
            total_frames: 0,
            lost_frames: 0,
            lost_frames_sum: 0,
            highest_lost_frames: 0,
            frames_sum: 0.0,
            iterations: 0,
            // target_frame_len,
            duration: 0.0,
            start_of_second: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is a construct"),
            debug_events: VecDeque::new(),
        }
    }

    pub fn last_frame_duration(&self) -> Option<Duration> {
        self.last_fram_duration
    }

    pub fn start(&mut self) {
        self.begin = Some(SystemTime::now());
    }

    pub fn start_debug_event(&mut self) {
        self.begin_debug_event = Some(SystemTime::now());
    }

    pub fn current_frame_len(&self) -> Result<Duration, std::time::SystemTimeError> {
        Ok(SystemTime::now().duration_since(self.begin.unwrap())?)
    }

    pub fn _should_advance(&self) -> bool {
        // self.target_frame_len.is_none()
        //     || self
        //         .current_frame_len()
        //         .map(|dur| dur.as_secs_f64())
        //         .unwrap_or_default()
        //         >= self.target_frame_len.unwrap()
        true
    }

    pub fn stop(&mut self) {
        self.end = Some(SystemTime::now());

        // info!(
        //     "> Measured Frame Length: {},\tTarget Frame Length: {},\tLoss: {}",
        //     self.current_frame_len().unwrap_or_default().as_secs_f64(),
        //     self.target_frame_len.unwrap_or_default(),
        //     (self.current_frame_len().unwrap_or_default().as_secs_f64()
        //         - self.target_frame_len.unwrap_or_default())
        //     .abs()
        // );
        self.frames_sum += self.current_frame_len().unwrap_or_default().as_secs_f64();

        self.frames += 1;

        self.last_fram_duration = Some(self.current_frame_len().unwrap_or_default());

        self.duration = self
            .end
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .expect("time is a construct")
            .as_secs_f64()
            - self.start_of_second.as_secs_f64();

        if self.duration >= 1.0 {
            for (label, duration) in self.debug_events.drain(..) {
                info!("{} => {}", label, duration.as_secs_f32());
            }

            self.start_of_second = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is a construct");
            self.total_frames += self.frames;

            info!(
                "# Frames {},\tDuration: {},\tExpected {} Frames: {},\tLost Frames: {}",
                self.frames, self.duration, self.frames, self.frames_sum, self.lost_frames
            );

            if self.lost_frames > self.highest_lost_frames {
                self.highest_lost_frames = self.lost_frames;
            }
            self.frames = 0;
            self.lost_frames = 0;
            self.frames_sum = 0.0;
            self.iterations += 1;
        }

        self.debug_events.drain(..);
    }

    pub fn stop_debug_event(&mut self) {
        self.end_debug_event = Some(SystemTime::now());
    }

    pub fn log_last_debug_event(&mut self, label: &str) {
        if let Some(duration) = self.query_last_debug_event() {
            self.debug_events.push_back((label.into(), duration));
        }
    }

    pub fn query_last_debug_event(&self) -> Option<Duration> {
        if let Some(start) = self.begin_debug_event {
            if let Some(end) = self.end_debug_event {
                let dur = end.duration_since(start);
                if dur.is_ok() {
                    return Some(dur.unwrap());
                } else {
                    return None;
                }
            }
        }

        None
    }

    pub fn exit_stats(&self) {
        info!(
            ">> Iterations: {},\tFPS: {},\tTotal Lost Frames: {},\tAverage: {},\tHigh:{}",
            self.iterations,
            self.total_frames / self.iterations.max(1),
            self.lost_frames_sum,
            self.lost_frames_sum / self.iterations.max(1),
            self.highest_lost_frames
        );
    }
}

pub struct PerfPlugin;

impl Plugin for PerfPlugin {
    fn build(&mut self, app: &mut crate::app::App) {
        app.insert_resource(PerfCounter::new(None))
            .add_systems(Schedule::Exit, (exit_stats,));
    }
}

fn exit_stats(perf: Res<PerfCounter>) {
    perf.exit_stats();
}
