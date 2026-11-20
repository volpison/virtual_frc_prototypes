//! Measuring how steadily frames reach the monitor.
//!
//! Pure: the caller hands in the time of each frame, so this can be tested without a clock.

use std::fmt;
use std::time::{Duration, Instant};

/// A gap this long is a pause (the window was dragged, hidden or minimised), not lost frames.
const PAUSE: Duration = Duration::from_millis(500);

/// How much time one report covers.
const REPORT_EVERY: Duration = Duration::from_secs(1);

/// How many refreshes were skipped between two frames shown `interval` apart, on a monitor
/// that refreshes every `period`.
///
/// Anything under one and a half periods counts as on time, so ordinary jitter is not a loss.
pub fn missed_refreshes(interval: Duration, period: Duration) -> u32 {
    let ratio = interval.as_secs_f64() / period.as_secs_f64();
    if ratio < 1.5 {
        0
    } else {
        ratio.round() as u32 - 1
    }
}

/// The value below which a share `q` (0.0 to 1.0) of the sorted values fall (nearest rank).
fn percentile(sorted: &[f64], q: f64) -> f64 {
    let n = sorted.len();
    let rank = (q * n as f64).ceil() as usize;
    sorted[rank.saturating_sub(1).min(n - 1)]
}

/// What one stretch of frames looked like.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub frames: u32,
    pub fps: f64,
    pub mean_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    /// Monitor refreshes that passed without a new frame.
    pub missed: u32,
    /// The time between refreshes the monitor is expected to have.
    pub period_ms: f64,
}

impl Report {
    /// Whether this stretch looks wrong for the monitor: lost refreshes, a slow tail, or an
    /// average rate that is more than 5% off the refresh rate.
    pub fn has_problem(&self) -> bool {
        self.missed > 0
            || self.p99_ms > 1.5 * self.period_ms
            || (self.mean_ms / self.period_ms - 1.0).abs() > 0.05
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} frames, {:.1} fps | between frames: mean {:.2} ms, p99 {:.2} ms, max {:.2} ms \
             (monitor period {:.2} ms) | missed refreshes: {}",
            self.frames,
            self.fps,
            self.mean_ms,
            self.p99_ms,
            self.max_ms,
            self.period_ms,
            self.missed
        )
    }
}

/// Collects the time of every shown frame and reports once a second.
pub struct FrameTimer {
    period: Duration,
    last_frame: Option<Instant>,
    window_start: Instant,
    intervals_ms: Vec<f64>,
    window_missed: u32,
    fps: f64,
    total_frames: u64,
    total_missed: u64,
}

impl FrameTimer {
    pub fn new(refresh_hz: f64) -> Self {
        Self {
            period: Self::period_of(refresh_hz),
            last_frame: None,
            window_start: Instant::now(),
            intervals_ms: Vec::new(),
            window_missed: 0,
            fps: 0.0,
            total_frames: 0,
            total_missed: 0,
        }
    }

    fn period_of(refresh_hz: f64) -> Duration {
        Duration::from_secs_f64(1.0 / refresh_hz.max(1.0))
    }

    /// The monitor changed (the window moved to another one): start measuring afresh.
    pub fn set_refresh_hz(&mut self, refresh_hz: f64) {
        self.period = Self::period_of(refresh_hz);
        self.restart_window();
    }

    fn restart_window(&mut self) {
        self.last_frame = None;
        self.intervals_ms.clear();
        self.window_missed = 0;
    }

    /// Note that a frame was shown at `now`. Returns a report when a second has passed.
    pub fn record(&mut self, now: Instant) -> Option<Report> {
        self.total_frames += 1;
        let Some(last) = self.last_frame.replace(now) else {
            self.window_start = now;
            return None;
        };
        let interval = now.duration_since(last);
        if interval > PAUSE {
            self.restart_window();
            self.last_frame = Some(now);
            self.window_start = now;
            return None;
        }

        let missed = missed_refreshes(interval, self.period);
        self.total_missed += u64::from(missed);
        self.window_missed += missed;
        self.intervals_ms.push(interval.as_secs_f64() * 1000.0);

        let elapsed = now.duration_since(self.window_start);
        if elapsed < REPORT_EVERY {
            return None;
        }

        let mut sorted = self.intervals_ms.clone();
        sorted.sort_by(f64::total_cmp);
        let frames = sorted.len();
        let report = Report {
            frames: frames as u32,
            fps: frames as f64 / elapsed.as_secs_f64(),
            mean_ms: sorted.iter().sum::<f64>() / frames as f64,
            p99_ms: percentile(&sorted, 0.99),
            max_ms: sorted[frames - 1],
            missed: self.window_missed,
            period_ms: self.period.as_secs_f64() * 1000.0,
        };
        self.fps = report.fps;
        self.intervals_ms.clear();
        self.window_missed = 0;
        self.window_start = now;
        Some(report)
    }

    /// Frames per second over the last report (0 before the first one).
    pub fn fps(&self) -> f64 {
        self.fps
    }

    pub fn total_frames(&self) -> u64 {
        self.total_frames
    }

