//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `premultiply`
//!
//! The header `include/lib.h` declares no namespace-renaming macros, so the
//! linker symbol equals the source-level name.

#![allow(non_camel_case_types)]

use std::os::raw::c_int;

/// Mirrors `cp_pixel_t` from `include/lib.h` (4 bytes, no padding).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Mirrors `cp_image_t` from `include/lib.h`.
#[repr(C)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

/// Size of `cp_pixel_t`, i.e. `sizeof(cp_pixel_t)` in the C source.
const PIXEL_SIZE: c_int = 4;

/// Alpha-premultiply every pixel of `img` in place.
///
/// Faithful translation of `premultiply` in `c_src/src/lib.c`, including its
/// quirks, which are deliberately preserved:
///
/// * `int stride = w * sizeof(cp_pixel_t);` promotes `w` to `size_t`, performs
///   the multiply in 64-bit unsigned arithmetic and then truncates back to
///   `int`. Truncation to 32 bits makes this indistinguishable from a wrapping
///   32-bit multiply, which is what is used here.
/// * The loop bound is `(int)stride * h`, an `int` multiply, reproduced with
///   wrapping semantics.
/// * The iteration walks raw bytes and covers `stride * h` bytes, so a negative
///   or zero bound performs no work at all.
/// * The alpha channel (`data[i + 3]`) is read but never written back, so alpha
///   is left untouched. This is not corrected.
/// * The float pipeline is single precision throughout: divide by `255.0f`,
///   multiply by the alpha factor, then scale by `255.0f` and truncate toward
///   zero on the way back to a byte.
///
/// # Safety
///
/// `img` must point to a valid `cp_image_t` whose `pix` buffer holds at least
/// `w * h` pixels, exactly as the C function requires. A null or undersized
/// pointer faults here just as it does in C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn premultiply(img: *mut cp_image_t) {
    let w: c_int = (*img).w;
    let h: c_int = (*img).h;
    // int stride = w * sizeof(cp_pixel_t);  -> 64-bit multiply truncated to int
    let stride: c_int = w.wrapping_mul(PIXEL_SIZE);
    let data: *mut u8 = (*img).pix as *mut u8;

    // for (int i = 0; i < (int)stride * h; i += sizeof(cp_pixel_t))
    let end: c_int = stride.wrapping_mul(h);
    let mut i: c_int = 0;
    while i < end {
        let base = i as isize;

        let a = f32::from(*data.offset(base + 3)) / 255.0f32;
        let mut r = f32::from(*data.offset(base)) / 255.0f32;
        let mut g = f32::from(*data.offset(base + 1)) / 255.0f32;
        let mut b = f32::from(*data.offset(base + 2)) / 255.0f32;

        r *= a;
        g *= a;
        b *= a;

        // (uint8_t)(x * 255.0f): truncate toward zero, keep the low byte.
        *data.offset(base) = (r * 255.0f32) as i32 as u8;
        *data.offset(base + 1) = (g * 255.0f32) as i32 as u8;
        *data.offset(base + 2) = (b * 255.0f32) as i32 as u8;
        // data[i + 3] (alpha) is intentionally left as-is, matching the C.

        i = i.wrapping_add(PIXEL_SIZE);
    }
}
