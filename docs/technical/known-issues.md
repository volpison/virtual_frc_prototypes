# Known issues

Found by reading the code in the first review (November 2026). Nothing here was run on a real monitor: the code was read, not executed. Names refer to functions in `src/` (see [Architecture](architecture.md)). Each item becomes a roadmap step when it is taken up.

## About the idea

1. **The demo flipped two distant colors. Fixed in step 4b.** The defaults differed by many 8-bit steps, so you saw flicker, not a third color. Colors are now 8-bit codes and start one step apart (127 and 128). With such a small gap the effect is subtle by nature: the gradient test pattern (key G, step 6) shows it better.
2. **Every pixel flips together.** In the full-window view all pixels change at once, the most visible kind of flicker. Real FRC gives neighbouring pixels different phases (spatiotemporal dithering, ideally blue noise).
3. **The flip patterns are few and clumpy.** `duty_a` gives three ratios, as runs (`AAB`). More levels need longer patterns spread as evenly as possible (`ABABA`, not `AAABB`). With N frames the pattern gives N−1 extra levels per 8-bit step: N=4 is about 2 more bits.

## Color and precision

4. **The color space was unclear. Fixed in step 4a.** The software path used to round light to 8 bits before the card encoded it, so linear code 1 showed as sRGB 12.7, code 2 as 21.7 and code 3 as 28.2, and dark colors banded. Now `frc::color::to_code` is the only place that rounds, a surface that stores plain codes is preferred, and the software texture matches the surface (see [the decision log](decisions.md)). Still open: colors in the program are linear-light values, not 8-bit codes, until step 4b.

## Timing

5. **The pattern was not tied to what the monitor shows. Mostly fixed in step 5.**
   - Fixed: the phase advanced per loop pass, also when a frame was not presented. It now advances only on a frame that was shown.
   - Fixed: frames were held back by a software timer made from the reported refresh rate (often 143.98, not 144). Frames are now drawn one after the other and showing one waits for the monitor; there is no timer.
   - Fixed: "FPS" counted loop passes. The title bar and a log line now count frames shown, and missed refreshes, using `frc::timing`.
   - Fixed: the refresh rate was read once. It is read again when the window moves.
   - Open: `Immediate` and `Mailbox` are not locked to vblank (frames can tear, drop or share a refresh). They stay as experiments; with vsync off the program now draws as fast as it can.
   - Open (step 8): in a window, the system compositor can retime frames. Fullscreen with direct flip is needed.
   - Open (step 8): variable refresh breaks the averaging and cannot be detected from here; the program should warn and the docs say to turn it off.
6. **The loop busy-waited. Fixed in step 5.** It now sleeps between refreshes (`ControlFlow::Wait` and a chain of redraw requests).

## Code

7. **The software path moves the wrong data.** A full CPU frame (about 33 MB at 4K) is filled and uploaded every refresh for what is one color. Per-pixel work belongs in a shader. The label "SIMD" was removed: it is a scalar loop under rayon, which the compiler may or may not vectorize.
8. **The bind-group layout is rebuilt on every resize** instead of being stored.
9. **Key handling.** Keys I, J, K, L and U permanently switch editing to color B; there is no key to lower B's blue except Space then C; letter keys use the typed character, so they move on non-QWERTY layouts. Use physical key codes.

## Display limits

10. **The panel is not symmetric.** Rise and fall times differ, so the average brightness is not the plain mean of the two colors. The monitor's own dithering can also beat against ours. Verify on each panel with a photodiode or a high-speed camera before trusting any result.
11. **The frame counter wrapped. Fixed in step 5.** It added with wrap-around on a `u32`, which came round after about 190 days at 240 frames a second, and a cycle that does not divide 2^32 (3 and 5 frames, for example) jumped once. The counter now starts over at 720,720, a multiple of every cycle from 2 to 16 frames, so the pattern carries on without a jump.
12. **The graphics driver may already dither** when reducing to 8 bits. Measure with it on and off.
