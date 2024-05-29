use ecs::{Scheduler, World};

pub fn game_loop(world: World, scheduler: Scheduler) {
    window::enter_winit_event_loop(world, scheduler);
}
