#![allow(dead_code)]

pub extern crate ecs_macro;
#[cfg(feature = "editor")]
pub extern crate egui;
pub use ecs_macro::*;

#[cfg(feature = "editor")]
pub mod egui_widget;
pub mod events;
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
