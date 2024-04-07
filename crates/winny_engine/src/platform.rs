use core::time;
use std::{
    env,
    error::Error,
    ffi::OsString,
    io::Read,
    marker::PhantomData,
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ecs::{Scheduler, World};
use logging::{error, info, trace, LogLevel};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub fn main_loop(
    scheduler: &mut Scheduler,
    world: &mut World,
    target_fps: Option<f64>,
    log_perf: bool,
) {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    // let mut app = LinkedLib::new(path_to_lib, path_to_write).unwrap();
    // app.run_startup(world);
    scheduler.startup(world);

    let target_frame_len = target_fps.map(|target| Some(1.0 / target)).unwrap_or(None);
    let mut perf = PerfCounter::new(
        target_frame_len,
        match log_perf {
            false => LogLevel::Info,
            true => LogLevel::Trace,
        },
    );

    let _ = event_loop.run(move |event, elwt| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            scheduler.exit(world);

            perf.exit_stats();
            elwt.exit();
        }
        Event::AboutToWait => {
            perf.start();

            // app.run_update(world);
            // app.refresh_if_modified();
            scheduler.run(world);

            while !perf.should_advance() {}
            perf.stop();
        }
        Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    device_id,
                    event,
                    is_synthetic,
                },
            ..
        } => {}
        _ => (),
    });
}

fn read_file(path: &String) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut f = std::fs::File::open(path)?;
    let metadata = f.metadata()?;
    let mut buf = vec![0u8; metadata.len() as usize];
    f.read(&mut buf)?;

    Ok(buf)
}

pub struct InputBuffer {
    buf: [Option<KeyInput>; 10],
    index: u8,
    len: u8,
}

impl Default for InputBuffer {
    fn default() -> Self {
        InputBuffer {
            buf: [None; 10],
            index: 0,
            len: 10,
        }
    }
}

impl InputBuffer {
    pub fn push(&mut self, e: KeyInput) {
        if self.index < self.len - 1 {
            self.buf[(self.index + 1) as usize] = Some(e);
            self.index += 1;
        } else {
            panic!("Need a bigger input buffer");
        }
    }

    pub fn pop(&mut self) -> Option<KeyInput> {
        let val = std::mem::replace(&mut self.buf[self.index as usize], None);
        self.index = self.index.saturating_sub(1);

        val
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum KeyCode {
    W,
    S,
    A,
    D,
    H,
    J,
    K,
    L,
    E,
    I,
    Key1,
    Key2,
    Escape,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyInput {
    pub vk: KeyCode,
    pub state: KeyState,
}

pub struct PerfCounter {
    begin: Option<SystemTime>,
    end: Option<SystemTime>,
    log_level: LogLevel,
    frames: usize,
    total_frames: usize,
    lost_frames: usize,
    lost_frames_sum: usize,
    highest_lost_frames: usize,
    frames_sum: f64,
    iterations: usize,
    target_frame_len: Option<f64>,
    duration: f64,
    start_of_second: Duration,
}

impl PerfCounter {
    pub fn new(target_frame_len: Option<f64>, log_level: LogLevel) -> Self {
        Self {
            log_level,
            begin: None,
            end: None,
            frames: 0,
            total_frames: 0,
            lost_frames: 0,
            lost_frames_sum: 0,
            highest_lost_frames: 0,
            frames_sum: 0.0,
            iterations: 0,
            target_frame_len,
            duration: 0.0,
            start_of_second: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is a construct"),
        }
    }

    pub fn start(&mut self) {
        self.begin = Some(SystemTime::now());
    }

    pub fn current_frame_len(&self) -> Result<Duration, std::time::SystemTimeError> {
        Ok(SystemTime::now().duration_since(self.begin.unwrap())?)
    }

    pub fn should_advance(&self) -> bool {
        self.target_frame_len.is_none()
            || self
                .current_frame_len()
                .map(|dur| dur.as_secs_f64())
                .unwrap_or_default()
                >= self.target_frame_len.unwrap()
    }

    pub fn stop(&mut self) {
        self.end = Some(SystemTime::now());

        if self.log_level == LogLevel::Trace {
            trace!(
                "> Measured Frame Length: {},\tTarget Frame Length: {},\tLoss: {}",
                self.current_frame_len().unwrap_or_default().as_secs_f64(),
                self.target_frame_len.unwrap_or_default(),
                (self.current_frame_len().unwrap_or_default().as_secs_f64()
                    - self.target_frame_len.unwrap_or_default())
                .abs()
            );
        }
        self.frames_sum += self.current_frame_len().unwrap_or_default().as_secs_f64();

        self.frames += 1;

        self.duration = self
            .end
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .expect("time is a construct")
            .as_secs_f64()
            - self.start_of_second.as_secs_f64();

        if self.duration >= 1.0 {
            self.start_of_second = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is a construct");
            self.total_frames += self.frames;

            if self.log_level == LogLevel::Trace {
                trace!(
                    "< Frames {},\tDuration: {},\tExpected {} Frames: {},\tLost Frames: {}",
                    self.frames,
                    self.duration,
                    self.frames,
                    self.frames_sum,
                    self.lost_frames
                );
            }

            if self.lost_frames > self.highest_lost_frames {
                self.highest_lost_frames = self.lost_frames;
            }
            self.frames = 0;
            self.lost_frames = 0;
            self.frames_sum = 0.0;
            self.iterations += 1;
        }
    }

    pub fn exit_stats(&self) {
        info!(
            ">> Iterations: {},\tFPS: {},\tTotal Lost Frames: {},\tAverage: {},\tHigh:{}",
            self.iterations,
            self.total_frames / self.iterations,
            self.lost_frames_sum,
            self.lost_frames_sum / self.iterations,
            self.highest_lost_frames
        );
    }
}
