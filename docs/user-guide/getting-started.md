# Getting started

VirtualFRC is a small test program. It flips between two colors on every screen refresh so you can see whether your monitor makes you perceive a color in between. It is an early prototype: it shows the idea, it does not improve the colors of anything else on your screen.

## How do I start it?

1. Install [Rust](https://www.rust-lang.org/tools/install) if you do not have it.
2. Open a terminal in the project folder and run `cargo run --release`.
3. A window opens. The title bar shows the measured frame rate, the drawing method, the view and the colors.

If you have a monitor of 144 Hz or more, make sure it is set to its highest refresh rate in your system's display settings first. Turn off variable refresh (G-Sync or FreeSync) for this program, because it spoils the effect.

## What should I try?

The two colors, A and B, are shown in the title bar as codes from 0 to 255 for red, green and blue (for example `A 127,127,127  B 128,128,128`). They start one step apart, which is the case this technique is for.

1. Start in the **side-by-side** view. The left half flips between A and B on every refresh. The right half is the closest plain color to what your eye should see from that flipping. One step apart, the flicker is too faint to see, and both halves should look the same.
2. Press **Space** to choose which color you are editing, then use the arrow keys (and **E** / **C** for blue) to move it. One press moves it by 1; hold **Shift** to move it by 10. Move A and B further apart and watch the flicker grow: the gap at which you start to see it is what your monitor and your eyes allow.
3. Press **1**, **2** or **3** to change the flip pattern (equal time, or color A twice or three times as long as B). The right half follows, so it stays the matching average. Press **M** for the other views.
4. Press **B** to compare drawing on the graphics card with drawing by the processor. They should look the same, dark colors included.
5. Watch the title bar: the frame rate should match your monitor's refresh rate, and `missed:` should stay at 0. Every missed refresh is one frame that was not shown in time, and it breaks the flipping. Press **V** to turn vsync off: the program then draws as fast as it can, the flipping runs faster than the monitor, and the effect is lost. That is useful only to see the difference.

The full list of keys is in the [README](../../README.md#keys). **R** puts both colors back to their starting values and **Esc** or **Q** quits.

## Where is my work kept?

Nowhere: the program saves no files. Colors go back to their defaults each time you start it.

## What if something goes wrong?

- **The window does not open, or the program stops at once.** It needs a graphics card with Vulkan, Metal or DirectX 12. Update your graphics driver and try again.
- **The frame rate in the title bar is lower than your monitor's refresh rate, or `missed:` keeps rising.** The effect will not work properly, because every refresh must show the right color. Close other programs, check the monitor is at its top refresh rate, and turn off variable refresh. The terminal also prints a `Frame timing` warning with the numbers; copy it into a bug report.
- **You see flicker.** That is expected when the two colors are far apart or the refresh rate is low. Move the colors closer together. If it bothers you, stop using the program.
- **The two halves look different.** Your monitor may react differently to going up and down in brightness, or may apply its own dithering. This is a known limit of the technique, see the [known issues](../technical/known-issues.md).
