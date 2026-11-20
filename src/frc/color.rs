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

/// A light level (linear, 0.0 to 1.0) as the encoded value a screen expects (sRGB).
pub fn srgb_encode(linear: f64) -> f64 {
    let v = linear.clamp(0.0, 1.0);
    if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// The opposite of [`srgb_encode`]: an encoded value back to a light level.
pub fn srgb_decode(encoded: f64) -> f64 {
    let v = encoded.clamp(0.0, 1.0);
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// The 8-bit code (0 to 255) that shows a light level: encode, then round to the nearest code.
///
/// This is the one place that rounds to 8 bits, so both drawing methods show the same codes.
pub fn to_code(linear: f64) -> u8 {
    (srgb_encode(linear) * 255.0 + 0.5) as u8
}

/// The number to hand the graphics card so that it puts exactly `to_code(linear)` on screen.
///
/// A surface that stores plain codes (`surface_is_srgb` false) takes the code over 255. A surface
/// that encodes for us (`surface_is_srgb` true) takes the light level of that code.
pub fn surface_value(linear: f64, surface_is_srgb: bool) -> f32 {
    let encoded = f64::from(to_code(linear)) / 255.0;
    (if surface_is_srgb {
        srgb_decode(encoded)
    } else {
        encoded
    }) as f32
}

/// Three 8-bit codes packed into one BGRA pixel (blue lowest, alpha fully opaque).
pub fn pack_bgra(code: [u8; 3]) -> u32 {
    let [r, g, b] = code.map(u32::from);
    b | (g << 8) | (r << 16) | (0xFF << 24)
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

    #[test]
    fn srgb_encode_matches_known_points() {
        // floating point leaves 1.0 a hair short; only the rounding to a code matters
        assert_eq!(srgb_encode(0.0), 0.0);
        assert!((srgb_encode(1.0) - 1.0).abs() < 1e-15);
        assert!((srgb_encode(0.5) - 0.735_356_983_052_449_5).abs() < 1e-12);
        assert!((srgb_encode(0.0031308) - 0.040_449_936).abs() < 1e-9);
    }

    #[test]
    fn srgb_decode_undoes_encode() {
        for i in 0..=1000 {
            let x = f64::from(i) / 1000.0;
            assert!((srgb_decode(srgb_encode(x)) - x).abs() < 1e-12, "{x}");
        }
    }

    #[test]
    fn srgb_functions_clamp_out_of_range_input() {
        assert_eq!(srgb_encode(-1.0), 0.0);
        assert_eq!(srgb_encode(7.0), srgb_encode(1.0));
        assert_eq!(srgb_decode(-1.0), 0.0);
        assert_eq!(srgb_decode(7.0), srgb_decode(1.0));
    }

    #[test]
    fn to_code_has_the_right_ends_and_middle() {
        assert_eq!(to_code(0.0), 0);
        assert_eq!(to_code(1.0), 255);
        assert_eq!(to_code(-0.5), 0);
        assert_eq!(to_code(2.0), 255);
        assert_eq!(to_code(0.5), 188);
        assert_eq!(to_code(f64::NAN), 0);
    }

    #[test]
    fn to_code_never_goes_down_as_light_goes_up() {
        let mut last = 0;
        for i in 0..=100_000 {
            let code = to_code(f64::from(i) / 100_000.0);
            assert!(code >= last, "at {i}");
            last = code;
        }
    }

    #[test]
    fn every_code_survives_the_trip_through_light_and_back() {
        for k in 0..=255u8 {
            let light = srgb_decode(f64::from(k) / 255.0);
            assert_eq!(to_code(light), k, "code {k}");
        }
    }

    #[test]
    fn dark_colors_get_many_distinct_codes() {
        // The old software path rounded light to 8 bits first, so everything below
        // 1/255 of full light collapsed onto the codes 0 and 13.
        let mut seen = [false; 256];
        for i in 0..=1000 {
            seen[usize::from(to_code(f64::from(i) / 1000.0 / 255.0))] = true;
        }
        let distinct = seen.iter().filter(|&&s| s).count();
        assert_eq!(distinct, 14, "codes 0 to 13");
        assert_eq!(to_code(1.0 / 255.0), 13);
    }

    #[test]
    fn surface_value_puts_exactly_the_wanted_code_on_either_kind_of_surface() {
        for k in 0..=255u8 {
            let light = srgb_decode(f64::from(k) / 255.0);

            // a surface that stores codes: the card rounds value * 255
            let plain = f64::from(surface_value(light, false));
            assert_eq!((plain * 255.0).round() as u8, k, "plain, code {k}");

            // a surface that encodes for us: the card encodes, then rounds
            let encoded = srgb_encode(f64::from(surface_value(light, true)));
            assert_eq!((encoded * 255.0).round() as u8, k, "sRGB, code {k}");
        }
    }

    #[test]
    fn pack_bgra_puts_blue_lowest_and_alpha_on_top() {
        assert_eq!(pack_bgra([1, 2, 3]), 0xFF01_0203);
        assert_eq!(pack_bgra([0, 0, 0]), 0xFF00_0000);
        assert_eq!(pack_bgra([255, 255, 255]), 0xFFFF_FFFF);
    }
}
