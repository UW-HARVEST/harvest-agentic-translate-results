//! Rust translation of `c_src/src/lib.c`.
//!
//! The C source exposes a single function, `premultiply`, which multiplies the
//! RGB channels of an RGBA image by its alpha channel in place. There are no
//! namespace-renaming preprocessor macros in `include/lib.h`, so the final
//! linker symbol is simply `premultiply`.

use std::ffi::c_int;

/// Mirrors `cp_pixel_t` from `include/lib.h`.
#[repr(C)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Mirrors `cp_image_t` from `include/lib.h`.
#[repr(C)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut CpPixel,
}

/// Size of `cp_pixel_t` in bytes, as used by the C `sizeof` expressions.
const PIXEL_SIZE: c_int = 4;

/// `void premultiply(cp_image_t *img);`
///
/// Faithful translation of the C loop, including its arithmetic quirks:
/// `stride` is computed as `w * sizeof(cp_pixel_t)` and stored in an `int`,
/// and the loop bound is `(int)stride * h`, so overflow behaviour and any
/// resulting early/late loop termination is reproduced via wrapping
/// multiplication rather than being "fixed".
#[unsafe(no_mangle)]
pub unsafe extern "C" fn premultiply(img: *mut CpImage) {
    let img = &mut *img;

    let w: c_int = img.w;
    let h: c_int = img.h;
    // int stride = w * sizeof(cp_pixel_t);
    let stride: c_int = w.wrapping_mul(PIXEL_SIZE);
    // uint8_t *data = (uint8_t *)img->pix;
    let data: *mut u8 = img.pix as *mut u8;

    // for (int i = 0; i < (int)stride * h; i += sizeof(cp_pixel_t))
    let limit: c_int = stride.wrapping_mul(h);
    let mut i: c_int = 0;
    while i < limit {
        let base = i as isize;

        let a = f32::from(*data.offset(base + 3)) / 255.0f32;
        let mut r = f32::from(*data.offset(base)) / 255.0f32;
        let mut g = f32::from(*data.offset(base + 1)) / 255.0f32;
        let mut b = f32::from(*data.offset(base + 2)) / 255.0f32;

        r *= a;
        g *= a;
        b *= a;

        *data.offset(base) = (r * 255.0f32) as u8;
        *data.offset(base + 1) = (g * 255.0f32) as u8;
        *data.offset(base + 2) = (b * 255.0f32) as u8;

        i = i.wrapping_add(PIXEL_SIZE);
    }
}
