use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use crate::{entity::Entity, QueryData};

pub(crate) mod archetype;
pub mod bundle;
pub mod components;
pub(crate) mod dumb_vec;
pub(crate) mod resources;
pub(crate) mod sparse_set;
pub(crate) mod table;

pub use archetype::*;
pub use bundle::*;
pub use components::*;
pub use dumb_vec::*;
pub use resources::*;
pub use sparse_set::*;
pub use table::*;
