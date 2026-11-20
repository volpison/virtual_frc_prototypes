# Roadmap

Where VirtualFRC is going, and where it is not.

## What this is

A prototype that explores software temporal dithering on high-refresh monitors: showing finer color steps than 8 bits by switching between neighbouring shades on every refresh. The aim is a small, fast, reusable core that could later sit under something larger, such as a post-process for one program or, much later, a system-wide layer.

## What it is not

- **Not a real 10-bit or 12-bit display.** It is a perceptual trick. It cannot be used for color-critical work.
- **Not a system-wide driver, yet.** A program can only control its own window. Doing this for the whole desktop (a virtual display, a compositor hook) is a separate, much larger project, to be decided after the core is proven. See [the decision log](docs/technical/decisions.md).
- **Not a calibration or measuring tool.** It may log timing, but it does not replace a colorimeter.
- **Not for variable-refresh (G-Sync / FreeSync) use.** Irregular frame times break the averaging. The program should ask for it to be turned off.
- **Not a video or game player.** It draws test patterns and, later, dithers images given to it.
- **Saves no files.** There is no document format to keep compatible.

## Now

Step 4: fixing the color pipeline (owning the rounding to 8 bits, flipping neighbouring codes). The list of [known issues](docs/technical/known-issues.md) is the backlog behind the steps below.

## Next

Each step is its own pull request. A step that only moves code changes no behaviour.

0. ✅ Set the project up to the house rules.
1. ✅ Format `src/main.rs` with rustfmt (pure formatting).
2. ✅ Split `src/main.rs` into modules by responsibility (pure move, no behaviour change).
3. ✅ Pull the flip-pattern and color maths into `src/frc/`, which knows nothing about windows or the GPU, with unit tests.
4. Fix the color pipeline: own the rounding to 8 bits, and make the demo flip neighbouring codes instead of two distant colors.
5. Pace the program by the monitor: advance the pattern once per frame actually shown, stop the busy loop, log frame-time spread and missed frames.
6. Replace the duty-cycle and checkerboard demos with a spatiotemporal blue-noise dither.
7. Fullscreen mode and a warning for variable refresh.
8. Reuse pre-computed frames for still images, and measure the CPU and power saving.

## Later

Ideas that are wanted but not scheduled:

- A per-program post-process that takes a higher-precision image and dithers it to 8 bits.
- A measurement mode (photodiode or high-speed camera) to check the real average on a given panel.
- Research into a system-wide layer: a virtual display on Windows, a compositor plugin on Linux.
- A Rust recipe for the project template, once this project has settled on its layout.
