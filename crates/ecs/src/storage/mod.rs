use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use crate::{
    any::{self, *},
    entity::Entity,
    QueryData,
};

pub mod archetype;
pub mod bundle;
pub mod components;
pub mod dumb_vec;
pub mod resources;
pub mod sparse_set;
pub mod table;

pub use archetype::*;
pub use bundle::*;
pub use components::*;
pub use dumb_vec::*;
pub use resources::*;
pub use sparse_set::*;
pub use table::*;
