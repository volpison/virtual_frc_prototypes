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

- `pattern.rs`: A is shown `duty` times per cycle; the long-run share of A is duty/(duty+1); the pattern repeats; no color is held longer than its share; frame 0 shows A; a very large frame counter still works.
- `color.rs`: choosing A or B; the average of two colors is symmetric, lies between them, and black with white gives grey; sRGB encoding and decoding agree with known values and undo each other; every one of the 256 codes survives the trip through light and back; rounding never goes down as light goes up; dark colors get their own codes; the value handed to the card puts exactly the wanted code on screen for both kinds of surface; a pixel is packed in the right byte order.

Still without tests, because they are tied to the window or the GPU: `state.rs`, `software.rs`, `render/` and `app.rs`. Planned for the pure core as it grows: the dither thresholds are evenly spread over a cycle.

Since `frc` uses no other crate, its tests also run with plain `rustc --test` and no download.

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
