//! The flip pattern: on which frames color A is shown and on which color B.

/// Whether color A (and not B) is shown on frame `phase`, when A is shown `duty_a`
/// times for every one time B is shown: 1 is 1:1, 2 is 2:1, 3 is 3:1.
///
/// A value of 0 behaves like 1. The pattern repeats every 2 frames for 1:1, and every
/// `duty_a + 1` frames above that. Frame 0 always shows A.
pub fn shows_a(phase: u32, duty_a: u32) -> bool {
    if duty_a <= 1 {
        phase & 1 == 0
    } else {
        (phase % (duty_a + 1)) < duty_a
    }
}

/// The share of frames that show color A (0.0 to 1.0), for the same `duty_a` as [`shows_a`].
///
/// 1:1 gives one half, 2:1 two thirds, 3:1 three quarters. This is the weight of A in the
/// average the eye should see.
pub fn share_a(duty_a: u32) -> f64 {
    if duty_a <= 1 {
        0.5
    } else {
        f64::from(duty_a) / (f64::from(duty_a) + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frames after which the pattern starts over.
    fn cycle(duty_a: u32) -> u32 {
        if duty_a <= 1 {
            2
        } else {
            duty_a + 1
        }
    }

    /// The longest run of equal values, looking at the pattern as a ring.
    fn longest_run(duty_a: u32, wanted: bool) -> u32 {
        let n = cycle(duty_a);
        let mut best = 0;
        for start in 0..n {
            let mut len = 0;
            while len < n && shows_a(start + len, duty_a) == wanted {
                len += 1;
            }
            best = best.max(len);
        }
        best
    }

    #[test]
    fn share_a_matches_the_frames_that_show_a() {
        for duty in 0..=7u32 {
            let frames = cycle(duty) * 100;
            let counted = (0..frames).filter(|&p| shows_a(p, duty)).count() as f64;
            assert!(
                (share_a(duty) - counted / f64::from(frames)).abs() < 1e-12,
                "duty {duty}"
            );
        }
    }

    #[test]
    fn share_a_has_the_known_values() {
        assert_eq!(share_a(0), 0.5);
        assert_eq!(share_a(1), 0.5);
        assert!((share_a(2) - 2.0 / 3.0).abs() < 1e-15);
        assert_eq!(share_a(3), 0.75);
    }

    #[test]
    fn frame_zero_shows_a() {
        for duty in 0..=8 {
            assert!(shows_a(0, duty), "duty {duty}");
        }
    }

    #[test]
    fn a_is_shown_duty_times_per_cycle() {
        for (duty, expected) in [(1, 1), (2, 2), (3, 3), (4, 4), (7, 7)] {
            let shown = (0..cycle(duty)).filter(|&p| shows_a(p, duty)).count();
            assert_eq!(shown, expected, "duty {duty}");
        }
    }

    #[test]
    fn long_run_share_of_a_is_duty_over_duty_plus_one() {
        for duty in 1..=7u32 {
            let cycles = 1000;
            let frames = cycle(duty) * cycles;
            let shown = (0..frames).filter(|&p| shows_a(p, duty)).count() as f64;
            let share = shown / frames as f64;
            let expected = duty as f64 / (duty as f64 + 1.0);
            assert!((share - expected).abs() < 1e-12, "duty {duty}: {share}");
        }
    }

    #[test]
    fn duty_zero_is_the_same_as_one() {
        for phase in 0..64 {
            assert_eq!(shows_a(phase, 0), shows_a(phase, 1));
        }
    }

    #[test]
    fn the_pattern_repeats_every_cycle() {
        for duty in 0..=8 {
            let n = cycle(duty);
            for phase in 0..(3 * n) {
                assert_eq!(shows_a(phase, duty), shows_a(phase + n, duty));
            }
        }
    }

    #[test]
    fn no_color_is_held_longer_than_its_share() {
        for duty in 1..=7 {
            assert_eq!(longest_run(duty, true), duty, "A, duty {duty}");
            assert_eq!(longest_run(duty, false), 1, "B, duty {duty}");
        }
    }

    #[test]
    fn the_pattern_still_works_far_into_a_long_session() {
        // A u32 counter reaches 4 billion after about 190 days at 240 frames a second.
        let far = 4_000_000_000u32;
        assert_eq!(shows_a(far, 1), far & 1 == 0);
        assert_eq!(shows_a(far, 3), (far % 4) < 3);
    }
}
