pub extern crate ecs;

pub mod prelude;

pub mod gfx;
mod gl;
pub mod math;
pub mod platform;

use std::{ffi::OsString, path::PathBuf};

use ecs::{any::TypeGetter, Event, IntoSystemStorage, Resource, Schedule, Scheduler, World};
use logging::trace;
use prelude::KeyInput;

pub struct App {
    world: World,
    scheduler: Scheduler,
    target_fps: Option<f64>,
}

impl Default for App {
    fn default() -> Self {
        App {
            world: World::default(),
            scheduler: Scheduler::new(),
            target_fps: None,
        }
    }
}

impl App {
    // pub fn hot_reload_lib(&mut self, mut dir: PathBuf, lib: &str) -> &mut Self {
    //     dir.push("target");
    //     #[cfg(debug_assertions)]
    //     dir.push("debug");
    //     #[cfg(not(debug_assertions))]
    //     dir.push("release");

    //     dir.push(lib);
    //     let path_to_lib = dir.as_os_str();

    //     let lib_ext = dir.extension().expect("Specify path extension");
    //     let mut path = dir.clone();
    //     path.pop();
    //     path.push(format!("libtemp.{}", lib_ext.to_str().unwrap()));

    //     let path_to_write = path.as_os_str();
    //     #[cfg(debug_assertions)]
    //     trace!(
    //         "Path to lib: {}, Path to write: {}",
    //         path_to_lib.to_str().unwrap(),
    //         path_to_write.to_str().unwrap()
    //     );

    //     self.dyn_lib = Some(
    //         LinkedLib::new(path_to_lib.into(), path_to_write.into())
    //             .expect("Could not find library"),
    //     );

    //     self
    // }

    pub fn insert_resource<R: std::fmt::Debug + Resource + TypeGetter>(
        &mut self,
        resource: R,
    ) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    pub fn register_event<E: std::fmt::Debug + ecs::events::Event + TypeGetter>(
        &mut self,
    ) -> &mut Self {
        self.world.register_event::<E>();
        self
    }

    pub fn add_systems<M, B: IntoSystemStorage<M>>(
        &mut self,
        schedule: Schedule,
        systems: B,
    ) -> &mut Self {
        self.scheduler.add_systems(schedule, systems);
        self
    }

    pub fn target_fps(&mut self, target: f64) -> &mut Self {
        self.target_fps = Some(target);
        self
    }

    // TODO: make this sync
    pub async fn run(&mut self) {
        platform::game_loop(&mut self.scheduler, &mut self.world, None, self.target_fps).await;
    }
}
