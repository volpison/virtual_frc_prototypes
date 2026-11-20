# Architecture

How the code is organised, for a developer who is new to it. It follows [the conventions](../../CONVENTIONS.md): one reason to change per file, pure logic apart from the machine, one boundary to the outside world.

## Today

The program is split by responsibility (the split was a pure move: no behaviour changed).

| File | What it does |
|---|---|
| `src/main.rs` | The crate's documentation (the keys), the module list and the entry point |
| `src/app.rs` | The window, the key handling and the frame loop |
| `src/state.rs` | What the program is doing: colors, flip pattern, view, drawing method, frame counts and the title text |
| `src/software.rs` | The software path: filling a CPU buffer (no window, no GPU) |
| `src/render/mod.rs` | The wgpu state: surface, both pipelines, resize and the two render paths |
| `src/render/shaders.rs` | The two WGSL shaders |
| `src/render/present.rs` | Choosing the present mode (vsync or not) |

Dependencies point one way: `app` calls `state` and `render`; `render` uses `state` and `software`; `state` and `software` use neither the window nor the GPU.

The pure rules (the flip pattern, the average of two colors) still sit in `state.rs` next to the title text. Pulling them into their own tested core is the next step on the roadmap.

## Where it is going

```
  window + keys          thin: reads keys, shows the title, calls the rest
        │
  frc-core               pure: flip patterns, dither thresholds, color maths; no window, no GPU
        │
  render                 the only code that talks to wgpu: surfaces, pipelines, shaders
  platform               frame timing, fullscreen, refresh-rate and variable-refresh checks
```

Still to come: `src/core/` for the pure maths (patterns, dither thresholds, color conversion) with unit tests in `tests/`, and `src/platform/` for operating-system specific timing and display queries. Later the core can become its own crate.

## How to add a feature

A new view today needs: an entry in `DisplayMode` (`state.rs`), a branch in the GPU shader (`render/shaders.rs`), a branch in `render_software` (`render/mod.rs`) with its fill in `software.rs`, and a key in `app.rs`. Each is one place in one responsibility, but four files for one idea is still a sign that the pattern logic should move into the core; record such changes in [the decision log](decisions.md).
