//! What the program is doing right now: colors, flip pattern, view, drawing method and frame counts.

use crate::frc::timing::FrameTimer;
use crate::frc::{color, pattern};

/// The two colors flipped between, as 8-bit codes. They start one step apart, which is
/// the case frame rate control is for.
pub(crate) const DEFAULT_COLOR_A: [u8; 3] = [127, 127, 127];
pub(crate) const DEFAULT_COLOR_B: [u8; 3] = [128, 128, 128];
/// How far one key press moves a code, and how far with Shift held.
pub(crate) const STEP: i32 = 1;
pub(crate) const COARSE_STEP: i32 = 10;

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

pub(crate) struct AppState {
    pub(crate) color_a: [u8; 3],
    pub(crate) color_b: [u8; 3],
    pub(crate) editing_a: bool,
    pub(crate) duty_a: u32,
    pub(crate) frame_phase: u32,
    pub(crate) metrics: FrameTimer,
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
            metrics: FrameTimer::new(monitor_hz),
            monitor_hz,
            use_vsync: true,
            mode: DisplayMode::SideBySide,
            backend: Backend::Gpu,
            cell_size: 8,
        }
    }

    /// The color this frame shows on the flipping side.
    pub(crate) fn temporal_color(&self) -> [u8; 3] {
        if pattern::shows_a(self.frame_phase, self.duty_a) {
            self.color_a
        } else {
            self.color_b
        }
    }

    /// The plain color on the other side: the nearest 8-bit color to what the eye should
    /// see from the flipping, weighted by how often A is shown.
    pub(crate) fn static_average(&self) -> [u8; 3] {
        color::mix(self.color_a, self.color_b, pattern::share_a(self.duty_a))
    }

    pub(crate) fn advance_phase(&mut self) {
        self.frame_phase = pattern::next_phase(self.frame_phase);
    }

    /// Move one channel of the selected color by `delta` codes, staying within 0 to 255.
    pub(crate) fn adjust_channel(&mut self, channel: usize, delta: i32) {
        let color = if self.editing_a {
            &mut self.color_a
        } else {
            &mut self.color_b
        };
        color[channel] = (i32::from(color[channel]) + delta).clamp(0, 255) as u8;
    }

    pub(crate) fn title_string(&self) -> String {
        let which = if self.editing_a { "A" } else { "B" };
        let present = if self.use_vsync { "Fifo" } else { "Imm" };
        format!(
            "VirtualFRC  |  {:.0} FPS  |  {}  |  {}  |  {}  |  edit {}  |  A {},{},{}  B {},{},{}  |  duty {}:1  |  cell {}  |  frames:{}  missed:{}",
            self.metrics.fps(),
            self.backend.name(),
            present,
            self.mode.name(),
            which,
            self.color_a[0],
            self.color_a[1],
            self.color_a[2],
            self.color_b[0],
            self.color_b[1],
            self.color_b[2],
            self.duty_a,
            self.cell_size,
            self.metrics.total_frames(),
            self.metrics.total_missed(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        AppState::new(144.0)
    }

    #[test]
    fn the_default_colors_are_one_code_apart_in_every_channel() {
        for ch in 0..3 {
            assert_eq!(DEFAULT_COLOR_B[ch] - DEFAULT_COLOR_A[ch], 1);
        }
    }

    #[test]
    fn a_key_press_moves_the_selected_color_by_the_step() {
        let mut s = state();
        s.adjust_channel(0, STEP);
        s.adjust_channel(2, -COARSE_STEP);
        assert_eq!(s.color_a, [128, 127, 117]);
        assert_eq!(s.color_b, DEFAULT_COLOR_B, "the other color is untouched");
        s.editing_a = false;
        s.adjust_channel(1, COARSE_STEP);
        assert_eq!(s.color_b, [128, 138, 128]);
    }

    #[test]
    fn codes_stop_at_zero_and_at_255() {
        let mut s = state();
        s.adjust_channel(0, -1000);
        s.adjust_channel(1, 1000);
        assert_eq!(s.color_a, [0, 255, 127]);
        s.adjust_channel(0, -STEP);
        s.adjust_channel(1, STEP);
        assert_eq!(s.color_a, [0, 255, 127]);
    }

    #[test]
    fn the_flipping_side_follows_the_pattern() {
        let mut s = state();
        assert_eq!(s.temporal_color(), DEFAULT_COLOR_A, "frame 0 shows A");
        s.advance_phase();
        assert_eq!(s.temporal_color(), DEFAULT_COLOR_B, "1:1 shows B next");
        s.duty_a = 3;
        s.frame_phase = 3;
        assert_eq!(
            s.temporal_color(),
            DEFAULT_COLOR_B,
            "3:1 shows B on the 4th frame"
        );
        s.frame_phase = 2;
        assert_eq!(s.temporal_color(), DEFAULT_COLOR_A);
    }

    #[test]
    fn the_plain_side_follows_how_often_a_is_shown() {
        let mut s = state();
        s.color_a = [255; 3];
        s.color_b = [0; 3];
        let half = s.static_average()[0];
        s.duty_a = 3;
        let three_quarters = s.static_average()[0];
        assert_eq!(half, 188, "equal shares of white and black light");
        assert!(
            three_quarters > half,
            "more A is brighter: {three_quarters}"
        );
    }

    #[test]
    fn the_plain_side_is_one_of_the_two_codes_when_they_are_a_step_apart() {
        let m = state().static_average()[0];
        assert!(m == 127 || m == 128, "{m}");
    }

    #[test]
    fn the_frame_counter_starts_over_without_a_jump_in_the_pattern() {
        for duty in 1..=3 {
            let mut s = state();
            s.duty_a = duty;
            s.frame_phase = pattern::PHASE_PERIOD - 2;
            let mut shown = Vec::new();
            for _ in 0..8 {
                shown.push(s.temporal_color() == DEFAULT_COLOR_A);
                s.advance_phase();
            }
            // the same 8 frames, counted without the counter ever starting over
            let expected: Vec<bool> = (0..8)
                .map(|i| pattern::shows_a(pattern::PHASE_PERIOD - 2 + i, duty))
                .collect();
            assert_eq!(shown, expected, "duty {duty}");
            assert!(s.frame_phase < 8, "the counter did start over");
        }
    }

    #[test]
    fn the_title_counts_shown_and_missed_frames() {
        let title = state().title_string();
        assert!(title.contains("frames:0  missed:0"), "{title}");
    }

    #[test]
    fn the_title_shows_both_colors_as_codes() {
        let title = state().title_string();
        assert!(title.contains("A 127,127,127  B 128,128,128"), "{title}");
    }
}
