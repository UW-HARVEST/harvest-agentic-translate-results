//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) exporting a
//! single public symbol: `premultiply` (declared in `include/lib.h`).
//!
//! The translation reproduces the original semantics exactly, including the
//! integer truncation / wrapping behaviour of the original pointer-arithmetic
//! loop and the f32 arithmetic used for the alpha premultiplication.

#![allow(non_camel_case_types)]

use std::ffi::c_int;

/// ```c
/// typedef struct cp_pixel_t {
///     uint8_t r;
///     uint8_t g;
///     uint8_t b;
///     uint8_t a;
/// } cp_pixel_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// ```c
/// typedef struct cp_image_t {
///     int w;
///     int h;
///     cp_pixel_t *pix;
/// } cp_image_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

/// `sizeof(cp_pixel_t)` in the original C code.
const PIXEL_SIZE: c_int = 4;

/// ```c
/// void premultiply(cp_image_t *img) {
///     int w = img->w;
///     int h = img->h;
///     int stride = w * sizeof(cp_pixel_t);
///     uint8_t *data = (uint8_t *)img->pix;
///     for (int i = 0; i < (int)stride * h; i += sizeof(cp_pixel_t)) {
///         float a = (float)data[i + 3] / 255.0f;
///         float r = (float)data[i + 0] / 255.0f;
///         float g = (float)data[i + 1] / 255.0f;
///         float b = (float)data[i + 2] / 255.0f;
///         r *= a;
///         g *= a;
///         b *= a;
///         data[i + 0] = (uint8_t)(r * 255.0f);
///         data[i + 1] = (uint8_t)(g * 255.0f);
///         data[i + 2] = (uint8_t)(b * 255.0f);
///     }
/// }
/// ```
///
/// Note: `stride` is computed with `size_t` arithmetic and then truncated back
/// to `int`, which is equivalent to a wrapping 32-bit multiplication by 4. The
/// loop bound `(int)stride * h` is likewise a 32-bit multiplication (reproduced
/// here with wrapping semantics). The alpha channel is intentionally left
/// untouched, exactly as in the original.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn premultiply(img: *mut cp_image_t) {
    let w: c_int = (*img).w;
    let h: c_int = (*img).h;
    let stride: c_int = w.wrapping_mul(PIXEL_SIZE);
    let data: *mut u8 = (*img).pix as *mut u8;

    let end: c_int = stride.wrapping_mul(h);
    let mut i: c_int = 0;
    while i < end {
        let idx = |off: c_int| -> *mut u8 { data.offset(i.wrapping_add(off) as isize) };

        let a: f32 = f32::from(*idx(3)) / 255.0f32;
        let mut r: f32 = f32::from(*idx(0)) / 255.0f32;
        let mut g: f32 = f32::from(*idx(1)) / 255.0f32;
        let mut b: f32 = f32::from(*idx(2)) / 255.0f32;

        r *= a;
        g *= a;
        b *= a;

        *idx(0) = (r * 255.0f32) as u8;
        *idx(1) = (g * 255.0f32) as u8;
        *idx(2) = (b * 255.0f32) as u8;

        i = i.wrapping_add(PIXEL_SIZE);
    }
}
