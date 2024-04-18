pub struct WindowPlugin {
    dimensions: [usize; 2],
    title: String,
}

#[derive(Debug, Resource, TypeGetter)]
pub struct Window {
    window: winit::window::Window,
}

impl Plugin for WindowPlugin {
    fn build(&self, world: &mut World, scheduler: &mut Scheduler) {}
}
