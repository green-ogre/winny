use std::{marker::PhantomData, ops::Deref};

use chrono::{DateTime, Local, TimeDelta};
use ecs::{prelude::*, WinnyComponent, WinnyResource};

use crate::prelude::{App, Plugin};

pub struct TimePlugin;

impl Plugin for TimePlugin {
    fn build(&mut self, app: &mut crate::app::App) {
        app.register_resource::<DeltaTime>()
            .add_systems(Schedule::PreStartUp, insert_delta)
            .add_systems(Schedule::Platform, update_delta);
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
    fn register_timer<Emitter: TimeoutEmitter>(&mut self) -> &mut Self;
}

impl TimeApp for App {
    fn register_timer<Emitter: TimeoutEmitter>(&mut self) -> &mut App {
        self.register_event::<Emitter>()
            .add_systems(Schedule::Platform, emit_timer::<Emitter>);

        self
    }
}

#[derive(WinnyComponent)]
pub struct Timer<Emitter: TimeoutEmitter>(
    chrono::DateTime<chrono::Local>,
    i64,
    PhantomData<Emitter>,
);

impl<Emitter: TimeoutEmitter> Timer<Emitter> {
    pub fn new(duration: impl Into<TimerDurationSeconds>) -> Self {
        Self(
            chrono::Local::now(),
            (duration.into().0 * 1000.0) as i64,
            PhantomData,
        )
    }
}

pub fn emit_timer<Emitter: TimeoutEmitter>(
    mut commands: Commands,
    mut writer: EventWriter<Emitter>,
    timers: Query<(Entity, Timer<Emitter>)>,
) {
    for (entity, timer) in timers.iter() {
        if TimeDelta::milliseconds(timer.1) <= chrono::Local::now().signed_duration_since(timer.0) {
            writer.send(Emitter::default());
            commands.get_entity(entity).despawn();
        }
    }
}

pub trait TimeoutEmitter: Event + Default {}

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
