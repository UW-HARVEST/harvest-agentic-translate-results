//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) exposing
//! one public symbol: `flip_horizontal`. The public types (`cp_pixel_t`,
//! `cp_image_t`) come from `include/lib.h`.
//!
//! Behaviour is reproduced exactly, including the original's (mis)naming: the
//! function is called `flip_horizontal` but it actually swaps rows, i.e. it
//! flips the image vertically. That is *not* "fixed" here.

#![allow(non_camel_case_types)]

use std::ffi::c_int;

/// `typedef struct cp_pixel_t { uint8_t r, g, b, a; } cp_pixel_t;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// `typedef struct cp_image_t { int w; int h; cp_pixel_t *pix; } cp_image_t;`
#[repr(C)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

/// ```c
/// void flip_horizontal(cp_image_t *img) {
///     cp_pixel_t *pix = img->pix;
///     int w = img->w;
///     int h = img->h;
///     int flips = h / 2;
///     for (int i = 0; i < flips; ++i) {
///         cp_pixel_t *a = pix + w * i;
///         cp_pixel_t *b = pix + w * (h - i - 1);
///         for (int j = 0; j < w; ++j) {
///             cp_pixel_t t = *a;
///             *a = *b;
///             *b = t;
///             ++a;
///             ++b;
///         }
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn flip_horizontal(img: *mut cp_image_t) {
    // The C code unconditionally dereferences `img`; a NULL pointer faults
    // there exactly as it does here.
    let pix: *mut cp_pixel_t = unsafe { (*img).pix };
    let w: c_int = unsafe { (*img).w };
    let h: c_int = unsafe { (*img).h };

    // Integer division truncates toward zero, matching C's `h / 2`.
    // (`h == INT_MIN` is division of the minimum value by 2, which is exact.)
    let flips: c_int = h.wrapping_div(2);

    let mut i: c_int = 0;
    while i < flips {
        // `pix + w * i` and `pix + w * (h - i - 1)`: the index expressions are
        // computed in `int` before being widened for the pointer arithmetic,
        // so wrapping ops reproduce the usual C codegen bit-for-bit.
        let off_a = w.wrapping_mul(i) as isize;
        let off_b = w.wrapping_mul(h.wrapping_sub(i).wrapping_sub(1)) as isize;

        let mut a: *mut cp_pixel_t = unsafe { pix.offset(off_a) };
        let mut b: *mut cp_pixel_t = unsafe { pix.offset(off_b) };

        let mut j: c_int = 0;
        while j < w {
            unsafe {
                let t = *a;
                *a = *b;
                *b = t;
                a = a.offset(1);
                b = b.offset(1);
            }
            j = j.wrapping_add(1);
        }

        i = i.wrapping_add(1);
    }
}
