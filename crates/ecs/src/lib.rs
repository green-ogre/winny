#![allow(dead_code)]

pub extern crate ecs_macro;
pub use ecs_macro::*;

pub mod events;
pub mod prelude;
pub mod query;
pub mod schedule;
pub mod storage;
pub mod system_param;
pub mod systems;
pub mod threads;
pub mod world;

pub use events::*;
pub use query::*;
pub use schedule::*;
pub use storage::*;
pub use system_param::*;
pub use systems::*;
pub use world::*;

#[cfg(test)]
mod test {
    use crate::dumb_vec::*;

    #[test]
    fn dumb_vec() {
        let layout = std::alloc::Layout::new::<String>();
        let capacity = 4;
        let drop = new_dumb_drop::<String>();
        let mut v = DumbVec::new(layout, capacity, drop);
        v.push::<String>("Hello".into());
        v.push::<String>(",".into());
        v.push::<String>("World".into());
        v.push::<String>("!".into());

        let mut n = v.to_new_with_capacity(2);

        println!("{:#?}", v.as_slice::<String>());
        println!("{:#?}", n.as_slice::<String>());

        v.remove_and_push_other(&mut n, 1);

        println!("{:#?}", v.as_slice::<String>());
        println!("{:#?}", n.as_slice::<String>());

        println!("{v:#?}");
        println!("{n:#?}");
    }
}
