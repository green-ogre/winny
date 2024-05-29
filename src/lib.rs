#![allow(unused)]

pub extern crate app;
pub extern crate ecs;
pub extern crate logger;
pub extern crate winny_math as math;

#[cfg(feature = "window")]
pub extern crate window;

#[cfg(feature = "window")]
pub extern crate gfx;

#[cfg(feature = "audio")]
pub extern crate audio;

pub mod prelude {
    #[cfg(feature = "hot_reload")]
    pub use hot_reload::prelude::*;

    #[cfg(feature = "window")]
    pub use window::prelude::*;

    #[cfg(feature = "window")]
    pub use gfx::prelude::*;

    #[cfg(feature = "audio")]
    pub use audio::prelude::*;

    pub use app::prelude::*;
    pub use ecs::prelude::*;
    pub use logger::*;
    pub use winny_math as math;
    pub use winny_math::prelude::*;
}
