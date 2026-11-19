# Architecture

How the code is organised, for a developer who is new to it. It follows [the conventions](../../CONVENTIONS.md): one reason to change per file, pure logic apart from the machine, one boundary to the outside world.

## Today

The whole program is one file, `src/main.rs` (about 1,100 lines), plus `Cargo.toml`. In order, it holds:

| Section | What it does |
|---|---|
| `DisplayMode`, `Backend` | The three views and the two drawing methods |
| `Metrics`, `AppState` | Frame counter, colors, flip pattern and the title text |
| `pack_bgra`, `fill_*` | The software path: fill a CPU buffer with rayon |
| `Uniforms`, `GpuState` | The wgpu setup, both render pipelines, resize and the two render functions |
| `GPU_SHADER`, `SOFT_SHADER` | The WGSL shaders |
| `choose_present_mode`, `main` | Present-mode choice, the window, the key handling and the frame loop |

Pure rules (the flip pattern, the average, the packing of colors) are mixed in with window and GPU code, and a new view touches four places in the same file. That is the first thing to fix, as its own change (see the roadmap).

## Where it is going

```
  window + keys          thin: reads keys, shows the title, calls the rest
        │
  frc-core               pure: flip patterns, dither thresholds, color maths; no window, no GPU
        │
  render                 the only code that talks to wgpu: surfaces, pipelines, shaders
  platform               frame timing, fullscreen, refresh-rate and variable-refresh checks
```

Planned folders (none exist yet):

| Folder | Holds |
|---|---|
| `src/core/` | Pure maths with unit tests; later it can become its own crate |
| `src/render/` | wgpu state, pipelines and shaders |
| `src/platform/` | Operating-system specific timing and display queries |
| `src/app.rs` | Window, key handling and the frame loop |
| `tests/` | Mirrors `src/core/` |

## How to add a feature

Today, a new view needs a new `DisplayMode` entry, a branch in the GPU shader, a branch in `render_software`, a key in the handler and a name in the title. Five places for one idea means the structure is wrong; split the file first (see [the decision log](decisions.md)).
