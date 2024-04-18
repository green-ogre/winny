pub mod prelude;

mod camera;
mod gfx;
mod gl;
mod math;
mod platform;
mod plugins;

use ::plugins::{Plugin, PluginSet};
use ecs::{any::TypeGetter, IntoSystemStorage, Resource, Schedule, Scheduler, World};

pub struct App {
    world: World,
    scheduler: Scheduler,
    plugins: Vec<Box<dyn Plugin>>,
}

impl Default for App {
    fn default() -> Self {
        App {
            world: World::default(),
            scheduler: Scheduler::new(),
            plugins: vec![],
        }
    }
}

impl App {
    pub(crate) fn add_plugin_boxed(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    pub fn add_plugins<T: PluginSet>(mut self, plugins: T) -> Self {
        for p in plugins.get().into_iter() {
            self.add_plugin_boxed(p);
        }

        self
    }

    pub fn insert_resource<R: std::fmt::Debug + Resource + TypeGetter>(
        mut self,
        resource: R,
    ) -> Self {
        self.world.insert_resource(resource);
        self
    }

    pub fn register_event<E: std::fmt::Debug + ecs::events::Event + TypeGetter>(mut self) -> Self {
        self.world.register_event::<E>();
        self
    }

    pub fn add_systems<M, B: IntoSystemStorage<M>>(
        mut self,
        schedule: Schedule,
        systems: B,
    ) -> Self {
        self.scheduler.add_systems(schedule, systems);
        self
    }

    pub fn run(mut self) {
        logger::init();

        for plugin in self.plugins.iter() {
            plugin.build(&mut self.world, &mut self.scheduler);
        }

        pollster::block_on(platform::game_loop(self.world, self.scheduler));
    }
}
