//! The gradient test pattern: a smooth ramp between two colors, drawn two ways.
//!
//! The top half is plain: every pixel shows the nearest 8-bit code, so the ramp steps in
//! visible bands. The bottom half is dithered: pixels near a band edge flip between the two
//! neighbouring codes, so the average the eye sees moves smoothly between the bands.

use super::color;

/// A 4 by 4 pattern of the numbers 0 to 15, laid out so that numbers close in value are far
/// apart on the screen (a Bayer matrix).
const BAYER4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// The value of the ramp at column `x` of `width`, in codes (it can fall between two codes).
/// The ramp runs from `from` at the first column to `to` at the last.
pub fn ramp(from: u8, to: u8, x: u32, width: u32) -> f64 {
    if width <= 1 {
        return f64::from(from);
    }
    let t = f64::from(x) / f64::from(width - 1);
    f64::from(from) + (f64::from(to) - f64::from(from)) * t
}

/// The nearest 8-bit code to `value`: what the plain half shows.
pub fn plain(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

/// How the bottom half of the gradient is dithered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dither {
    /// A fixed pattern over the screen that also shifts every frame (the default).
    Ordered,
    /// Every pixel of a column flips together, in time only: the 4 frames of a cycle use 4
    /// evenly spaced numbers, the same for the whole screen.
    TimeOnly,
}

impl Dither {
    pub fn next(self) -> Self {
        match self {
            Dither::Ordered => Dither::TimeOnly,
            Dither::TimeOnly => Dither::Ordered,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Dither::Ordered => "ordered",
            Dither::TimeOnly => "time only",
        }
    }

    /// The number from 0 to 15 for pixel (`x`, `y`) on frame `phase`.
    pub fn threshold(self, x: u32, y: u32, phase: u32) -> u8 {
        match self {
            Dither::Ordered => threshold(x, y, phase),
            Dither::TimeOnly => (2 + 4 * (phase % 4)) as u8,
        }
    }
}

/// The number from 0 to 15 that decides whether pixel (`x`, `y`) rounds up on frame `phase`.
///
/// Across a 4 by 4 block of pixels the sixteen numbers are all different. At one pixel, four
/// frames in a row visit four numbers spaced evenly through 0 to 15. The pattern repeats
/// every 4 frames.
pub fn threshold(x: u32, y: u32, phase: u32) -> u8 {
    let spatial = BAYER4[(y % 4) as usize][(x % 4) as usize];
    (spatial + 4 * (phase % 4) as u8) % 16
}

/// The code pixel (`x`, `y`) shows for `value` on frame `phase`: the code just below the value,
/// or the one above it, so that over the pixels and the frames they average out to `value`.
/// A value that is exactly a code always shows that code.
pub fn dithered(value: f64, x: u32, y: u32, phase: u32, dither: Dither) -> u8 {
    let up = (f64::from(dither.threshold(x, y, phase)) + 0.5) / 16.0;
    (value + up).floor().clamp(0.0, 255.0) as u8
}

