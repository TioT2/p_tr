pub struct Timer {
    start_time: std::time::Instant,

    new_paused: bool,
    paused: bool,

    global_delta_time: std::time::Duration,
    global_current_time: std::time::Instant,

    paused_delta_time: std::time::Duration,

    delta_time: std::time::Duration,
    current_time: std::time::Instant,

    fps_last_time: std::time::Instant,
    fps_frame_count: usize,
    fps_duration: std::time::Duration,

    fps: f64,
}

pub struct State<'a> {
    timer: &'a mut Timer,
    current_time: f64,
    delta_time: f64,
    global_current_time: f64,
    global_delta_time: f64,
    fps: f64,
}

impl<'a> State<'a> {
    pub fn get_is_paused(&self) -> bool {
        self.timer.paused
    }

    pub fn set_is_paused(&mut self, new_paused: bool) {
        self.timer.new_paused = new_paused;
    }

    pub fn get_global_delta_time(&self) -> f64 {
        self.global_delta_time
    }

    pub fn get_global_time(&self) -> f64 {
        self.global_current_time
    }

    pub fn get_delta_time(&self) -> f64 {
        self.delta_time
    }

    pub fn get_time(&self) -> f64 {
        self.current_time
    }

    pub fn get_fps(&self) -> f64 {
        self.fps
    }
}

impl Timer {
    pub fn new() -> Timer {
        let now = std::time::Instant::now();
        let small_delta = std::time::Duration::from_micros(1);

        Timer {
            start_time: now,

            new_paused: false,
            paused: false,
            global_delta_time: small_delta,
            global_current_time: now,
            paused_delta_time: small_delta,
            delta_time: small_delta,
            current_time: now,
            fps_last_time: now,
            fps_duration: std::time::Duration::from_secs_f64(1.0),
            fps_frame_count: 1,
            fps: 30.0,
        }
    }

    pub fn response(&mut self) {
        if self.paused != self.new_paused {
            self.paused = self.new_paused;

            if self.paused {
                self.delta_time = std::time::Duration::from_secs(0);
            } else {
                self.paused_delta_time = self.global_current_time - self.current_time;
            }
        }


        let new_global_time = std::time::Instant::now();

        self.global_delta_time = new_global_time.duration_since(self.global_current_time);
        self.global_current_time = new_global_time;

        if !self.paused {
            self.delta_time = self.global_delta_time;
            self.current_time = self
                .global_current_time
                .checked_sub(self.paused_delta_time)
                .unwrap();
        }

        let fps_current_duration = self.global_current_time.duration_since(self.fps_last_time);
        self.fps_frame_count += 1;
        if fps_current_duration > self.fps_duration {
            self.fps = self.fps_frame_count as f64 / fps_current_duration.as_secs_f64();
            self.fps_frame_count = 0;
            self.fps_last_time = self.global_current_time;
        }
    }

    pub fn get_state<'a>(&'a mut self) -> State<'a> {
        State {
            current_time: self
                .current_time
                .duration_since(self.start_time)
                .as_secs_f64(),
            delta_time: self.delta_time.as_secs_f64(),
            global_current_time: self
                .global_current_time
                .duration_since(self.start_time)
                .as_secs_f64(),
            global_delta_time: self.global_delta_time.as_secs_f64(),
            fps: self.fps,
            timer: self,
        }
    }
}
