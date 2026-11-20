//! Colors as three numbers (red, green, blue), each from 0.0 to 1.0.

/// A color in the precision the program edits it in.
pub type Rgb64 = [f64; 3];

/// The same color in the precision the graphics card takes.
pub fn to_f32(c: Rgb64) -> [f32; 3] {
    [c[0] as f32, c[1] as f32, c[2] as f32]
}

/// The color a frame shows: A when `use_a` is true, otherwise B.
pub fn select(use_a: bool, a: Rgb64, b: Rgb64) -> [f32; 3] {
    to_f32(if use_a { a } else { b })
}

/// The plain average of two colors, channel by channel: what the eye should see if
/// the two are flipped fast enough and in equal shares.
pub fn average(a: Rgb64, b: Rgb64) -> [f32; 3] {
    [
        ((a[0] + b[0]) * 0.5) as f32,
        ((a[1] + b[1]) * 0.5) as f32,
        ((a[2] + b[2]) * 0.5) as f32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Rgb64 = [0.40, 0.70, 0.30];
    const B: Rgb64 = [0.30, 0.55, 0.80];

    #[test]
    fn to_f32_keeps_exact_values_and_rounds_the_rest() {
        assert_eq!(to_f32([0.0, 0.5, 1.0]), [0.0, 0.5, 1.0]);
        assert_eq!(to_f32([0.4, 0.7, 0.3]), [0.4f32, 0.7f32, 0.3f32]);
    }

    #[test]
    fn select_picks_a_or_b() {
        assert_eq!(select(true, A, B), to_f32(A));
        assert_eq!(select(false, A, B), to_f32(B));
    }

    #[test]
    fn average_of_black_and_white_is_grey() {
        assert_eq!(average([0.0; 3], [1.0; 3]), [0.5, 0.5, 0.5]);
    }

    #[test]
    fn average_of_two_equal_colors_is_that_color() {
        assert_eq!(average(A, A), to_f32(A));
    }

    #[test]
    fn average_does_not_depend_on_the_order() {
        assert_eq!(average(A, B), average(B, A));
    }

    #[test]
    fn average_lies_between_the_two_colors_in_every_channel() {
        let steps = [0.0, 0.1, 0.25, 0.4, 0.5, 0.75, 0.9, 1.0];
        for &r1 in &steps {
            for &r2 in &steps {
                let (a, b) = ([r1, 0.2, 0.9], [r2, 0.8, 0.1]);
                let m = average(a, b);
                for ch in 0..3 {
                    let (lo, hi) = (a[ch].min(b[ch]) as f32, a[ch].max(b[ch]) as f32);
                    assert!(lo <= m[ch] && m[ch] <= hi, "channel {ch}: {a:?} {b:?}");
                }
            }
        }
    }
}
