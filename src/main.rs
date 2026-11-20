//! Virtual Frame Rate Control Prototypes (VirtualFRC)
//!
//! Display modes (M):
//!   0 – Side-by-side: Left = temporal, Right = static average
//!   1 – Full temporal
//!   2 – Checkerboard flicker-dither
//!
//! Backends (B):
//!   GPU     – pure GPU clear / shader path (default)
//!   Software – multi-threaded SIMD fill of a CPU buffer, then upload
//!
//! Controls:
//!   M          : cycle display mode
//!   B          : toggle GPU ↔ Software backend
//!   Arrow/WASD : adjust selected color (R/G)
//!   E / C      : adjust Blue
//!   Space      : toggle editing A ↔ B
//!   1 / 2 / 3  : duty cycle
//!   V          : Fifo ↔ Immediate
//!   [ / ]      : checkerboard cell size
//!   R          : reset colors
//!   Esc / Q    : quit

mod app;
mod frc;
mod render;
mod software;
mod state;

fn main() {
    app::run();
}
