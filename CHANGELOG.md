# Changelog

What changed for users, newest first. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [Semantic Versioning](https://semver.org/). Write for the people who use the program, not for developers.

## [Unreleased]

### Changed
- Colors are now set as 8-bit codes from 0 to 255, shown in the title bar. The two colors start one step apart (127 and 128), which is the case frame rate control is for. Arrow keys move a code by 1, and by 10 while Shift is held.
- The plain half of the side-by-side view now shows the closest 8-bit color to the average of the two flipped colors. With codes one step apart that is one of the two codes: it cannot show the in-between color, which is the point of the comparison.
- Both drawing methods now put exactly the same 8-bit colors on screen: the program picks the codes itself instead of leaving the rounding to the graphics card.
- The list of keys in the README now matches the program: it adds M (view), B (drawing method), V (vsync) and the bracket keys, and no longer promises a key that does not exist.

### Fixed
- With the flip pattern 2:1 or 3:1, the plain half of the side-by-side view still showed the 1:1 average. It now matches the pattern.
- The software drawing method (key B) now shows dark colors with the same fine steps as the GPU method. Before, dark colors were visibly banded in the software method only.

<!--
When releasing: rename "Unreleased" to "[x.y.z] - YYYY-MM-DD", add a fresh Unreleased above it.
Sections: Added, Changed, Fixed, Removed. Delete the empty ones.
-->
