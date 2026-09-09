//! pngwtran.c lines 1-575: transforms the data in a row for PNG writers
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

/* Pack pixels into bytes.  Pass the true bit depth in bit_depth.  The
 * row_info bit depth should be 8 (one pixel per byte).  The channels
 * should be 1 (this only happens on grayscale and paletted images).
 */
pub unsafe fn png_do_pack(row_info: png_row_infop, row: png_bytep, bit_depth: png_uint_32) {
    if (*row_info).bit_depth == 8 && (*row_info).channels == 1 {
        match bit_depth as c_int {
            1 => {
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut mask: c_int;
                let mut v: c_int;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                sp = row;
                dp = row;
                mask = 0x80;
                v = 0;

                i = 0;
                while i < row_width {
                    if *sp != 0 {
                        v |= mask;
                    }

                    sp = sp.add(1);

                    if mask > 1 {
                        mask >>= 1;
                    } else {
                        mask = 0x80;
                        *dp = v as png_byte;
                        dp = dp.add(1);
                        v = 0;
                    }

                    i += 1;
                }

                if mask != 0x80 {
                    *dp = v as png_byte;
                }
            }

            2 => {
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut shift: c_uint;
                let mut v: c_int;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                sp = row;
                dp = row;
                shift = 6;
                v = 0;

                i = 0;
                while i < row_width {
                    let value: png_byte;

                    value = ((*sp as c_int) & 0x03) as png_byte;
                    v |= (value as c_int) << shift;

                    if shift == 0 {
                        shift = 6;
                        *dp = v as png_byte;
                        dp = dp.add(1);
                        v = 0;
                    } else {
                        shift -= 2;
                    }

                    sp = sp.add(1);

                    i += 1;
                }

                if shift != 6 {
                    *dp = v as png_byte;
                }
            }

            4 => {
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut shift: c_uint;
                let mut v: c_int;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                sp = row;
                dp = row;
                shift = 4;
                v = 0;

                i = 0;
                while i < row_width {
                    let value: png_byte;

                    value = ((*sp as c_int) & 0x0f) as png_byte;
                    v |= (value as c_int) << shift;

                    if shift == 0 {
                        shift = 4;
                        *dp = v as png_byte;
                        dp = dp.add(1);
                        v = 0;
                    } else {
                        shift -= 4;
                    }

                    sp = sp.add(1);

                    i += 1;
                }

                if shift != 4 {
                    *dp = v as png_byte;
                }
            }

            _ => {}
        }

        (*row_info).bit_depth = bit_depth as png_byte;
        (*row_info).pixel_depth =
            bit_depth.wrapping_mul((*row_info).channels as png_uint_32) as png_byte;
        (*row_info).rowbytes = PNG_ROWBYTES((*row_info).pixel_depth as u32, (*row_info).width);
    }
}

/* Shift pixel values to take advantage of whole range.  Pass the
 * true number of bits in bit_depth.  The row should be packed
 * according to row_info->bit_depth.  Thus, if you had a row of
 * bit depth 4, but the pixels only had values from 0 to 7, you
 * would pass 3 as bit_depth, and this routine would translate the
 * data to 0 to 15.
 */
