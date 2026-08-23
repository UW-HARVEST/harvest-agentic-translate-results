//! Translation of `c_src/src/pngwutil.c`

use crate::*;

/* Arrays to facilitate interlacing - use pass (0 - 6) as index. */
pub(crate) static png_pass_start: [png_byte; 7] = [0, 4, 0, 2, 0, 1, 0];
pub(crate) static png_pass_inc: [png_byte; 7] = [8, 8, 4, 4, 2, 2, 1];
pub(crate) static png_pass_ystart: [png_byte; 7] = [0, 0, 4, 0, 2, 0, 1];
pub(crate) static png_pass_yinc: [png_byte; 7] = [8, 8, 8, 4, 4, 2, 2];

#[repr(C)]
pub(crate) struct compression_state {
    pub input: png_const_bytep,      /* The uncompressed input data */
    pub input_len: png_alloc_size_t, /* Its length */
    pub output_len: png_uint_32,     /* Final compressed length */
    pub output: [png_byte; 1024],    /* First block of output */
}

include!("gen/pngwutil_p01.rs");
include!("gen/pngwutil_p02.rs");
include!("gen/pngwutil_p03.rs");
include!("gen/pngwutil_p04.rs");
include!("gen/pngwutil_p05.rs");
include!("gen/pngwutil_p06.rs");
include!("gen/pngwutil_p07.rs");
