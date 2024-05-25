use ecs::WinnyResource;

use super::*;

#[derive(Debug, WinnyResource, Clone, Copy)]
pub struct WindowPlugin {
    pub inner_size: (u32, u32),
    pub virtual_size: (u32, u32),
    pub position: (u32, u32),
    pub close_on_escape: bool,
}

impl Default for WindowPlugin {
    fn default() -> Self {
        Self {
            inner_size: (1920, 1080),
            virtual_size: (1920, 1080),
            position: (10, 10),
            close_on_escape: false,
        }
    }
}

impl Plugin for WindowPlugin {
    fn build(&self, world: &mut World, _scheduler: &mut Scheduler) {
        world.insert_resource(self.clone());
    }
}
