//! Shared definitions for the simplified-read implementation
//! (c_src/src/pngread.c, PNG_SIMPLIFIED_READ_SUPPORTED section).
#![allow(non_upper_case_globals)]

use core::ffi::c_int;

use crate::types::*;

/* Encoding of PNG data (used by the color-map code) */
pub const P_NOTSET: c_int = 0; /* File encoding not yet known */
pub const P_sRGB: c_int = 1; /* 8-bit encoded to sRGB gamma */
pub const P_LINEAR: c_int = 2; /* 16-bit linear: not encoded, NOT pre-multiplied! */
pub const P_FILE: c_int = 3; /* 8-bit encoded to file gamma, not sRGB or linear */
pub const P_LINEAR8: c_int = 4; /* 8-bit linear: only from a file value */

/* Color-map processing */
pub const PNG_CMAP_NONE: c_int = 0;
pub const PNG_CMAP_GA: c_int = 1;
pub const PNG_CMAP_TRANS: c_int = 2;
pub const PNG_CMAP_RGB: c_int = 3;
pub const PNG_CMAP_RGB_ALPHA: c_int = 4;

pub const PNG_CMAP_NONE_BACKGROUND: c_int = 256;
pub const PNG_CMAP_GA_BACKGROUND: c_int = 231;
pub const PNG_CMAP_TRANS_BACKGROUND: c_int = 254;
pub const PNG_CMAP_RGB_BACKGROUND: c_int = 256;
pub const PNG_CMAP_RGB_ALPHA_BACKGROUND: c_int = 216;

/// `png_image_read_control` from pngread.c.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct png_image_read_control {
    /* Arguments */
    pub image: png_imagep,
    pub buffer: png_voidp,
    pub row_stride: png_int_32,
    pub colormap: png_voidp,
    pub background: png_const_colorp,

    /* Instance variables */
    pub local_row: png_voidp,
    pub first_row: png_voidp,
    pub row_step: isize, /* ptrdiff_t */
    pub file_encoding: c_int,
    pub gamma_to_linear: png_fixed_point,
    pub colormap_processing: c_int,
}

/// `PNG_DIV51(v8)` from pngread.c
#[inline]
pub fn PNG_DIV51(v8: png_uint_32) -> png_uint_32 {
    (v8.wrapping_mul(5).wrapping_add(130)) >> 8
}
