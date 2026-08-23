//! pngwutil.c — assembled from the translated chunks.
use crate::*;

/* Constant tables for the interlace passes; pngwutil.c lines 24-30, i.e. the
 * file scope statics that precede the first translated function.
 */
static png_pass_start: [png_byte; 7] = [0, 4, 0, 2, 0, 1, 0];
static png_pass_inc: [png_byte; 7] = [8, 8, 4, 4, 2, 2, 1];
static png_pass_ystart: [png_byte; 7] = [0, 0, 4, 0, 2, 0, 1];
static png_pass_yinc: [png_byte; 7] = [8, 8, 8, 4, 4, 2, 2];

include!("pngwutil_p1.rs");
include!("pngwutil_p2.rs");
include!("pngwutil_p3.rs");
include!("pngwutil_p4.rs");
