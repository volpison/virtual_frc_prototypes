# VirtualFRC Prototypes

Software temporal dithering ("virtual FRC") on high-refresh-rate monitors.

Many monitors reach more colors than their 8-bit input by flipping a pixel between two neighbouring shades very quickly (frame rate control, FRC). Your eye averages the flips into a shade in between. VirtualFRC tries the same trick from the program side, on a monitor fast enough to allow it.

**Status: a prototype.** It shows the principle and measures the frame rate. It does not yet give you finer colors in real use. See the [roadmap](ROADMAP.md) for where it is going and what it will never be.

## What you can do

- See two colors flipped on every refresh next to their plain average, and judge the match by eye.
- Switch between four views: side-by-side, full-window flip, a checkerboard and a gradient test pattern.
- Compare two ways of drawing: on the graphics card (GPU) or filled by the processor and uploaded (Software).
- Set the two colors as 8-bit codes (0 to 255), starting one step apart, and change the flip pattern (1:1, 2:1, 3:1) and vsync from the keyboard.
- Read the measured frame rate and the number of missed refreshes in the title bar. A missed refresh breaks the effect, so this number should stay at 0.

## Run it

You need [Rust](https://www.rust-lang.org/tools/install) 1.75 or newer and a graphics card with Vulkan, Metal or DirectX 12. A monitor at 144 Hz or more works best.

```
cargo run --release
```

The same command works on Windows, Linux and macOS.

## Keys

| Key | What it does |
|---|---|
| Arrow keys or W A S D | Change red and green of the selected color by 1 (hold Shift: by 10) |
| E / C | Raise / lower blue of the selected color (also with Shift) |
| Space | Switch between editing color A and color B |
| I K / J L / U | Change green / red / blue of color B directly (also with Shift) |
| M | Next view (side-by-side, full flip, checkerboard, gradient) |
| G | Gradient test pattern with a grey ramp (sets both colors) |
| B | Switch drawing method (GPU or Software) |
| 1 / 2 / 3 | Flip pattern 1:1, 2:1 or 3:1 |
| V | Vsync on or off (off draws as fast as possible, which loses the effect) |
| [ / ] | Smaller / larger checkerboard squares |
| R | Reset both colors |
| Esc or Q | Quit |

## Good to know

This is a perceptual trick, not a real 10-bit or 12-bit screen. Panel response, the monitor's own dithering and personal sensitivity to flicker all change what you see. Do not use it for color-critical work, and stop if you find the flicker uncomfortable.

## Learn more

- [Getting started](docs/user-guide/getting-started.md): the first steps and what to try
- [All documentation](docs/index.md)
- [Roadmap](ROADMAP.md) and [changelog](CHANGELOG.md)
- [Contributing](docs/contributing.md)

## Licence

MIT. See [LICENSE-MIT](LICENSE-MIT).
