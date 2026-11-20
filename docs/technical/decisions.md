# Decision log

One short paragraph per decision, newest last: what was decided, why, and what was left out. Future you forgets the why.

Template for a new entry:

```
## Short title (YYYY-MM)

Decided: …
Why: …
Left out: … (and when to revisit it, if ever)
```

The first entries below came from Fabulator and apply to every project started from this template. Change them if a project decides otherwise, and say so in a new entry.

## Docs live in the repository (2026-11)

Decided: documentation is Markdown under `docs/`, next to the code, with a short README that links to it.
Why: it changes in the same pull request as the code, it is reviewed with it, and a link check can keep it healthy. A wiki drifts away from the code.
Left out: a hosted documentation site. Add one only when readers need it.

## Every change is a pull request from a branch (2026-11)

Decided: nothing is pushed to `main`; each change is a pull request with one topic and a "Check before merging" list.
Why: small, checked changes are easy to review and easy to undo, and CI judges them before they land.
Left out: direct pushes, even for tiny fixes.

## A new kind of document keeps its data under one key (2026-11)

Decided: when a project learns a new kind of document, all of its data lives under one key named after it (`"game": { ... }`).
Why: kinds do not mix their lists with the shared parts, and an older version that does not know the kind leaves the key untouched.
Left out: moving every kind into a common `world` and `document` layout; it would touch everything without adding a feature, and a minimum reader version makes it safe to do later as its own step.

## Releases are one command and a tag check (2026-11)

Decided: the version is changed everywhere by one command (`node scripts/version.mjs set`), and the release workflow refuses a tag that does not match. It creates a draft release; a person publishes it.
Why: Fabulator keeps its version in five files; they drifted before this existed, and an installer was once named from the wrong one. A draft keeps the last decision with a person.
Left out: publishing automatically, and any particular build: each project adds its own build steps to the workflow.

## Pure moves and behaviour changes are separate pull requests (2026-11)

Decided: a refactor that changes no behaviour is its own pull request.
Why: a pure move is easy to check (the tests do not change); a mixed one hides mistakes.
Left out: "while I am here" fixes in a move.

## Recipes are copy-in, not built in (2026-11)

Decided: stack-specific material (Tauri + Svelte, Python desktop) lives in `docs/recipes/` and inactive workflow files in `recipes/`; the template's active CI only checks docs.
Why: the template stays language-neutral and green on its own, and a project copies only what its stack needs. The older projects' code was not carried over, only their habits.
Left out: a full starter app per stack. Add one only when two projects have needed the same scaffolding.

## Rust with wgpu and winit for the prototype (2026-11)

Decided: the prototype stays in Rust, drawing with wgpu in a winit window, as it was received.
Why: it is already built that way, one code base reaches Windows, Linux and macOS, and a Rust core can later be reused in a library or a native layer.
Left out: a separate rewrite in C++ or C#. Revisit if a system-level driver needs it; a driver is its own project.

## Format first, then move, then change (2026-11)

