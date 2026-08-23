//! Translation of `c_src/src/pngpread.c`

use crate::*;

pub(crate) const PNG_READ_SIG_MODE: c_int = 0;
pub(crate) const PNG_READ_CHUNK_MODE: c_int = 1;
pub(crate) const PNG_READ_IDAT_MODE: c_int = 2;
pub(crate) const PNG_READ_tEXt_MODE: c_int = 4;
pub(crate) const PNG_READ_zTXt_MODE: c_int = 5;
pub(crate) const PNG_READ_DONE_MODE: c_int = 6;
pub(crate) const PNG_READ_iTXt_MODE: c_int = 7;
pub(crate) const PNG_ERROR_MODE: c_int = 8;

/* Arrays to facilitate interlacing - use pass (0 - 6) as index. */
pub(crate) static png_pass_start: [png_byte; 7] = [0, 4, 0, 2, 0, 1, 0];
pub(crate) static png_pass_inc: [png_byte; 7] = [8, 8, 4, 4, 2, 2, 1];
pub(crate) static png_pass_ystart: [png_byte; 7] = [0, 0, 4, 0, 2, 0, 1];
pub(crate) static png_pass_yinc: [png_byte; 7] = [8, 8, 8, 4, 4, 2, 2];

include!("gen/pngpread_p01.rs");
include!("gen/pngpread_p02.rs");
