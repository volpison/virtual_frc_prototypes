# Known issues

Found by reading the code in the first review (November 2026). Nothing here was run on a real monitor: the code was read, not executed. Names refer to functions in `src/` (see [Architecture](architecture.md)). Each item becomes a roadmap step when it is taken up.

## About the idea

1. **The demo flips two distant colors.** The defaults differ by many 8-bit steps, so you see flicker, not a third color. FRC is useful between neighbouring codes (127 and 128), where the difference is almost invisible. The demo should default to neighbouring codes.
2. **Every pixel flips together.** In the full-window view all pixels change at once, the most visible kind of flicker. Real FRC gives neighbouring pixels different phases (spatiotemporal dithering, ideally blue noise).
3. **The flip patterns are few and clumpy.** `duty_a` gives three ratios, as runs (`AAB`). More levels need longer patterns spread as evenly as possible (`ABABA`, not `AAABB`). With N frames the pattern gives N−1 extra levels per 8-bit step: N=4 is about 2 more bits.

## Color and precision

4. **The color space is unclear.** The GPU path prefers an sRGB surface, so its values are linear light (right for averaging). If none exists it falls back to the first format, and the average is then wrong. The software path stores those same values in a `Bgra8Unorm` texture, so it rounds in linear space: linear code 1 shows as sRGB 12.7, code 2 as 21.7, code 3 as 28.2. Dark colors band badly. The fix is to own the rounding: use a non-sRGB surface and encode and dither in the shader.

## Timing

5. **The pattern is not tied to what the monitor shows.**
   - The phase advances per loop pass, also when a frame was not presented (`Timeout`, lost surface).
   - Frames are held back by a software timer made from the reported refresh rate, which is often 143.98 and not 144.
   - `Immediate` and `Mailbox` are not locked to vblank: frames can tear, drop or share a refresh.
   - "FPS" counts loop passes, not frames shown. Missed frames and frame-time spread are what matter.
   - In a window, the system compositor can retime frames. Fullscreen with direct flip is needed.
   - Variable refresh breaks the averaging. The refresh rate is read once, so moving the window to another monitor leaves it stale.
6. **The loop busy-waits.** `ControlFlow::Poll` keeps a core at 100% although `Fifo` already blocks. Request a redraw instead.

## Code

7. **The software path moves the wrong data.** A full CPU frame (about 33 MB at 4K) is filled and uploaded every refresh for what is one color. Per-pixel work belongs in a shader. It is also named "SIMD" but is a scalar loop under rayon.
8. **The bind-group layout is rebuilt on every resize** instead of being stored.
9. **Key handling.** Keys I, J, K, L and U permanently switch editing to color B; there is no key to lower B's blue except Space then C; letter keys use the typed character, so they move on non-QWERTY layouts. Use physical key codes.

## Display limits

10. **The panel is not symmetric.** Rise and fall times differ, so the average brightness is not the plain mean of the two colors. The monitor's own dithering can also beat against ours. Verify on each panel with a photodiode or a high-speed camera before trusting any result.
11. **The graphics driver may already dither** when reducing to 8 bits. Measure with it on and off.
