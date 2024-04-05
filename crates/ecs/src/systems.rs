use crate::{Commands, Query, World};

// pub trait SystemParam {
//     type Param: SystemParam;
//
//     fn to_param(&self, world: &World) -> Self::Param;
// }
//
// pub trait SystemFunc {
//     fn run(&self, world: &World);
// }
//
// impl SystemParam for Commands {
//     type Param = Commands;
//
//     fn to_param(&self, world: &World) -> Self::Param {
//         Commands::new()
//     }
// }
//
// impl<'a, T, F> SystemParam for Query<'a, T, F> {
//     type Param = Query<'a, T, F>;
//
//     fn to_param(&self, world: &World) -> Self::Param {
//         Query::new(&world)
//     }
// }
//
// // impl<F: 'static, P: SystemParam> SystemFunc for F
// // where
// //     F: FnMut(P),
// // {
// //     fn run(&self, world: &World) {
// //         self(<P as SystemParam>::Param)
// //     }
// // }
//
// fn test_system_c(commands: &mut Commands) {}
// fn test_system_q(query: Query<()>) {}
//
// fn test() {
//     let world = World::default();
//
//     (test_system_c)(&mut Commands::new());
//     (test_system_q)(Query::new(&world));
// }
