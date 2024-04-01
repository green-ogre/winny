#![allow(dead_code)]

pub extern crate ecs_derive;
pub use ecs_derive::*;

pub mod any;
pub mod entity;
pub mod events;
pub mod query;
pub mod resources;
pub mod storage;
pub mod world;

pub use any::*;
pub use events::*;
pub use query::*;
pub use resources::*;
pub use storage::*;
pub use world::*;

pub type StartUpSystem<T> = fn(world: &mut World, args: &T);

pub fn default_start_up_system<T>(_world: &mut World, _args: &T) {}

#[cfg(test)]
mod winny {
    use super::*;

    #[derive(Event, Debug, TestTypeGetter)]
    struct TestEvent(usize);

    #[derive(Resource, TestTypeGetter, Debug, Default, PartialEq, Eq)]
    enum GameMode {
        Debug,
        Release,
        #[default]
        ReleaseNoMenu,
        Sandbox,
        StressTest,
    }

    #[derive(ComponentTest, Debug, TestTypeGetter, Clone)]
    struct Health(usize);

    #[derive(ComponentTest, TestTypeGetter, Debug, Clone)]
    struct Weight(usize);

    #[derive(ComponentTest, TestTypeGetter, Debug, Clone)]
    struct Size(usize);

    #[derive(BundleTest)]
    pub struct TestBundle {
        size: Size,
        weight: Weight,
        health: Health,
    }

    #[test]
    fn newest() {
        let mut world = World::default();

        let e1 = world.spawn(TestBundle {
            size: Size(1),
            weight: Weight(2),
            health: Health(3),
        });

        let e2 = world.spawn((Health(10),));
        let e3 = world.spawn((Health(15),));
        let e4 = world.spawn((Weight(35),));

        {
            let e1_h = Query::<Health, (With<Size>, With<Weight>)>::new(&world)
                .get(e1)
                .unwrap();
            assert_eq!(3, e1_h.0);
        }

        let mut commands = Commands::new();

        commands.get_entity(e2).despawn();
        commands.get_entity(e3).insert(Size(200)).insert(Size(4));
        commands.get_entity(e4).insert(Weight(4)).remove::<Health>();

        println!("{:#?}", world);

        commands.sync(&mut world);

        panic!("{:#?}", world);
    }

    // #[test]
    // fn entities() {
    //     let mut world = World::default();

    //     let _ = world.spawn((Health(10),));
    //     let _ = world.spawn((Health(15),));
    //     let _ = world.spawn((Weight(35),));
    //     let _ = world.spawn((Health(15), Weight(2)));
    // }

    // #[test]
    // fn queries() {
    //     let mut world = World::default();

    //     let _ = world.spawn((Health(10),));
    //     let e2 = world.spawn((Health(15), Size(20)));
    //     let _ = world.spawn((Weight(35), Size(20)));
    //     let _ = world.spawn((Health(15), Weight(2), Size(20)));

    //     for (mut health, mut weight) in Query::<(Health, Weight)>::new(&world).iter_mut() {
    //         assert_ne!(100, health.0);
    //         assert_ne!(100, weight.0);

    //         weight.0 = 100;
    //         health.0 = 100;
    //     }

    //     for (health, weight) in Query::<(Health, Weight)>::new(&world).iter() {
    //         assert_eq!(100, health.0);
    //         assert_eq!(100, weight.0);
    //     }

    //     let mut num_items = 0;
    //     for size in Query::<Size, Or<(With<Health>, With<Weight>)>>::new(&world).iter() {
    //         assert_eq!(20, size.0);
    //         num_items += 1;
    //     }
    //     assert_eq!(3, num_items);

    //     {
    //         let e2_h = Query::<Health, With<Size>>::new(&world).get(&e2);
    //         assert_eq!(15, e2_h.0);
    //     }

    //     {
    //         let mut e2_h = Query::<Health, With<Size>>::new(&world).get_mut(&e2);
    //         e2_h.0 = 2;
    //     }

    //     {
    //         let e2_h = Query::<Health, With<Size>>::new(&world).get(&e2);
    //         assert_eq!(2, e2_h.0);
    //     }

    //     {
    //         let e3_w = Query::<Weight, (With<Size>, Without<Health>)>::new(&world)
    //             .get_single()
    //             .unwrap();
    //         assert_eq!(35, e3_w.0);
    //     }

    //     {
    //         let (e3_w, e3_s) = Query::<(Weight, Size), Without<Health>>::new(&world)
    //             .get_single_mut()
    //             .unwrap();
    //         assert_eq!(35, e3_w.0);
    //         assert_eq!(20, e3_s.0);
    //     }
    // }

    // #[test]
    // fn events() {
    //     let mut world = World::default();
    //     world.register_event::<TestEvent>();

    //     {
    //         let mut ev_tests = EventWriter::<TestEvent>::new(&world);
    //         ev_tests.send(TestEvent(10));
    //         ev_tests.send(TestEvent(10));
    //         ev_tests.send(TestEvent(10));
    //     }

    //     {
    //         let mut reader = EventReader::<TestEvent>::new(&world).read();
    //         assert_eq!(3, reader.len());
    //         for ev in reader.drain(..) {
    //             println!("First Round: {:#?}", ev);
    //         }
    //     }

    //     {
    //         let mut reader = EventReader::<TestEvent>::new(&world).read();
    //         assert_eq!(0, reader.len());
    //         for ev in reader.drain(..) {
    //             println!("Second Round: {:#?}", ev);
    //         }
    //     }
    // }

    // #[test]
    // fn resources() {
    //     let mut world = World::default();

    //     let new_resource = GameMode::Debug;
    //     world.insert_resource(new_resource);

    //     {
    //         let game_mode = Res::<GameMode>::new(&world);
    //         assert_eq!(GameMode::Debug, *game_mode.as_ref());
    //     }

    //     {
    //         let mut game_mode = ResMut::<GameMode>::new(&world);
    //         *game_mode.as_mut() = GameMode::Release;
    //         assert_eq!(GameMode::Release, *game_mode.as_mut());
    //     }
    // }
}
