//! Colors as three 8-bit codes (red, green, blue), each from 0 to 255, and the light they give.

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

/// The number to hand the graphics card so that it puts exactly `code` on screen.
///
/// A surface that stores plain codes (`surface_is_srgb` false) takes the code over 255. A surface
/// that encodes for us (`surface_is_srgb` true) takes the light level of that code.
pub fn code_value(code: u8, surface_is_srgb: bool) -> f32 {
    let encoded = f64::from(code) / 255.0;
    (if surface_is_srgb {
        srgb_decode(encoded)
    } else {
        encoded
    }) as f32
}

/// The color the eye should see when `a` is shown for a share `share_a` of the frames
/// (0.0 to 1.0) and `b` for the rest, as the nearest 8-bit codes.
///
/// The eye adds up light, so the mix is made in light, not in codes. With codes one step
/// apart the true result falls between two codes, and the nearest code is all an 8-bit
/// screen can show without flipping.
pub fn mix(a: [u8; 3], b: [u8; 3], share_a: f64) -> [u8; 3] {
    let share_a = share_a.clamp(0.0, 1.0);
    let light = |code: u8| srgb_decode(f64::from(code) / 255.0);
    let channel = |i: usize| to_code(light(a[i]) * share_a + light(b[i]) * (1.0 - share_a));
    [channel(0), channel(1), channel(2)]
}

/// Three 8-bit codes packed into one BGRA pixel (blue lowest, alpha fully opaque).
pub fn pack_bgra(code: [u8; 3]) -> u32 {
    let [r, g, b] = code.map(u32::from);
    b | (g << 8) | (r << 16) | (0xFF << 24)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn code_value_puts_exactly_the_wanted_code_on_either_kind_of_surface() {
        for k in 0..=255u8 {
            // a surface that stores codes: the card rounds value * 255
            let plain = f64::from(code_value(k, false));
            assert_eq!((plain * 255.0).round() as u8, k, "plain, code {k}");

            // a surface that encodes for us: the card encodes, then rounds
            let encoded = srgb_encode(f64::from(code_value(k, true)));
            assert_eq!((encoded * 255.0).round() as u8, k, "sRGB, code {k}");
        }
    }

    #[test]
    fn mix_at_the_ends_gives_back_a_or_b() {
        let (a, b) = ([10, 120, 250], [200, 5, 0]);
        assert_eq!(mix(a, b, 1.0), a);
        assert_eq!(mix(a, b, 0.0), b);
        assert_eq!(mix(a, b, 7.0), a, "share above 1 counts as 1");
        assert_eq!(mix(a, b, -3.0), b, "share below 0 counts as 0");
    }

    #[test]
    fn mix_of_two_equal_colors_is_that_color_for_any_share() {
        for k in 0..=255u8 {
            for share in [0.0, 0.25, 0.5, 2.0 / 3.0, 0.75, 1.0] {
                assert_eq!(mix([k, k, k], [k, k, k], share), [k, k, k], "{k} {share}");
            }
        }
    }

    #[test]
    fn mix_is_made_in_light_not_in_codes() {
        // Half black and half white light is code 188, not the middle code 127 or 128.
        assert_eq!(mix([0; 3], [255; 3], 0.5), [188; 3]);
    }

    #[test]
    fn mix_does_not_depend_on_the_order_at_half() {
        let (a, b) = ([10, 120, 250], [200, 5, 0]);
        assert_eq!(mix(a, b, 0.5), mix(b, a, 0.5));
    }

    #[test]
    fn mix_lies_between_the_two_colors_and_follows_the_share() {
        for (lo, hi) in [(0u8, 1u8), (127, 128), (0, 255), (30, 31), (200, 255)] {
            let mut last = lo;
            for step in 0..=20 {
                let share_hi = f64::from(step) / 20.0;
                let m = mix([lo; 3], [hi; 3], 1.0 - share_hi)[0];
                assert!(lo <= m && m <= hi, "{lo} {hi} {share_hi}");
                assert!(m >= last, "never goes back: {lo} {hi} {share_hi}");
                last = m;
            }
        }
    }

    #[test]
    fn pack_bgra_puts_blue_lowest_and_alpha_on_top() {
        assert_eq!(pack_bgra([1, 2, 3]), 0xFF01_0203);
        assert_eq!(pack_bgra([0, 0, 0]), 0xFF00_0000);
        assert_eq!(pack_bgra([255, 255, 255]), 0xFFFF_FFFF);
    }
}
