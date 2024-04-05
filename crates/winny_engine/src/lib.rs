pub extern crate ecs;

use logging::*;

pub mod prelude;

pub mod gfx;
mod gl;
pub mod input;

#[cfg(target_os = "windows")]
mod win32;

#[cfg(target_os = "macos")]
mod macos;

use ecs::{any::TypeGetter, Event, Resource, StartUpSystem, World};

pub struct App<T: std::default::Default> {
    reload_path: String,
    world: World,
    start_up: StartUpSystem<T>,
    args: T,
}

impl<T: Default> Default for App<T> {
    fn default() -> Self {
        App {
            reload_path: String::new(),
            world: World::default(),
            start_up: ecs::default_start_up_system,
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

    pub fn run(&mut self) {
        trace!("Entering Startup");
        (self.start_up)(&mut self.world, &self.args);
        trace!("Exiting Startup");
        // enter_platform(self.reload_path.clone(), &mut self.world);
    }
}

#[cfg(target_os = "windows")]
pub fn enter_platform(reload_path: String, world: &mut World) {
    trace!("Entering Windows Main");
    win32::win32_main(reload_path, world);
}

#[cfg(target_os = "macos")]
pub fn enter_platform(lib_path: String) {
    trace!("Entering MacOS Main");

    let mut world = World::default();
    macos::macos_main(lib_path, &mut world);
}
