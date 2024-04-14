#![allow(dead_code)]

pub extern crate ecs_derive;
pub use ecs_derive::*;

pub mod any;
pub mod events;
pub mod query;
pub mod storage;
pub mod systems;
pub mod threads;
pub mod world;

pub use any::*;
pub use events::*;
pub use query::*;
pub use storage::*;
pub use systems::*;
pub use threads::*;
pub use world::*;

#[cfg(test)]
mod winny {
    use core::panic;
    use std::time::SystemTime;

    use super::*;

    // #[derive(Event, Debug, TestTypeGetter)]
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

    // #[derive(BundleTest, Debug)]
    // pub struct TestBundle {
    //     size: Size,
    //     weight: Weight,
    //     health: Health,
    // }

    #[test]
    fn newest() {
        // let mut world = World::default();

        // let e1 = world.spawn(TestBundle {
        //     size: Size(1),
        //     weight: Weight(2),
        //     health: Health(3),
        // });

        // let e2 = world.spawn((Health(10),));
        // let e3 = world.spawn((Health(15),));
        // let e4 = world.spawn((Weight(35),));

        // {
        //     let e1_h = Query::<Health, (With<Size>, With<Weight>)>::new(&world)
        //         .get(e1)
        //         .unwrap();
        //     assert_eq!(3, e1_h.0);
        // }

        // let mut commands = Commands::new();

        // commands.get_entity(e1).despawn();
        // commands.get_entity(e2).insert(Size(200)).remove::<Health>();
        // commands.get_entity(e4).insert(Weight(4)).remove::<Health>();

        // println!("{:#?}", world);

        // commands.sync(&mut world);

        // println!("{:#?}", world);

        // let health_q = Query::<Health>::new(&world);
        // for health in health_q.iter() {
        //     println!("{:?}", health);
        // }

        // panic!();
    }

    #[test]
    fn commands_spawn() {
        // let mut world = World::default();
        // let mut commands = Commands::new();

        // commands.spawn(TestBundle {
        //     health: Health(10),
        //     size: Size(3),
        //     weight: Weight(45),
        // });

        // commands.spawn(TestBundle {
        //     health: Health(30),
        //     size: Size(300),
        //     weight: Weight(5),
        // });

        // commands.spawn((Health(1), Size(2), Weight(3)));

        // {
        //     let mut commands = Commands::new();
        //     commands.spawn((Health(20),));
        //     commands.sync(&mut world);
        // }

        // commands.sync(&mut world);

        // panic!("{:#?}", world);
    }

    fn crunch_lots_of_numbers() {
        let mut y: isize = 0;
        for x in 0..10_000_000 {
            y = (x as isize) % 10;
            y -= 4;
            y += 4 * (x as isize) % 3;
        }
    }

    fn hello_world(_commands: Commands) {
        // println!("Hello World!");
    }

    fn q1(query: Query<Weight>) {
        // println!("I am a query!");

        crunch_lots_of_numbers();
    }

    fn q2(query: Query<(Health, Weight)>, entity_q: Query<Entity>) {
        // println!("I am a query!");

        crunch_lots_of_numbers();
    }

    fn q3(query_single: Query<Health>) {
        // println!("I am a query!");

        crunch_lots_of_numbers();
    }

    #[derive(Debug, TestTypeGetter)]
    pub struct Info;
    impl Resource for Info {}

    fn i_am_a_resource(res: Res<Info>) {
        // println!("I am a resource!");
    }

    fn goodbye_world(_commands: Commands) {
        // println!("Goodbye World!");
    }

    #[test]
    fn systems() {
        let mut world = World::default();
        world.insert_resource(Info);
        let mut scheduler = Scheduler::new();
        scheduler.add_systems(Schedule::StartUp, hello_world);
        scheduler.add_systems(Schedule::Update, (q1, q2, q3));
        scheduler.add_systems(Schedule::Exit, goodbye_world);

        world.spawn((Health(10), Weight(10)));
        world.spawn((Health(2),));

        // println!("{:#?}", world);

        let start = SystemTime::now();
        scheduler.startup(&world);
        scheduler.run(&world);
        scheduler.exit(&world);
        let end = SystemTime::now();

        let duration = end.duration_since(start);

        if let Ok(duration) = duration {
            println!(
                "MULTITHREADED TIME: {:?}s, {:?}ms",
                duration.as_secs(),
                duration.as_millis(),
            );
        } else {
            println!("CLOCK ERROR");
        }

        let start = SystemTime::now();
        scheduler.startup_single_thread(&world);
        scheduler.run_single_thread(&world);
        scheduler.exit_single_thread(&world);
        let end = SystemTime::now();

        let duration = end.duration_since(start);

        if let Ok(duration) = duration {
            println!(
                "SINGLETHREADED TIME: {:?}s, {:?}ms",
                duration.as_secs(),
                duration.as_millis(),
            );
        } else {
            println!("CLOCK ERROR");
        }

        panic!();
    }
}
