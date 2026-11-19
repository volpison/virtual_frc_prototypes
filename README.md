# Virtual Frame Rate Control Prototypes (VirtualFRC)

Software temporal dithering / virtual FRC prototype.

Rapidly alternates between two solid colors on a high-refresh-rate monitor so the human visual system integrates them into an intermediate perceived color. This is the same principle used by hardware Frame Rate Control (FRC) in many LCD panels, applied deliberately from the application side.

## Goals

- Demonstrate that high-Hz monitors can be used to synthesize additional color levels via temporal averaging.
- Measure achieved frame rate vs monitor native refresh rate.
- Provide both a GPU path (primary) and a future software/SIMD path.
- Serve as a foundation for more advanced experiments (duty-cycle patterns, multi-frame FRC sequences, gradient tests, etc.).

## Requirements

- Rust 1.75+ (1.80+ recommended)
- A GPU with Vulkan, Metal, or DX12 support
- Ideally a monitor running at 144 Hz or higher (240 Hz+ recommended for cleaner results)

## Build & Run

```bash
cargo run --release
```

On Windows / Linux / macOS the same command works.

## Controls

| Key              | Action                                      |
|------------------|---------------------------------------------|
| Arrow keys / WASD| Adjust currently selected color (R/G)       |
| E / C            | Adjust Blue channel of selected color       |
| Space            | Toggle which color is being edited (A ↔ B)  |
| I J K L U M      | Directly edit Color B                       |
| 1 / 2 / 3        | Set duty cycle (1:1, 2:1, 3:1)              |
| R                | Reset both colors to defaults               |
| Esc or Q         | Quit                                        |

The window title continuously shows:

- Current measured FPS
- Detected monitor refresh rate
- Which color is being edited
- Current duty cycle
- RGB values of both colors

## Technical Notes

- Uses `wgpu` with `PresentMode::Immediate` (or `Mailbox`) whenever the driver supports it, so the application is not locked to vsync.
- A pure clear + present is intentionally used so the GPU path has almost zero overhead.
- Duty cycle > 1 lets you create finer intermediate levels (classic FRC technique).
- The effect is most convincing when the two colors are close to each other and the refresh rate is high.

## Limitations (honest)

This is a perceptual technique, not true higher bit-depth. It cannot replace a native 10-bit or 12-bit panel for color-critical work. LCD response time, the monitor’s own internal FRC/dithering, and individual sensitivity to temporal artifacts all affect the result.

## Future directions

- Software SIMD presenter (AVX2 / AVX-512 / NEON)
- Configurable multi-frame FRC patterns
- Side-by-side comparison (static average vs temporal)
- Fullscreen exclusive mode
- Gradient test patterns
- Export of measured timing data

## License

MIT OR Apache-2.0
