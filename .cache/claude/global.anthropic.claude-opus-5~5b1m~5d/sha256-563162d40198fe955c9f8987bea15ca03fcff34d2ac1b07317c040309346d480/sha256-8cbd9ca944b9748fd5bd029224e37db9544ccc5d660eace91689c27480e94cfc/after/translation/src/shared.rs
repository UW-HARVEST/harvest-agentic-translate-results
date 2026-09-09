//! File-local (C `static` / typedef) definitions from pngread.c, pngwrite.c and
//! pngwutil.c that are shared between the Rust modules a single C file was
//! split into.
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use crate::types::*;
use core::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// pngread.c
// ---------------------------------------------------------------------------

/* File encodings ("E_"/"P_" values) */
pub const P_NOTSET: c_int = 0; /* File encoding not yet known */
pub const P_sRGB: c_int = 1; /* 8-bit encoded to sRGB gamma */
pub const P_LINEAR: c_int = 2; /* 16-bit linear: not encoded, NOT pre-multiplied! */
pub const P_FILE: c_int = 3; /* 8-bit encoded to file gamma, not sRGB or linear */
pub const P_LINEAR8: c_int = 4; /* 8-bit linear: only from a file value */

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

pub const PNG_GRAY_COLORMAP_ENTRIES: c_int = 256;
pub const PNG_GA_COLORMAP_ENTRIES: c_int = 256;
pub const PNG_RGB_COLORMAP_ENTRIES: c_int = 216;
pub const sRGB_TOLERANCE: png_fixed_point = 1000;

/// `PNG_DIV51(v8)`
#[inline]
pub const fn PNG_DIV51(v8: png_uint_32) -> png_uint_32 {
    (v8 * 5 + 130) >> 8
}

/// `PNG_RGB_INDEX(r,g,b)` from pngread.c
#[inline]
pub const fn PNG_RGB_INDEX(r: png_uint_32, g: png_uint_32, b: png_uint_32) -> png_byte {
    (PNG_DIV51(r) * 36 + PNG_DIV51(g) * 6 + PNG_DIV51(b)) as png_byte
}

#[repr(C)]
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
    pub row_step: isize,
    pub file_encoding: c_int,
    pub gamma_to_linear: png_fixed_point,
    pub colormap_processing: c_int,
}

impl Default for png_image_read_control {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}

// ---------------------------------------------------------------------------
// pngwrite.c
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct png_image_write_control {
    /* Arguments */
    pub image: png_imagep,
    pub buffer: png_const_voidp,
    pub row_stride: png_int_32,
    pub colormap: png_const_voidp,
    pub convert_to_8bit: c_int,

    /* Instance variables */
    pub first_row: png_const_voidp,
    pub local_row: png_voidp,
    pub row_step: isize,

    /* Byte count for memory writing */
    pub memory: png_bytep,
    pub memory_bytes: png_alloc_size_t, /* not used for STDIO */
    pub output_bytes: png_alloc_size_t, /* running total */
}

impl Default for png_image_write_control {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}

// ---------------------------------------------------------------------------
// pngrutil.c
// ---------------------------------------------------------------------------

/// `LZ77Min` - minimum LZ77 match length overhead used by png_decompress_chunk.
pub const LZ77Min: png_uint_32 = 2 + 5 + 4;

/* Arrays to facilitate interlace calculations */
pub static png_pass_start: [png_byte; 7] = [0, 4, 0, 2, 0, 1, 0];
pub static png_pass_inc: [png_byte; 7] = [8, 8, 4, 4, 2, 2, 1];
pub static png_pass_ystart: [png_byte; 7] = [0, 0, 4, 0, 2, 0, 1];
pub static png_pass_yinc: [png_byte; 7] = [8, 8, 8, 4, 4, 2, 2];

// ---------------------------------------------------------------------------
// pngwutil.c
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct compression_state {
    pub input: png_const_bytep,       /* The uncompressed input data */
    pub input_len: png_alloc_size_t,  /* Its length */
    pub output_len: png_uint_32,      /* Final compressed length */
    pub output: [png_byte; 1024],     /* First block of output */
}

impl Default for compression_state {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}
