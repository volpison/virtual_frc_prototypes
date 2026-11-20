# Architecture

How the code is organised, for a developer who is new to it. It follows [the conventions](../../CONVENTIONS.md): one reason to change per file, pure logic apart from the machine, one boundary to the outside world.

## Today

The program is split by responsibility (the split was a pure move: no behaviour changed).

| File | What it does |
|---|---|
| `src/main.rs` | The crate's documentation (the keys), the module list and the entry point |
| `src/app.rs` | The window, the key handling and the frame loop: each frame ends by asking for the next one, and showing it waits for the monitor |
| `src/state.rs` | What the program is doing: the two colors as 8-bit codes, flip pattern setting, view, drawing method, frame counts and the title text |
| `src/frc/` | The pure rules, with unit tests: `pattern.rs` (which color a frame shows), `timing.rs` (measuring missed refreshes and frame-time spread), `color.rs` (sRGB encoding, rounding to an 8-bit code, the light-space mix of two codes, the value handed to the card, packing a pixel), `gradient.rs` (the test ramp: plain and ordered-dither rows) |
| `src/software.rs` | The software path: filling a CPU buffer with the 8-bit codes of the colors or the gradient, one row per rayon task (no window, no GPU) |
| `src/render/mod.rs` | The wgpu state: surface, both pipelines, resize and the two render paths |
| `src/render/shaders.rs` | The two WGSL shaders |
| `src/render/present.rs` | Choosing the present mode (vsync or not) |

Dependencies point one way: `app` calls `state` and `render`; `render` uses `state` and `software`; `state` uses `frc`; `frc` and `software` use neither the window nor the GPU. `frc` uses no other crate at all, so its tests need nothing but the compiler.

The module is called `frc`, not `core`, because `core` is the name of Rust's own standard-library crate and a module of that name causes ambiguous paths.

## Where it is going

```
  window + keys          thin: reads keys, shows the title, calls the rest
        │
  frc-core               pure: flip patterns, dither thresholds, color maths; no window, no GPU
        │
  render                 the only code that talks to wgpu: surfaces, pipelines, shaders
  platform               frame timing, fullscreen, refresh-rate and variable-refresh checks
```

Still to come: the blue-noise dither (roadmap step 7), and `src/platform/` for operating-system specific timing and display queries. Later `frc` can become its own crate; its tests then move from the files into `tests/`.

## How to add a feature

A new view today needs: an entry in `DisplayMode` (`state.rs`), a branch in the GPU shader (`render/shaders.rs`), a branch in `render_software` (`render/mod.rs`) with its fill in `software.rs`, and a key in `app.rs`. Each is one place in one responsibility, but four files for one idea is still a sign that the view logic should move into `frc`; record such changes in [the decision log](decisions.md). A new rule about colors or patterns goes in `src/frc/` with its test next to it, and uses no window or GPU type.
