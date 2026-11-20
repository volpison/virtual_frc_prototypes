# Changelog

What changed for users, newest first. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [Semantic Versioning](https://semver.org/). Write for the people who use the program, not for developers.

## [Unreleased]

### Fixed
- The software drawing method (key B) now shows dark colors with the same fine steps as the GPU method. Before, dark colors were visibly banded in the software method only.

### Changed
- Both drawing methods now put exactly the same 8-bit colors on screen: the program picks the codes itself instead of leaving the rounding to the graphics card.
- The list of keys in the README now matches the program: it adds M (view), B (drawing method), V (vsync) and the bracket keys, and no longer promises a key that does not exist.

<!--
When releasing: rename "Unreleased" to "[x.y.z] - YYYY-MM-DD", add a fresh Unreleased above it.
Sections: Added, Changed, Fixed, Removed. Delete the empty ones.
-->