pub unsafe fn png_do_shift(
    row_info: png_row_infop,
    row: png_bytep,
    bit_depth: png_const_color_8p,
) {
    if (*row_info).color_type as c_int != PNG_COLOR_TYPE_PALETTE {
        let mut shift_start: [c_int; 4] = [0; 4];
        let mut shift_dec: [c_int; 4] = [0; 4];
        let mut channels: c_uint = 0;

        if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
            shift_start[channels as usize] =
                (*row_info).bit_depth as c_int - (*bit_depth).red as c_int;
            shift_dec[channels as usize] = (*bit_depth).red as c_int;
            channels += 1;

            shift_start[channels as usize] =
                (*row_info).bit_depth as c_int - (*bit_depth).green as c_int;
            shift_dec[channels as usize] = (*bit_depth).green as c_int;
            channels += 1;

            shift_start[channels as usize] =
                (*row_info).bit_depth as c_int - (*bit_depth).blue as c_int;
            shift_dec[channels as usize] = (*bit_depth).blue as c_int;
            channels += 1;
        } else {
            shift_start[channels as usize] =
                (*row_info).bit_depth as c_int - (*bit_depth).gray as c_int;
            shift_dec[channels as usize] = (*bit_depth).gray as c_int;
            channels += 1;
        }

        if ((*row_info).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0 {
            shift_start[channels as usize] =
                (*row_info).bit_depth as c_int - (*bit_depth).alpha as c_int;
            shift_dec[channels as usize] = (*bit_depth).alpha as c_int;
            channels += 1;
        }

        /* With low row depths, could only be grayscale, so one channel */
        if (*row_info).bit_depth < 8 {
            let mut bp: png_bytep = row;
            let mut i: usize;
            let mask: c_uint;
            let row_bytes: usize = (*row_info).rowbytes;

            if (*bit_depth).gray == 1 && (*row_info).bit_depth == 2 {
                mask = 0x55;
            } else if (*row_info).bit_depth == 4 && (*bit_depth).gray == 3 {
                mask = 0x11;
            } else {
                mask = 0xff;
            }

            i = 0;
            while i < row_bytes {
                let mut j: c_int;
                let v: c_uint;
                let mut out: c_uint;

                v = *bp as c_uint;
                out = 0;

                j = shift_start[0];
                while j > -shift_dec[0] {
                    if j > 0 {
                        out |= v << j;
                    } else {
                        out |= (v >> (-j)) & mask;
                    }

                    j -= shift_dec[0];
                }

                *bp = (out & 0xff) as png_byte;

                i += 1;
                bp = bp.add(1);
            }
        } else if (*row_info).bit_depth == 8 {
            let mut bp: png_bytep = row;
            let mut i: png_uint_32;
            let istop: png_uint_32 = channels.wrapping_mul((*row_info).width);

            i = 0;
            while i < istop {
                let c: c_uint = i % channels;
                let mut j: c_int;
                let v: c_uint;
                let mut out: c_uint;

                v = *bp as c_uint;
                out = 0;

                j = shift_start[c as usize];
                while j > -shift_dec[c as usize] {
                    if j > 0 {
                        out |= v << j;
                    } else {
                        out |= v >> (-j);
                    }

                    j -= shift_dec[c as usize];
                }

                *bp = (out & 0xff) as png_byte;

                i += 1;
                bp = bp.add(1);
            }
        } else {
            let mut bp: png_bytep;
            let mut i: png_uint_32;
            let istop: png_uint_32 = channels.wrapping_mul((*row_info).width);

            bp = row;
            i = 0;
            while i < istop {
                let c: c_uint = i % channels;
                let mut j: c_int;
                let mut value: c_uint;
                let v: c_uint;

                v = png_get_uint_16(bp as png_const_bytep) as c_uint;
                value = 0;

                j = shift_start[c as usize];
                while j > -shift_dec[c as usize] {
                    if j > 0 {
                        value |= v << j;
                    } else {
                        value |= v >> (-j);
                    }

                    j -= shift_dec[c as usize];
                }
                *bp = ((value >> 8) & 0xff) as png_byte;
                bp = bp.add(1);
                *bp = (value & 0xff) as png_byte;
                bp = bp.add(1);

                i += 1;
            }
        }
    }
}

pub unsafe fn png_do_write_swap_alpha(row_info: png_row_infop, row: png_bytep) {
    {
        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
            if (*row_info).bit_depth == 8 {
                /* This converts from ARGB to RGBA */
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                i = 0;
                dp = row;
                sp = dp;
                while i < row_width {
                    let save: png_byte = *sp;
                    sp = sp.add(1);
                    *dp = *sp;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = *sp;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = *sp;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = save;
                    dp = dp.add(1);

                    i += 1;
                }
            } else {
                /* This converts from AARRGGBB to RRGGBBAA */
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                i = 0;
                dp = row;
                sp = dp;
                while i < row_width {
                    let mut save: [png_byte; 2] = [0; 2];
                    save[0] = *sp;
                    sp = sp.add(1);
                    save[1] = *sp;
                    sp = sp.add(1);
                    *dp = *sp;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = *sp;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = *sp;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = *sp;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = *sp;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = *sp;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = save[0];
                    dp = dp.add(1);
                    *dp = save[1];
                    dp = dp.add(1);

                    i += 1;
                }
            }
        } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
            if (*row_info).bit_depth == 8 {
                /* This converts from AG to GA */
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                i = 0;
                dp = row;
                sp = dp;
                while i < row_width {
                    let save: png_byte = *sp;
                    sp = sp.add(1);
                    *dp = *sp;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = save;
                    dp = dp.add(1);

                    i += 1;
                }
            } else {
                /* This converts from AAGG to GGAA */
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                i = 0;
                dp = row;
                sp = dp;
                while i < row_width {
                    let mut save: [png_byte; 2] = [0; 2];
                    save[0] = *sp;
                    sp = sp.add(1);
                    save[1] = *sp;
                    sp = sp.add(1);
                    *dp = *sp;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = *sp;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = save[0];
                    dp = dp.add(1);
                    *dp = save[1];
                    dp = dp.add(1);

                    i += 1;
                }
            }
        }
    }
}

