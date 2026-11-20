//! What the program is doing right now: colors, flip pattern, view, drawing method and frame counts.

use std::time::{Duration, Instant};

use crate::frc::{color, pattern};

pub(crate) const DEFAULT_COLOR_A: [f64; 3] = [0.40, 0.70, 0.30];
pub(crate) const DEFAULT_COLOR_B: [f64; 3] = [0.30, 0.55, 0.80];
pub(crate) const STEP: f64 = 0.02;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DisplayMode {
    SideBySide = 0,
    FullTemporal = 1,
    Checkerboard = 2,
}

impl DisplayMode {
    pub(crate) fn next(self) -> Self {
        match self {
            DisplayMode::SideBySide => DisplayMode::FullTemporal,
            DisplayMode::FullTemporal => DisplayMode::Checkerboard,
            DisplayMode::Checkerboard => DisplayMode::SideBySide,
        }
    }
    pub(crate) fn name(self) -> &'static str {
        match self {
            DisplayMode::SideBySide => "Side-by-side",
            DisplayMode::FullTemporal => "Full temporal",
            DisplayMode::Checkerboard => "Checkerboard",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Backend {
    Gpu,
    Software,
}

impl Backend {
    pub(crate) fn toggle(self) -> Self {
        match self {
            Backend::Gpu => Backend::Software,
            Backend::Software => Backend::Gpu,
        }
    }
    pub(crate) fn name(self) -> &'static str {
        match self {
            Backend::Gpu => "GPU",
            Backend::Software => "Software+SIMD",
        }
    }
}

pub(crate) struct Metrics {
    frame_count: u64,
    last_report: Instant,
    current_fps: f64,
    total_frames: u64,
}

impl Metrics {
    pub(crate) fn new() -> Self {
        Self {
            frame_count: 0,
            last_report: Instant::now(),
            current_fps: 0.0,
            total_frames: 0,
        }
    }
    pub(crate) fn record_frame(&mut self) {
        self.frame_count += 1;
        self.total_frames += 1;
        let elapsed = self.last_report.elapsed();
        if elapsed >= Duration::from_millis(400) {
            self.current_fps = self.frame_count as f64 / elapsed.as_secs_f64();
            self.frame_count = 0;
            self.last_report = Instant::now();
        }
    }
}

pub(crate) struct AppState {
    pub(crate) color_a: [f64; 3],
    pub(crate) color_b: [f64; 3],
    pub(crate) editing_a: bool,
    pub(crate) duty_a: u32,
    pub(crate) frame_phase: u32,
    pub(crate) metrics: Metrics,
    pub(crate) monitor_hz: f64,
    pub(crate) use_vsync: bool,
    pub(crate) mode: DisplayMode,
    pub(crate) backend: Backend,
    pub(crate) cell_size: u32,
}

impl AppState {
    pub(crate) fn new(monitor_hz: f64) -> Self {
        Self {
            color_a: DEFAULT_COLOR_A,
            color_b: DEFAULT_COLOR_B,
            editing_a: true,
            duty_a: 1,
            frame_phase: 0,
            metrics: Metrics::new(),
            monitor_hz,
            use_vsync: true,
            mode: DisplayMode::SideBySide,
            backend: Backend::Gpu,
            cell_size: 8,
        }
    }

    pub(crate) fn temporal_color(&self) -> [f32; 3] {
        let use_a = pattern::shows_a(self.frame_phase, self.duty_a);
        color::select(use_a, self.color_a, self.color_b)
    }

    pub(crate) fn static_average(&self) -> [f32; 3] {
        color::average(self.color_a, self.color_b)
    }

    pub(crate) fn color_a_f32(&self) -> [f32; 3] {
        color::to_f32(self.color_a)
    }
    pub(crate) fn color_b_f32(&self) -> [f32; 3] {
        color::to_f32(self.color_b)
    }

    pub(crate) fn advance_phase(&mut self) {
        self.frame_phase = self.frame_phase.wrapping_add(1);
    }

    pub(crate) fn adjust_channel(&mut self, channel: usize, delta: f64) {
        let color = if self.editing_a {
            &mut self.color_a
        } else {
            &mut self.color_b
        };
        color[channel] = (color[channel] + delta).clamp(0.0, 1.0);
    }

    pub(crate) fn title_string(&self) -> String {
        let which = if self.editing_a { "A" } else { "B" };
        let present = if self.use_vsync { "Fifo" } else { "Imm" };
        format!(
            "VirtualFRC  |  {:.0} FPS  |  {}  |  {}  |  {}  |  edit {}  |  duty {}:1  |  cell {}  |  frames:{}",
            self.metrics.current_fps,
            self.backend.name(),
            present,
            self.mode.name(),
            which,
            self.duty_a,
            self.cell_size,
            self.metrics.total_frames,
        )
    }
}
