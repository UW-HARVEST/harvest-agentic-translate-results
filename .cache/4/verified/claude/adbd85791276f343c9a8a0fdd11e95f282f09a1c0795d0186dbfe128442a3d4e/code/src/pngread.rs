//! Translation of `c_src/src/pngread.c`

use crate::*;

/* Encoding of PNG data (used by the color-map code) */
pub(crate) const P_NOTSET: c_int = 0; /* File encoding not yet known */
pub(crate) const P_sRGB: c_int = 1;   /* 8-bit encoded to sRGB gamma */
pub(crate) const P_LINEAR: c_int = 2; /* 16-bit linear: not encoded, NOT pre-multiplied! */
pub(crate) const P_FILE: c_int = 3;   /* 8-bit encoded to file gamma, not sRGB or linear */
pub(crate) const P_LINEAR8: c_int = 4;/* 8-bit linear: only from a file value */

pub(crate) const PNG_CMAP_NONE: c_int = 0;
pub(crate) const PNG_CMAP_GA: c_int = 1;
pub(crate) const PNG_CMAP_TRANS: c_int = 2;
pub(crate) const PNG_CMAP_RGB: c_int = 3;
pub(crate) const PNG_CMAP_RGB_ALPHA: c_int = 4;

pub(crate) const PNG_CMAP_NONE_BACKGROUND: c_uint = 256;
pub(crate) const PNG_CMAP_GA_BACKGROUND: c_uint = 231;
pub(crate) const PNG_CMAP_TRANS_BACKGROUND: c_uint = 254;
pub(crate) const PNG_CMAP_RGB_BACKGROUND: c_uint = 256;
pub(crate) const PNG_CMAP_RGB_ALPHA_BACKGROUND: c_uint = 216;

/* Arguments to png_image_finish_read: */
#[repr(C)]
pub(crate) struct png_image_read_control {
    /* Arguments */
    pub image: png_imagep,
    pub buffer: png_voidp,
    pub row_stride: png_int_32,
    pub colormap: png_voidp,
    pub background: png_const_colorp,

    /* Instance variables */
    pub local_row: png_voidp,
    pub first_row: png_voidp,
    pub row_step: isize,          /* step between rows */
    pub file_encoding: c_int,     /* E_ values above */
    pub gamma_to_linear: png_fixed_point, /* For P_FILE, reciprocal of gamma */
    pub colormap_processing: c_int, /* PNG_CMAP_ values above */
}

pub(crate) const sRGB_TOLERANCE: png_fixed_point = 1000;

/* PNG_DIV51(v8) */
#[inline]
pub(crate) fn PNG_DIV51(v8: png_uint_32) -> png_uint_32 {
    (v8 * 5 + 130) >> 8
}

pub(crate) const PNG_GRAY_COLORMAP_ENTRIES: c_uint = 256;
pub(crate) const PNG_GA_COLORMAP_ENTRIES: c_uint = 256;
pub(crate) const PNG_RGB_COLORMAP_ENTRIES: c_uint = 216;

/* Return a palette index to the above palette given three 8-bit sRGB values. */
#[inline]
pub(crate) fn PNG_RGB_INDEX(r: png_uint_32, g: png_uint_32, b: png_uint_32) -> png_byte {
    (6 * (6 * PNG_DIV51(r) + PNG_DIV51(g)) + PNG_DIV51(b)) as png_byte
}

include!("gen/pngread_p01.rs");
include!("gen/pngread_p02.rs");
include!("gen/pngread_p03.rs");
include!("gen/pngread_p04.rs");
include!("gen/pngread_p05.rs");
include!("gen/pngread_p06.rs");
include!("gen/pngread_p07.rs");
include!("gen/pngread_p08.rs");
include!("gen/pngread_p09.rs");
