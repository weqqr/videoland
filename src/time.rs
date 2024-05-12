use std::time::{Duration, Instant};

use crate::core::ResMut;

pub struct Time {
    start_of_previous_frame: Instant,
    dtime: Duration,
}

impl Time {
    pub fn new() -> Self {
        Self {
            start_of_previous_frame: Instant::now(),
            dtime: Duration::ZERO,
        }
    }

    pub fn fps(&self) -> f64 {
        1.0 / self.dtime.as_secs_f64()
    }

    pub fn dtime_s(&self) -> f64 {
        self.dtime.as_secs_f64()
    }

    pub fn dtime_ms(&self) -> f64 {
        self.dtime.as_secs_f64() * 1000.0
    }

    pub fn advance_frame(&mut self) {
        let now = Instant::now();
        self.dtime = now - self.start_of_previous_frame;
        self.start_of_previous_frame = now;
    }
}

pub fn advance(mut time: ResMut<Time>) {
    time.advance_frame();
}