/// Fill one row of the pattern, `y` rows from the top of a window `height` rows tall.
///
/// Each channel ramps by itself, from `from` to `to` across the row. The top half of the
/// window is plain and the bottom half is dithered on frame `phase`.
pub fn fill_row(
    row: &mut [u32],
    y: u32,
    height: u32,
    from: [u8; 3],
    to: [u8; 3],
    phase: u32,
    dither: Dither,
) {
    let width = row.len() as u32;
    let top = y < height / 2;
    for (x, pixel) in row.iter_mut().enumerate() {
        let x = x as u32;
        let code = |ch: usize| {
            let value = ramp(from[ch], to[ch], x, width);
            if top {
                plain(value)
            } else {
                dithered(value, x, y, phase, dither)
            }
        };
        *pixel = color::pack_bgra([code(0), code(1), code(2)]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ramp_starts_and_ends_on_its_two_codes() {
        assert_eq!(ramp(96, 112, 0, 1280), 96.0);
        assert_eq!(ramp(96, 112, 1279, 1280), 112.0);
        assert_eq!(ramp(112, 96, 1279, 1280), 96.0, "it can run downwards");
        assert_eq!(ramp(96, 112, 0, 1), 96.0, "a one-column window");
        assert_eq!(ramp(96, 112, 0, 0), 96.0, "an empty window");
    }

    #[test]
    fn the_ramp_passes_through_the_middle() {
        assert!((ramp(0, 100, 50, 101) - 50.0).abs() < 1e-12);
    }

    #[test]
    fn plain_is_the_nearest_code() {
        assert_eq!(plain(99.4), 99);
        assert_eq!(plain(99.6), 100);
        assert_eq!(plain(-3.0), 0);
        assert_eq!(plain(300.0), 255);
    }

    #[test]
    fn the_sixteen_numbers_of_a_block_are_all_different() {
        for phase in 0..4 {
            let mut seen = [false; 16];
            for y in 0..4 {
                for x in 0..4 {
                    seen[usize::from(threshold(x, y, phase))] = true;
                }
            }
            assert!(seen.iter().all(|&s| s), "phase {phase}");
        }
    }

    #[test]
    fn four_frames_visit_four_evenly_spaced_numbers_at_every_pixel() {
        for y in 0..4 {
            for x in 0..4 {
                let mut seen: Vec<u8> = (0..4).map(|p| threshold(x, y, p)).collect();
                seen.sort_unstable();
                assert_eq!(seen[1] - seen[0], 4, "pixel {x},{y}");
                assert_eq!(seen[2] - seen[1], 4);
                assert_eq!(seen[3] - seen[2], 4);
            }
        }
    }

    #[test]
    fn the_pattern_repeats_every_four_frames() {
        for phase in 0..8 {
            assert_eq!(threshold(3, 1, phase), threshold(3, 1, phase + 4));
        }
    }

    #[test]
    fn a_value_that_is_exactly_a_code_never_flickers() {
        for code in [0u8, 1, 96, 127, 254, 255] {
            for phase in 0..4 {
                for y in 0..4 {
                    for x in 0..4 {
                        assert_eq!(
                            dithered(f64::from(code), x, y, phase, Dither::Ordered),
                            code
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_pixel_shows_only_the_code_below_or_the_code_above() {
        for i in 0..=200 {
            let v = 100.0 + f64::from(i) / 200.0 * 3.0;
            for phase in 0..4 {
                for (x, y) in [(0, 0), (1, 2), (3, 3)] {
                    let c = dithered(v, x, y, phase, Dither::Ordered);
                    assert!(c == v.floor() as u8 || c == v.ceil() as u8, "{v} gave {c}");
                }
            }
        }
    }

    #[test]
    fn a_block_of_pixels_averages_to_the_value_on_every_single_frame() {
        // Neighbouring pixels share the flicker out: one frame is enough to see the value.
        for i in 0..=300 {
            let v = 100.0 + f64::from(i) / 300.0 * 2.0;
            for phase in 0..4 {
                let mut sum = 0.0;
                for y in 0..4 {
                    for x in 0..4 {
                        sum += f64::from(dithered(v, x, y, phase, Dither::Ordered));
                    }
                }
                let mean = sum / 16.0;
                assert!((mean - v).abs() <= 1.0 / 32.0 + 1e-9, "{v}: {mean}");
            }
        }
    }

    #[test]
    fn one_pixel_over_four_frames_averages_to_the_value_within_a_quarter_code() {
        for i in 0..=300 {
            let v = 100.0 + f64::from(i) / 300.0 * 2.0;
            for (x, y) in [(0, 0), (1, 0), (2, 1), (3, 3)] {
                let sum: f64 = (0..4)
                    .map(|p| f64::from(dithered(v, x, y, p, Dither::Ordered)))
                    .sum();
                assert!((sum / 4.0 - v).abs() <= 0.25 + 1e-9, "{v} at {x},{y}");
            }
        }
    }

    #[test]
    fn the_plain_half_steps_in_bands_and_the_dithered_half_uses_the_same_codes() {
        // 16 codes across 1280 pixels: the plain row has 17 flat bands (96 to 112)
        let (from, to, height) = ([96; 3], [112; 3], 100);
        let mut top = vec![0u32; 1280];
        let mut bottom = vec![0u32; 1280];
        fill_row(&mut top, 10, height, from, to, 0, Dither::Ordered);
        fill_row(&mut bottom, 60, height, from, to, 0, Dither::Ordered);

        let blue = |p: u32| (p & 0xFF) as u8;
        let mut bands: Vec<u8> = top.iter().map(|&p| blue(p)).collect();
        bands.dedup();
        assert_eq!(bands.len(), 17, "{bands:?}");
        assert!(
            top.windows(2).all(|w| blue(w[0]) <= blue(w[1])),
            "never goes down"
        );

        assert!(bottom.iter().all(|&p| (96..=112).contains(&blue(p))));
        assert_ne!(top, bottom, "the two halves are not the same");
        // the dithered row has many more changes of code than the plain one
        let changes = |row: &[u32]| row.windows(2).filter(|w| w[0] != w[1]).count();
        assert!(changes(&bottom) > 10 * changes(&top));
    }

    #[test]
    fn the_halves_split_at_the_middle_row() {
        let (from, to) = ([100; 3], [101; 3]);
        let mut row = vec![0u32; 64];
        fill_row(&mut row, 49, 100, from, to, 0, Dither::Ordered);
        let plain_row = row.clone();
        fill_row(&mut row, 50, 100, from, to, 0, Dither::Ordered);
        assert_ne!(row, plain_row, "row 50 of 100 is the first dithered row");
    }

    #[test]
    fn every_channel_ramps_by_itself_and_alpha_is_opaque() {
        let mut row = vec![0u32; 256];
        fill_row(
            &mut row,
            0,
            100,
            [10, 100, 200],
            [10, 110, 100],
            0,
            Dither::Ordered,
        );
        let (first, last) = (row[0], row[255]);
        let ch = |p: u32| ((p >> 16) as u8, (p >> 8) as u8, p as u8); // red, green, blue
        assert_eq!(ch(first), (10, 100, 200));
        assert_eq!(ch(last), (10, 110, 100));
        assert!(row.iter().all(|&p| p >> 24 == 0xFF));
        assert!(row.iter().all(|&p| ch(p).0 == 10), "red stays flat");
    }

    #[test]
    fn time_only_dither_is_the_same_for_every_pixel_on_a_frame() {
        for phase in 0..4 {
            let first = Dither::TimeOnly.threshold(0, 0, phase);
            for (x, y) in [(1, 0), (2, 3), (7, 9), (1000, 500)] {
                assert_eq!(Dither::TimeOnly.threshold(x, y, phase), first);
            }
        }
    }

    #[test]
    fn time_only_visits_four_evenly_spaced_numbers_in_four_frames() {
        let seen: Vec<u8> = (0..4)
            .map(|p| Dither::TimeOnly.threshold(0, 0, p))
            .collect();
        assert_eq!(seen, vec![2, 6, 10, 14]);
        assert_eq!(
            Dither::TimeOnly.threshold(0, 0, 4),
            2,
            "repeats every 4 frames"
        );
    }

    #[test]
    fn time_only_averages_to_the_value_over_four_frames_within_a_quarter_code() {
        for i in 0..=300 {
            let v = 100.0 + f64::from(i) / 300.0 * 2.0;
            let sum: f64 = (0..4)
                .map(|p| f64::from(dithered(v, 5, 7, p, Dither::TimeOnly)))
                .sum();
            assert!((sum / 4.0 - v).abs() <= 0.25 + 1e-9, "{v}");
        }
    }

    #[test]
    fn time_only_never_flickers_on_an_exact_code() {
        for phase in 0..4 {
            assert_eq!(dithered(120.0, 3, 3, phase, Dither::TimeOnly), 120);
        }
    }

    #[test]
    fn the_two_dithers_switch_back_and_forth_with_names() {
        assert_eq!(Dither::Ordered.next(), Dither::TimeOnly);
        assert_eq!(Dither::TimeOnly.next(), Dither::Ordered);
        assert_ne!(Dither::Ordered.name(), Dither::TimeOnly.name());
    }
}
