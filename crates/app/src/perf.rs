use std::{
    collections::VecDeque,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::plugins::Plugin;
use ecs::{prelude::*, WinnyResource};
use logger::*;

#[derive(Debug, WinnyResource, Clone)]
pub(crate) struct PerfCounter {
    begin: Option<SystemTime>,
    begin_debug_event: Option<SystemTime>,
    end: Option<SystemTime>,
    end_debug_event: Option<SystemTime>,
    last_fram_duration: Option<Duration>,
    frames: usize,
    total_frames: usize,
    lost_frames: usize,
    lost_frames_sum: usize,
    highest_lost_frames: usize,
    frames_sum: f64,
    iterations: usize,
    // target_frame_len: Option<f64>,
    duration: f64,
    start_of_second: Duration,
    debug_events: VecDeque<(String, Duration)>,
}

impl PerfCounter {
    pub fn new(target_fps: Option<f64>) -> Self {
        let _target_frame_len = target_fps.map(|target| 1.0 / target);

        Self {
            begin: None,
            begin_debug_event: None,
            end: None,
            end_debug_event: None,
            last_fram_duration: None,
            frames: 0,
            total_frames: 0,
            lost_frames: 0,
            lost_frames_sum: 0,
            highest_lost_frames: 0,
            frames_sum: 0.0,
            iterations: 0,
            // target_frame_len,
            duration: 0.0,
            start_of_second: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is a construct"),
            debug_events: VecDeque::new(),
        }
    }

    // pub fn last_frame_duration(&self) -> Option<Duration> {
    //     self.last_fram_duration
    // }

    pub fn start(&mut self) {
        self.begin = Some(SystemTime::now());
    }

    pub fn start_debug_event(&mut self) {
        self.begin_debug_event = Some(SystemTime::now());
    }

    pub fn current_frame_len(&self) -> Result<Duration, std::time::SystemTimeError> {
        Ok(SystemTime::now().duration_since(self.begin.unwrap())?)
    }

    pub fn _should_advance(&self) -> bool {
        // self.target_frame_len.is_none()
        //     || self
        //         .current_frame_len()
        //         .map(|dur| dur.as_secs_f64())
        //         .unwrap_or_default()
        //         >= self.target_frame_len.unwrap()
        true
    }

    pub fn stop(&mut self) {
        self.end = Some(SystemTime::now());

        // info!(
        //     "> Measured Frame Length: {},\tTarget Frame Length: {},\tLoss: {}",
        //     self.current_frame_len().unwrap_or_default().as_secs_f64(),
        //     self.target_frame_len.unwrap_or_default(),
        //     (self.current_frame_len().unwrap_or_default().as_secs_f64()
        //         - self.target_frame_len.unwrap_or_default())
        //     .abs()
        // );
        self.frames_sum += self.current_frame_len().unwrap_or_default().as_secs_f64();

        self.frames += 1;

        self.last_fram_duration = Some(self.current_frame_len().unwrap_or_default());

        self.duration = self
            .end
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .expect("time is a construct")
            .as_secs_f64()
            - self.start_of_second.as_secs_f64();

        if self.duration >= 1.0 {
            for (label, duration) in self.debug_events.drain(..) {
                info!("{} => {}", label, duration.as_secs_f32());
            }

            self.start_of_second = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time is a construct");
            self.total_frames += self.frames;

            info!(
                "# Frames {},\tDuration: {},\tExpected {} Frames: {},\tLost Frames: {}",
                self.frames, self.duration, self.frames, self.frames_sum, self.lost_frames
            );

            if self.lost_frames > self.highest_lost_frames {
                self.highest_lost_frames = self.lost_frames;
            }
            self.frames = 0;
            self.lost_frames = 0;
            self.frames_sum = 0.0;
            self.iterations += 1;
        }

        self.debug_events.drain(..);
    }

    pub fn stop_debug_event(&mut self) {
        self.end_debug_event = Some(SystemTime::now());
    }

    pub fn log_last_debug_event(&mut self, label: &str) {
        if let Some(duration) = self.query_last_debug_event() {
            self.debug_events.push_back((label.into(), duration));
        }
    }

    pub fn query_last_debug_event(&self) -> Option<Duration> {
        if let Some(start) = self.begin_debug_event {
            if let Some(end) = self.end_debug_event {
                let dur = end.duration_since(start);
                if dur.is_ok() {
                    return Some(dur.unwrap());
                } else {
                    return None;
                }
            }
        }

        None
    }

    pub fn exit_stats(&self) {
        info!(
            ">> Iterations: {},\tFPS: {},\tTotal Lost Frames: {},\tAverage: {},\tHigh:{}",
            self.iterations,
            self.total_frames / self.iterations.max(1),
            self.lost_frames_sum,
            self.lost_frames_sum / self.iterations.max(1),
            self.highest_lost_frames
        );
    }
}

#[derive(Clone, Copy)]
pub struct PerfPlugin;

impl Plugin for PerfPlugin {
    fn build(&mut self, app: &mut crate::app::App) {
        app.insert_resource(PerfCounter::new(None))
            .add_systems(Schedule::Exit, exit_stats);
    }
}

fn exit_stats(perf: Res<PerfCounter>) {
    perf.exit_stats();
}
