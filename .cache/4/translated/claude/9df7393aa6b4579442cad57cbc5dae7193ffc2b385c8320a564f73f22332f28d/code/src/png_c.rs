//! Translation of `c_src/src/png.c`

use crate::*;

include!("gen/png_c_tables.rs");

/* png.c: PNG_sRGB_FROM_LINEAR (pngpriv.h) */
#[inline]
pub fn PNG_sRGB_FROM_LINEAR(linear: png_uint_32) -> png_byte {
    (0xff & ((png_sRGB_base[(linear >> 15) as usize] as png_uint_32
        + ((((linear & 0x7fff) * png_sRGB_delta[(linear >> 15) as usize] as png_uint_32) >> 12)))
        >> 8)) as png_byte
}

/* png.c fp parser helper macros: png_fp_add / png_fp_set from line 2130.
 * Renamed to *_state because png.c also has a static function png_fp_add(). */
#[inline]
pub(crate) fn png_fp_add_state(state: &mut c_int, flags: c_int) {
    *state |= flags;
}
#[inline]
pub(crate) fn png_fp_set_state(state: &mut c_int, value: c_int) {
    *state = value | (*state & PNG_FP_STICKY);
}

include!("gen/png_c_p01.rs");
include!("gen/png_c_p02.rs");
include!("gen/png_c_p03.rs");
include!("gen/png_c_p04.rs");
include!("gen/png_c_p05.rs");
include!("gen/png_c_p06.rs");
include!("gen/png_c_p07.rs");
include!("gen/png_c_p08.rs");
include!("gen/png_c_p09.rs");
include!("gen/png_c_p10.rs");