pub unsafe fn png_do_write_invert_alpha(row_info: png_row_infop, row: png_bytep) {
    {
        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
            if (*row_info).bit_depth == 8 {
                /* This inverts the alpha channel in RGBA */
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                i = 0;
                dp = row;
                sp = dp;
                while i < row_width {
                    /* Does nothing
                    *(dp++) = *(sp++);
                    *(dp++) = *(sp++);
                    *(dp++) = *(sp++);
                    */
                    sp = sp.add(3);
                    dp = sp;
                    *dp = (255 - *sp as c_int) as png_byte;
                    sp = sp.add(1);

                    i += 1;
                }
            } else {
                /* This inverts the alpha channel in RRGGBBAA */
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                i = 0;
                dp = row;
                sp = dp;
                while i < row_width {
                    /* Does nothing
                    *(dp++) = *(sp++);
                    *(dp++) = *(sp++);
                    *(dp++) = *(sp++);
                    *(dp++) = *(sp++);
                    *(dp++) = *(sp++);
                    *(dp++) = *(sp++);
                    */
                    sp = sp.add(6);
                    dp = sp;
                    *dp = (255 - *sp as c_int) as png_byte;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = (255 - *sp as c_int) as png_byte;
                    sp = sp.add(1);

                    i += 1;
                }
            }
        } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
            if (*row_info).bit_depth == 8 {
                /* This inverts the alpha channel in GA */
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                i = 0;
                dp = row;
                sp = dp;
                while i < row_width {
                    *dp = *sp;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = (255 - *sp as c_int) as png_byte;
                    dp = dp.add(1);
                    sp = sp.add(1);

                    i += 1;
                }
            } else {
                /* This inverts the alpha channel in GGAA */
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                i = 0;
                dp = row;
                sp = dp;
                while i < row_width {
                    /* Does nothing
                    *(dp++) = *(sp++);
                    *(dp++) = *(sp++);
                    */
                    sp = sp.add(2);
                    dp = sp;
                    *dp = (255 - *sp as c_int) as png_byte;
                    dp = dp.add(1);
                    sp = sp.add(1);
                    *dp = (255 - *sp as c_int) as png_byte;
                    sp = sp.add(1);

                    i += 1;
                }
            }
        }
    }
}

/* Transform the data according to the user's wishes.  The order of
 * transformations is significant.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_do_write_transformations(
    png_ptr: png_structrp,
    row_info: png_row_infop,
) {
    if png_ptr.is_null() {
        return;
    }

    if ((*png_ptr).transformations & PNG_USER_TRANSFORM) != 0 {
        if (*png_ptr).write_user_transform_fn.is_some() {
            ((*png_ptr).write_user_transform_fn.unwrap())(
                /* User write transform function */
                png_ptr,   /* png_ptr */
                row_info,  /* row_info: */
                /*  png_uint_32 width;       width of row */
                /*  size_t rowbytes;         number of bytes in row */
                /*  png_byte color_type;     color type of pixels */
                /*  png_byte bit_depth;      bit depth of samples */
                /*  png_byte channels;       number of channels (1-4) */
                /*  png_byte pixel_depth;    bits per pixel (depth*channels) */
                (*png_ptr).row_buf.add(1),
            ); /* start of pixel data for row */
        }
    }

    if ((*png_ptr).transformations & PNG_FILLER) != 0 {
        png_do_strip_channel(
            row_info,
            (*png_ptr).row_buf.add(1),
            (((*png_ptr).flags & PNG_FLAG_FILLER_AFTER) == 0) as c_int,
        );
    }

    if ((*png_ptr).transformations & PNG_PACKSWAP) != 0 {
        png_do_packswap(row_info, (*png_ptr).row_buf.add(1));
    }

    if ((*png_ptr).transformations & PNG_PACK) != 0 {
        png_do_pack(
            row_info,
            (*png_ptr).row_buf.add(1),
            (*png_ptr).bit_depth as png_uint_32,
        );
    }

    if ((*png_ptr).transformations & PNG_SWAP_BYTES) != 0 {
        png_do_swap(row_info, (*png_ptr).row_buf.add(1));
    }

    if ((*png_ptr).transformations & PNG_SHIFT) != 0 {
        png_do_shift(
            row_info,
            (*png_ptr).row_buf.add(1),
            &(*png_ptr).shift as png_const_color_8p,
        );
    }

    if ((*png_ptr).transformations & PNG_SWAP_ALPHA) != 0 {
        png_do_write_swap_alpha(row_info, (*png_ptr).row_buf.add(1));
    }

    if ((*png_ptr).transformations & PNG_INVERT_ALPHA) != 0 {
        png_do_write_invert_alpha(row_info, (*png_ptr).row_buf.add(1));
    }

    if ((*png_ptr).transformations & PNG_BGR) != 0 {
        png_do_bgr(row_info, (*png_ptr).row_buf.add(1));
    }

    if ((*png_ptr).transformations & PNG_INVERT_MONO) != 0 {
        png_do_invert(row_info, (*png_ptr).row_buf.add(1));
    }
}
