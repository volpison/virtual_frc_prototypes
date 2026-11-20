# Tests

## Running the tests

```
cargo fmt --check                    # formatting
cargo clippy --all-targets           # lints
cargo test                           # unit tests of the rules in src/frc/
cargo build --release                # the real build
node --test "scripts/*.test.mjs"     # tests of the docs check and version script
node scripts/check-docs.mjs          # links, heading links and the docs index
```

## What the tests cover

The pure rules in `src/frc/` are tested, with the tests in the same files under `#[cfg(test)]` (the program is a binary crate, so a separate `tests/` folder cannot reach them yet; it will once `frc` is a library crate):

- `pattern.rs`: the frame counter starts over at a multiple of every cycle length, with no jump in the pattern; the share of A matches the frames that show it (and has the known values 1/2, 2/3, 3/4); A is shown `duty` times per cycle; the long-run share of A is duty/(duty+1); the pattern repeats; no color is held longer than its share; frame 0 shows A; a very large frame counter still works.
- `timing.rs`: a refresh counts as missed from 1.5 periods on; steady frames give a clean report each second; a skipped refresh is counted and flagged; a long pause is not counted as lost frames; the wrong average rate is flagged; the percentile.
- `color.rs`: the mix of two codes is made in light (half black and half white is code 188), gives back A or B at the ends, keeps equal colors, lies between the two and follows the share; sRGB encoding and decoding agree with known values and undo each other; every one of the 256 codes survives the trip through light and back; rounding never goes down as light goes up; dark colors get their own codes; the value handed to the card for a code puts exactly that code on screen for both kinds of surface; a pixel is packed in the right byte order.

- `gradient.rs`: the ramp hits both end codes; the plain half is the nearest code and steps in bands; the 16 thresholds of a 4 by 4 block are all different and four frames visit evenly spaced ones; a value that is exactly a code never flickers; a pixel only shows the code below or above; a block averages to the value on every frame (within 1/32 of a code) and one pixel over four frames within 1/4; the halves split at the middle row; channels ramp separately.

- `state.rs`: default colors one code apart, key steps and the 0 to 255 limits, the flipping side follows the pattern (also across the counter starting over), the plain side follows the share, the title shows the codes and the frame counts.

Still without tests, because they are tied to the window or the GPU: `software.rs`, `render/` and `app.rs`. 

Since `frc` and `state.rs` use no other crate, their tests also run with plain `rustc --test` and no download (compile a small file that includes `frc/mod.rs` and `state.rs` as modules).

The window, the GPU path and the look of the effect cannot be checked by a test. They are checked by the build, and by a person running the program on a real monitor.

## Checking by eye

On a monitor at 120 Hz or more: run `cargo run --release`, check the title bar shows the real refresh rate and a frame rate close to it, and compare the two halves of the side-by-side view. Note the monitor, the refresh rate, the graphics card and whether variable refresh was off.

## What CI does

`.github/workflows/ci.yml` runs on every push to `main` and every pull request:

- **docs**: the script tests and the docs check, on Linux.
- **rust**: format check, clippy, build and tests, on **Windows and Linux**. Windows is where most users run it; Linux catches path and portability mistakes. macOS is not covered yet.

CI cannot open a window or reach a GPU, so it proves the code builds, not that the effect works.

## Writing a test

- A new pure rule gets a test in the matching file.
- A bug fix starts with a test that fails for the bug.
- Compare paths with forward slashes; never assume the machine's separator.