Decided: the first pull requests are rustfmt formatting only, then project setup, then the split of `src/main.rs` into modules, and only after those any behaviour change.
Why: [the conventions](../../CONVENTIONS.md#4-workflow-small-checked-boring) keep pure moves apart from behaviour changes, and nothing here could be compiled in the author's environment, so each step has to stay small enough to review by eye.
Left out: fixing the [known issues](known-issues.md) in the same pull requests.

## Flip neighbouring codes with spatiotemporal noise (2026-11)

Decided: the direction is dithering between neighbouring 8-bit codes, with a different phase per pixel, instead of flipping two chosen colors across the whole window.
Why: that is what real FRC does, and it is where flicker stays below what the eye notices. The present demo is a proof of the principle only.
Left out: keeping the two-color flip as the main feature. It may stay as a test view.

## The program owns the rounding to 8 bits (2026-11)

Decided: render to a non-sRGB surface and do the color encoding and the dithering in the shader.
Why: FRC needs to know exactly which two integer codes reach the monitor; an sRGB surface rounds where we cannot see or control it, and the software path currently rounds in linear space.
Left out: relying on the driver's own dithering.

## Pace by the monitor, not by a timer (2026-11)

Decided: vsync (`Fifo`) is the default, the pattern advances once per frame actually shown, and the loop is driven by redraw requests, not polling. `Immediate` and `Mailbox` stay as experiments.
Why: a pattern that skips or repeats a refresh gives the wrong average. A busy loop also wastes the power this project wants to save.
Left out: a software timer built from the reported refresh rate.

## A system-wide layer is a later, separate project (2026-11)

Decided: this repository stays a prototype and a reusable core. A virtual display driver, a compositor hook or a desktop capture layer is investigated only after the core works on a real monitor.
Why: a program can only control its own window. A system-wide layer adds a frame of latency, and games in exclusive fullscreen and protected video may bypass it.
Left out: starting the driver now.

## Licence is MIT only (2026-11)

Decided: the project is under the MIT licence alone. `Cargo.toml` said "MIT OR Apache-2.0" when the project was received; the owner does not want the Apache licence, so it was dropped.
Why: MIT is short and permissive, and nothing here needs the explicit patent grant that Apache adds.
Left out: dual licensing. Revisit if the core becomes a library that companies want a patent grant from.

## CI runs on Windows and Linux (2026-11)

Decided: the Rust jobs run on `windows-latest` and `ubuntu-latest`; macOS is not covered yet.
Why: Windows is where high-refresh gaming monitors are mostly used; Linux is cheap and catches portability mistakes.
Left out: macOS, until someone uses it there.

## Split by responsibility before any fix (2026-11)

Decided: `src/main.rs` was cut into `app`, `state`, `software` and `render` (with `shaders` and `present` inside it) in one pull request that moves code and changes nothing else. Only `mod` and `use` lines and `pub(crate)` visibility were added; the moved code was checked line by line against the original.
Why: the known issues all need changes inside these areas, and a fix is easy to review only once the area has its own file. Nothing could be compiled in the author's environment, so CI is the check that the move is complete.
Left out: moving the pattern maths out of `state.rs` into a core (the next step), and renaming anything.

## The pure rules live in `src/frc/`, tested in the same files (2026-11)

Decided: the flip pattern and the color maths were taken out of `state.rs` into `src/frc/` (`pattern.rs`, `color.rs`), which uses no other crate. Their tests sit beside them under `#[cfg(test)]`. `state.rs` now calls them.
Why: these are the rules the later steps will change most, and they can now be tested without a window, a GPU or a download. The extraction changes no behaviour: the old inline code and the new functions were compared on 12.7 million cases and agree on all. The module is not called `core`, because that clashes with Rust's own `core` crate.
Left out: a `tests/` folder (a binary crate has no library to import; revisit when `frc` becomes a library crate), and moving `pack_bgra` into `frc` (it belongs with the rounding to 8 bits, roadmap step 4).

## The program rounds to 8 bits, once, and picks the surface to match (2026-11)

Decided: `frc::color::to_code` (encode to sRGB, round to the nearest code) is the only place that rounds a color to 8 bits. The renderer prefers a surface that stores plain codes and hands it `code / 255`; if only an sRGB surface exists, it hands the light level of that code, so the same code comes out. The software path packs the same codes, and its texture is sRGB when the surface is, so the codes survive unchanged.
Why: FRC needs to know exactly which two neighbouring codes reach the monitor. Before, the GPU path left the rounding to the card and the software path rounded light before encoding it, so the two paths disagreed and dark colors banded.
Left out: a dither shader (roadmap step 6), and a 10-bit surface. The codes the GPU path shows can differ from before only by the card's own rounding at an exact halfway point.

## Colors stay linear-light values until step 4b (2026-11)

Decided: step 4a does not change what a color in the program means (linear light, 0.0 to 1.0, edited in steps of 0.02). Editing in 8-bit codes with neighbouring defaults is step 4b, its own pull request.
Why: 4a then changes the pipeline without changing what the demo looks like, apart from the banding fix, and 4b is the visible redesign that needs a real monitor and a person to judge.
Left out: converting the state to codes in the same pull request.

## Colors are 8-bit codes, and the plain side is the nearest code (2026-11)

Decided: the two colors are stored and edited as 8-bit codes (0 to 255 per channel), start one code apart (127 and 128), and move by 1 per key press, or 10 with Shift. The plain half of the side-by-side view shows `frc::color::mix` of the two codes: the mix is made in light, weighted by how often A is shown, and rounded to the nearest code.
Why: frame rate control is about neighbouring codes, so the demo should speak in codes. The eye adds light, so the mix is made in light, not in codes (half black and half white light is code 188, not 127). Weighting by the pattern fixes a mismatch: with 2:1 or 3:1 the flipping side averages to 2/3 or 3/4 of A, but the plain side always showed the 1:1 average.
Left out: showing the exact in-between color on the plain side (an 8-bit screen cannot; a 10-bit surface could, later), a Shift-free way to cross the whole range (key repeat is still ignored), and a gradient test pattern, which would show the effect better than two flat halves.
