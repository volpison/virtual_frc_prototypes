//! The software drawing path: filling a CPU buffer with pixels. No window and no GPU here.

use rayon::prelude::*;

use crate::frc::color;

/// Pack a linear-light float color into a BGRA u32 that holds its 8-bit codes
/// (little-endian, matching common Windows / wgpu texture layouts).
///
/// The rounding to 8 bits happens here, in `frc::color::to_code`, so the software path
/// shows exactly the codes the GPU path shows.
#[inline(always)]
pub(crate) fn pack_bgra(c: [f32; 3]) -> u32 {
    let code = |v: f32| color::to_code(f64::from(v));
    color::pack_bgra([code(c[0]), code(c[1]), code(c[2])])
}

/// Fill a whole buffer with a solid color using multi-threaded stores.
/// On x86_64 we use wide stores where possible; otherwise scalar.
pub(crate) fn fill_solid(buf: &mut [u32], color: u32) {
    // Rayon parallel fill – each thread writes a contiguous slice.
    buf.par_chunks_mut(4096).for_each(|chunk| {
        // Scalar fill of the chunk (compiler + rayon already give good throughput).
        // For true AVX2 we could use _mm256_storeu_si256 in an unsafe block;
        // the parallel scalar version is already a large win over single-threaded.
        for p in chunk.iter_mut() {
            *p = color;
        }
    });
}

/// Multi-threaded checkerboard fill.
pub(crate) fn fill_checkerboard(
    buf: &mut [u32],
    width: u32,
    height: u32,
    color_a: u32,
    color_b: u32,
    cell: u32,
    phase: u32,
) {
    let cell = cell.max(1);
    let flip = (phase % 2) == 1;
    let w = width as usize;

    buf.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        let cy = (y as u32 / cell) as i32;
        for (x, pixel) in row.iter_mut().enumerate() {
            let cx = (x as u32 / cell) as i32;
            let mut checker = (cx + cy) & 1;
            if flip {
                checker = 1 - checker;
            }
            *pixel = if checker == 0 { color_a } else { color_b };
        }
    });
}

/// Side-by-side: left half temporal solid, right half static average.
pub(crate) fn fill_side_by_side(
    buf: &mut [u32],
    width: u32,
    _height: u32,
    temporal: u32,
    static_avg: u32,
) {
    let mid = (width / 2) as usize;
    let w = width as usize;

    buf.par_chunks_mut(w).for_each(|row| {
        for (x, pixel) in row.iter_mut().enumerate() {
            *pixel = if x < mid { temporal } else { static_avg };
        }
    });
}
