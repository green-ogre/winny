use winny::prelude::*;

pub fn main() {
    App::default()
        .add_plugins(DefaultPlugins {
            window: WindowPlugin {
                title: "time_example",
                close_on_escape: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .register_timer::<InfoTimeout>()
        .add_systems(Schedule::StartUp, spawn_timer)
        .add_systems(Schedule::Update, print_on_timeout)
        .run();
}

#[derive(Event, Default)]
struct InfoTimeout;

fn spawn_timer(mut commands: Commands) {
    commands.spawn(Timer::new(1.0, InfoTimeout));
}

fn print_on_timeout(timeouts: EventReader<InfoTimeout>, mut commands: Commands) {
    for _ in timeouts.read() {
        info!("Timeout!");
        commands.spawn(Timer::new(1.0, InfoTimeout));
    }
}
