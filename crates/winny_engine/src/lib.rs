pub extern crate ecs;

pub mod prelude;

pub mod gfx;
mod gl;
pub mod input;
pub mod logging;
#[cfg(windows)]
mod win32;

use ecs::{any::TypeGetter, Event, Resource, StartUpSystem, World};

pub struct App<T: std::default::Default> {
    reload_path: String,
    world: World,
    start_up: StartUpSystem<T>,
    log: bool,
    args: T,
}

impl<T: Default> Default for App<T> {
    fn default() -> Self {
        App {
            reload_path: String::new(),
            world: World::default(),
            start_up: ecs::default_start_up_system,
            log: false,
            args: T::default(),
        }
    }
}

impl<T: std::default::Default> App<T> {
    pub fn path_to_dll(&mut self, path: String) -> &mut Self {
        self.reload_path = path;
        self
    }

    pub fn insert_resource<R: std::fmt::Debug + Resource + TypeGetter>(
        &mut self,
        resource: R,
    ) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    pub fn register_event<E: std::fmt::Debug + Event + TypeGetter>(&mut self) -> &mut Self {
        self.world.register_event::<E>();
        self
    }

    pub fn register_start_up_system(&mut self, start_up: StartUpSystem<T>) -> &mut Self {
        self.start_up = start_up;
        self
    }

    pub fn arguments(&mut self, args: T) -> &mut Self {
        self.args = args;
        self
    }

    pub fn log(&mut self) -> &mut Self {
        self.log = true;
        self
    }

    pub fn run(&mut self) {
        (self.start_up)(&mut self.world, &self.args);
        enter_platform(self.log, self.reload_path.clone(), &mut self.world);
    }
}

#[cfg(windows)]
fn enter_platform(log: bool, reload_path: String, world: &mut World) {
    win32::win32_main(log, reload_path, world);
}
