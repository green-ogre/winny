#![allow(dead_code)]

pub extern crate ecs_macro;
pub use ecs_macro::*;

pub mod events;
pub mod prelude;
pub mod query;
pub mod schedule;
pub mod storage;
pub mod systems;
pub mod threads;
pub mod world;

pub use events::*;
pub use query::*;
pub use schedule::*;
pub use storage::*;
pub use systems::*;
pub use world::*;

pub extern crate any_vec;
