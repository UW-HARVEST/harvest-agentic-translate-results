//! Rust translation of `c_src/src/lib.c`.
//!
//! Provides `tfm`, which walks a packed array of 3-float structure-tensor
//! entries (`dx2`, `dy2`, `dxy`) and writes a 2-float eigenvector-ish result
//! per entry.
//!
//! The floating point expressions below are transcribed verbatim from the C,
//! including association order, so that the emitted `f32` operations match the
//! original bit for bit.

use std::ffi::c_int;

/// Reproduces the C expression `((0) > (sqd)) ? (0) : (sqd)`.
///
/// In C the integer `0` is converted to `float` for both the comparison and
/// the result type. Note that this is *not* `f32::max`: when `sqd` is NaN the
/// comparison is false and the NaN is returned, which is the behavior the C
/// code has and which must be preserved.
#[inline]
fn clamp_low_zero(sqd: f32) -> f32 {
    if 0.0f32 > sqd { 0.0f32 } else { sqd }
}

/// Computes `sqd` exactly as written in the C source.
///
/// `(dy2 * dy2) - (2.0f * dx2 * dy2) + (dx2 * dx2) + (4.0f * dxy * dxy)`
/// with C's left-to-right association for `-`/`+` and for the `*` chains.
#[inline]
fn sqd_of(dx2: f32, dy2: f32, dxy: f32) -> f32 {
    ((dy2 * dy2) - ((2.0f32 * dx2) * dy2)) + (dx2 * dx2) + ((4.0f32 * dxy) * dxy)
}

/// Computes `lambda` as written in the C source:
/// `0.5f * (dy2 + dx2 + sqrtf(max(0, sqd)))`.
#[inline]
fn lambda_of(dx2: f32, dy2: f32, sqd: f32) -> f32 {
    0.5f32 * ((dy2 + dx2) + clamp_low_zero(sqd).sqrt())
}

/// Safe core of the transform, operating on one entry at a time.
///
/// `src_chunk` is the 3-float input entry, `dest_chunk` the 2-float output.
fn tfm_one(dest_chunk: &mut [f32], src_chunk: &[f32]) {
    if src_chunk[0] < src_chunk[1] {
        let dx2 = src_chunk[0];
        let dy2 = src_chunk[1];
        let dxy = src_chunk[2];
        let sqd = sqd_of(dx2, dy2, dxy);
        let lambda = lambda_of(dx2, dy2, sqd);
        dest_chunk[0] = dx2 - lambda;
        dest_chunk[1] = dxy;
    } else {
        // The C swaps which input slot feeds dy2/dx2 in this branch.
        let dy2 = src_chunk[0];
        let dx2 = src_chunk[1];
        let dxy = src_chunk[2];
        let sqd = sqd_of(dx2, dy2, dxy);
        let lambda = lambda_of(dx2, dy2, sqd);
        dest_chunk[0] = dxy;
        dest_chunk[1] = dx2 - lambda;
    }
}

/// C entry point: `void tfm(float *dest, const float *src, int count);`
///
/// # Safety
///
/// `src` must point to at least `count * 3` readable `f32`s and `dest` to at
/// least `count * 2` writable `f32`s. As in the C, a non-positive `count` does
/// nothing and the pointers are never dereferenced.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tfm(dest: *mut f32, src: *const f32, count: c_int) {
    // Matches the C loop guard `i < count`: nothing to do for count <= 0, and
    // in that case the pointers are not touched at all.
    if count <= 0 {
        return;
    }
    let n = count as usize;

    let src_slice = unsafe { std::slice::from_raw_parts(src, n * 3) };
    let dest_slice = unsafe { std::slice::from_raw_parts_mut(dest, n * 2) };

    for (dest_chunk, src_chunk) in dest_slice.chunks_mut(2).zip(src_slice.chunks(3)) {
        tfm_one(dest_chunk, src_chunk);
    }
}
