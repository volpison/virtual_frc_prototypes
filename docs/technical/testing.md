# Tests

## Running the tests

```
cargo fmt --check                    # formatting
cargo clippy --all-targets           # lints
cargo test                           # unit tests (there are none yet)
cargo build --release                # the real build
node --test "scripts/*.test.mjs"     # tests of the docs check and version script
node scripts/check-docs.mjs          # links, heading links and the docs index
```

## What the tests cover

Nothing in the Rust code is tested yet: the pure rules still sit in `src/state.rs` next to display code. Once the pure core exists ([roadmap](../../ROADMAP.md)), it gets the tests, mirrored under `tests/`:

- the flip pattern over a whole cycle averages to the requested level;
- no pattern holds one value longer than its allowed run;
- the dither threshold values are evenly spread over a cycle;
- rounding to 8 bits gives exactly the intended neighbouring codes.

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
