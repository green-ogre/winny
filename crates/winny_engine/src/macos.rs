use ecs::World;
use logging::error;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub fn macos_main(path_to_lib: String, world: &mut World) {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    event_loop.set_control_flow(ControlFlow::Poll);

    // ControlFlow::Wait pauses the event loop if no events are available to process.
    // This is ideal for non-game applications that only update in response to user
    // input, and uses significantly less power/CPU time than ControlFlow::Poll.
    event_loop.set_control_flow(ControlFlow::Wait);

    let _ = event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("The close button was pressed; stopping");
                elwt.exit();
            }
            Event::AboutToWait => {
                // Application update code.

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.
            }
            _ => (),
        }
    });
}

type LinkedFunc<'lib, I = (), O = ()> = libloading::Symbol<'lib, unsafe extern "C" fn(I) -> O>;

struct LinkedLib<'lib> {
    lib: libloading::Library,
    startup: Option<LinkedFunc<'lib, &'static mut World>>,
    update_render: Option<LinkedFunc<'lib, &'static mut World>>,
}

impl<'lib> LinkedLib<'lib> {
    pub fn new(path_to_lib: String) -> Result<Self, ()> {
        unsafe {
            let lib = libloading::Library::new(path_to_lib).map_err(|_| ())?;

            Ok(Self {
                lib,
                startup: None,
                update_render: None,
            })
        }
    }

    // Must be called after 'new' and before 'startup' and 'update'
    pub fn sync(&'lib mut self) -> Result<(), ()> {
        unsafe {
            self.startup = Some(self.lib.get(b"startup").map_err(|_| ())?);
            self.update_render = Some(self.lib.get(b"update_game_and_render").map_err(|_| ())?);

            Ok(())
        }
    }

    pub fn run_startup(&self, world: &'static mut World) -> Result<(), ()> {
        let Some(startup) = &self.startup else {
            error!("LinkedLib :: Failed to initialize 'startup'");
            return Err(());
        };

        unsafe { Ok(startup(world)) }
    }

    pub fn run_update(&self, world: &'static mut World) -> Result<(), ()> {
        let Some(update) = &self.update_render else {
            error!("LinkedLib :: Failed to initialize 'startup'");
            return Err(());
        };

        unsafe { Ok(update(world)) }
    }
}
