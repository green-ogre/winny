use chrono::{DateTime, Local, TimeDelta};
use ecs::{prelude::*, WinnyComponent, WinnyResource};

use crate::{
    app::AppSchedule,
    prelude::{App, Plugin},
};

pub struct TimePlugin;

impl Plugin for TimePlugin {
    fn build(&mut self, app: &mut crate::app::App) {
        app.register_resource::<DeltaTime>()
            .add_systems(AppSchedule::PreStartUp, insert_delta)
            .add_systems(AppSchedule::Platform, update_delta);
    }
}

fn insert_delta(mut commands: Commands) {
    commands.insert_resource(DeltaTime::new());
}

fn update_delta(mut delta: ResMut<DeltaTime>) {
    delta.update();
}

#[derive(WinnyResource)]
pub struct DeltaTime {
    elapsed: f32,
    last_time: DateTime<Local>,
    pub delta: f32,
}

impl DeltaTime {
    pub fn new() -> Self {
        Self {
            elapsed: 0.0,
            last_time: chrono::Local::now(),
            delta: 0.0,
        }
    }

    pub fn update(&mut self) {
        let delta = chrono::Local::now().signed_duration_since(self.last_time);
        self.last_time = chrono::Local::now();
        self.delta = delta.num_milliseconds() as f32 * 1e-3;
        self.elapsed += self.delta;
    }

    pub fn wrapping_elapsed_as_seconds(&self) -> f32 {
        self.elapsed
    }
}

pub trait TimeApp {
    fn register_timer<E: Event>(&mut self) -> &mut Self;
}

impl TimeApp for App {
    fn register_timer<E: Event>(&mut self) -> &mut App {
        self.register_event::<E>()
            .add_systems(AppSchedule::Platform, emit_timer::<E>);

        self
    }
}

#[derive(WinnyComponent)]
pub struct Timer<E: Event>(chrono::DateTime<chrono::Local>, i64, Option<E>);

impl<E: Event> Timer<E> {
    pub fn new(duration: impl Into<TimerDurationSeconds>, event: E) -> Self {
        Self(
            chrono::Local::now(),
            (duration.into().0 * 1000.0) as i64,
            Some(event),
        )
    }
}

pub fn emit_timer<E: Event>(
    mut commands: Commands,
    mut writer: EventWriter<E>,
    mut timers: Query<(Entity, Mut<Timer<E>>)>,
) {
    for (entity, timer) in timers.iter_mut() {
        if TimeDelta::milliseconds(timer.1) <= chrono::Local::now().signed_duration_since(timer.0) {
            if let Some(event) = timer.2.take() {
                writer.send(event);
            }
            commands.get_entity(entity).despawn();
        }
    }
}

pub struct TimerDurationSeconds(f32);

macro_rules! impl_timer_duration {
    ($t:ident) => {
        impl From<$t> for TimerDurationSeconds {
            fn from(value: $t) -> Self {
                Self(value as f32)
            }
        }
    };
}

impl_timer_duration!(f32);
impl_timer_duration!(u32);
impl_timer_duration!(usize);
