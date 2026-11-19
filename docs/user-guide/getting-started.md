# Getting started

VirtualFRC is a small test program. It flips between two colors on every screen refresh so you can see whether your monitor makes you perceive a color in between. It is an early prototype: it shows the idea, it does not improve the colors of anything else on your screen.

## How do I start it?

1. Install [Rust](https://www.rust-lang.org/tools/install) if you do not have it.
2. Open a terminal in the project folder and run `cargo run --release`.
3. A window opens. The title bar shows the measured frame rate, the drawing method, the view and the colors.

If you have a monitor of 144 Hz or more, make sure it is set to its highest refresh rate in your system's display settings first. Turn off variable refresh (G-Sync or FreeSync) for this program, because it spoils the effect.

## What should I try?

1. Start in the **side-by-side** view. The left half flips between color A and color B. The right half is their plain average. From a normal distance, do the halves look the same?
2. Press **Space** to choose which color you are editing, then use the arrow keys (and **E** / **C** for blue) to move the two colors closer together. The closer they are, the less flicker you see.
3. Press **1**, **2** or **3** to change the flip pattern (equal time, or color A twice or three times as long as B). Press **M** for the other views.
4. Press **B** to compare drawing on the graphics card with drawing by the processor. They should look the same.
5. Press **V** to turn vsync off and on. Watch the frame rate in the title bar.

The full list of keys is in the [README](../../README.md#keys). **R** puts both colors back to their starting values and **Esc** or **Q** quits.

## Where is my work kept?

Nowhere: the program saves no files. Colors go back to their defaults each time you start it.

## What if something goes wrong?

- **The window does not open, or the program stops at once.** It needs a graphics card with Vulkan, Metal or DirectX 12. Update your graphics driver and try again.
- **The frame rate in the title bar is lower than your monitor's refresh rate.** The effect will not work properly, because every refresh must show the right color. Close other programs, and check the monitor is at its top refresh rate.
- **You see flicker.** That is expected when the two colors are far apart or the refresh rate is low. Move the colors closer together. If it bothers you, stop using the program.
- **The two halves look different.** Your monitor may react differently to going up and down in brightness, or may apply its own dithering. This is a known limit of the technique, see the [known issues](../technical/known-issues.md).