    /// Monitor refreshes that passed without a new frame, since the start.
    pub fn total_missed(&self) -> u64 {
        self.total_missed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(x: f64) -> Duration {
        Duration::from_secs_f64(x / 1000.0)
    }

    /// Feed frames `gaps_ms` apart; returns every report made.
    fn run(timer: &mut FrameTimer, start: Instant, gaps_ms: &[f64]) -> Vec<Report> {
        let mut t = start;
        let mut reports: Vec<Report> = timer.record(t).into_iter().collect();
        for &gap in gaps_ms {
            t += ms(gap);
            reports.extend(timer.record(t));
        }
        reports
    }

    #[test]
    fn a_refresh_is_missed_from_one_and_a_half_periods() {
        let p = ms(10.0);
        assert_eq!(missed_refreshes(Duration::ZERO, p), 0);
        assert_eq!(missed_refreshes(ms(10.0), p), 0);
        assert_eq!(missed_refreshes(ms(14.9), p), 0);
        assert_eq!(missed_refreshes(ms(15.1), p), 1);
        assert_eq!(missed_refreshes(ms(20.0), p), 1);
        assert_eq!(missed_refreshes(ms(30.0), p), 2);
    }

    #[test]
    fn percentile_takes_the_nearest_rank() {
        let v: Vec<f64> = (1..=100).map(f64::from).collect();
        assert_eq!(percentile(&v, 0.99), 99.0);
        assert_eq!(percentile(&v, 1.0), 100.0);
        assert_eq!(percentile(&v, 0.5), 50.0);
        assert_eq!(percentile(&[7.0], 0.99), 7.0);
    }

    #[test]
    fn steady_frames_give_a_clean_report_each_second() {
        let mut timer = FrameTimer::new(100.0);
        let reports = run(&mut timer, Instant::now(), &[10.0; 300]);
        assert_eq!(
            reports.len(),
            3,
            "300 frames of 10 ms is 3 s: one report per second"
        );
        for r in &reports {
            assert!((r.fps - 100.0).abs() < 1.0, "{r}");
            assert!((r.mean_ms - 10.0).abs() < 1e-6, "{r}");
            assert_eq!(r.missed, 0);
            assert!(!r.has_problem(), "{r}");
        }
        assert_eq!(timer.total_missed(), 0);
        assert!((timer.fps() - 100.0).abs() < 1.0);
    }

    #[test]
    fn a_skipped_refresh_is_counted_and_flagged() {
        let mut timer = FrameTimer::new(100.0);
        let mut gaps = vec![10.0; 120];
        gaps[50] = 20.0; // one refresh passed without a frame
        let reports = run(&mut timer, Instant::now(), &gaps);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].missed, 1);
        assert_eq!(reports[0].max_ms, 20.0);
        assert!(reports[0].has_problem());
        assert_eq!(timer.total_missed(), 1);
    }

    #[test]
    fn two_lost_refreshes_in_one_gap_count_as_two() {
        let mut timer = FrameTimer::new(100.0);
        run(&mut timer, Instant::now(), &[10.0, 30.0, 10.0]);
        assert_eq!(timer.total_missed(), 2);
    }

    #[test]
    fn a_long_pause_is_not_counted_as_lost_frames() {
        let mut timer = FrameTimer::new(100.0);
        let mut gaps = vec![10.0; 50];
        gaps.push(2000.0); // the window was dragged for two seconds
        gaps.extend([10.0; 150]);
        let reports = run(&mut timer, Instant::now(), &gaps);
        assert_eq!(timer.total_missed(), 0);
        assert!(reports.iter().all(|r| !r.has_problem()), "{reports:?}");
    }

    #[test]
    fn the_wrong_average_rate_is_flagged_even_without_missed_refreshes() {
        // 144 Hz expected, but frames arrive every 10 ms (100 Hz): vsync is off or the monitor is not at 144 Hz
        let mut timer = FrameTimer::new(144.0);
        let reports = run(&mut timer, Instant::now(), &[10.0; 120]);
        assert_eq!(reports[0].missed, 0);
        assert!(reports[0].has_problem(), "{}", reports[0]);
    }

    #[test]
    fn changing_the_monitor_changes_what_is_on_time() {
        let mut timer = FrameTimer::new(144.0);
        timer.set_refresh_hz(60.0);
        run(&mut timer, Instant::now(), &[16.7; 10]);
        assert_eq!(timer.total_missed(), 0);
    }

    #[test]
    fn the_first_frame_makes_no_interval_but_is_counted() {
        let mut timer = FrameTimer::new(60.0);
        assert!(timer.record(Instant::now()).is_none());
        assert_eq!(timer.total_frames(), 1);
        assert_eq!(timer.fps(), 0.0);
    }

    #[test]
    fn the_report_reads_in_plain_words() {
        let mut timer = FrameTimer::new(100.0);
        let r = &run(&mut timer, Instant::now(), &[10.0; 120])[0];
        let text = r.to_string();
        assert!(
            text.contains("fps") && text.contains("p99") && text.contains("missed refreshes: 0"),
            "{text}"
        );
    }
}
