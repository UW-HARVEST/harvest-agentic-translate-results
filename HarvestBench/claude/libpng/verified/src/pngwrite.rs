//! Translation of `c_src/src/pngwrite.c`

use crate::*;

/* Arguments to png_image_write_main: */
#[repr(C)]
pub(crate) struct png_image_write_control {
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

include!("gen/pngwrite_p01.rs");
include!("gen/pngwrite_p02.rs");
include!("gen/pngwrite_p03.rs");
include!("gen/pngwrite_p04.rs");
include!("gen/pngwrite_p05.rs");
