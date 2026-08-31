//! pngrtran.c lines 4344-5180: palette/bit-depth expansion, 16-bit expansion,
//! quantization and the master read-transformation driver.
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

/* Expands a palette row to an RGB or RGBA row depending
 * upon whether you supply trans and num_trans.
 */
pub unsafe fn png_do_expand_palette(
    png_ptr: png_structrp,
    row_info: png_row_infop,
    row: png_bytep,
    palette: png_const_colorp,
    trans_alpha: png_const_bytep,
    num_trans: c_int,
) {
    let mut shift: c_int;
    let mut value: c_int;
    let mut sp: png_bytep;
    let mut dp: png_bytep;
    let mut i: png_uint_32;
    let row_width: png_uint_32 = (*row_info).width;

    if (*row_info).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        if ((*row_info).bit_depth as c_int) < 8 {
            match (*row_info).bit_depth as c_int {
                1 => {
                    sp = row.add((row_width.wrapping_sub(1) >> 3) as usize);
                    dp = row.add(row_width as usize).sub(1);
                    shift = 7 - ((row_width.wrapping_add(7) & 0x07) as c_int);
                    i = 0;
                    while i < row_width {
                        if ((*sp as c_int) >> shift) & 0x01 != 0 {
                            *dp = 1;
                        } else {
                            *dp = 0;
                        }

                        if shift == 7 {
                            shift = 0;
                            sp = sp.sub(1);
                        } else {
                            shift += 1;
                        }

                        dp = dp.sub(1);
                        i = i.wrapping_add(1);
                    }
                }

                2 => {
                    sp = row.add((row_width.wrapping_sub(1) >> 2) as usize);
                    dp = row.add(row_width as usize).sub(1);
                    shift = ((3u32.wrapping_sub(row_width.wrapping_add(3) & 0x03)) << 1) as c_int;
                    i = 0;
                    while i < row_width {
                        value = ((*sp as c_int) >> shift) & 0x03;
                        *dp = value as png_byte;
                        if shift == 6 {
                            shift = 0;
                            sp = sp.sub(1);
                        } else {
                            shift += 2;
                        }

                        dp = dp.sub(1);
                        i = i.wrapping_add(1);
                    }
                }

                4 => {
                    sp = row.add((row_width.wrapping_sub(1) >> 1) as usize);
                    dp = row.add(row_width as usize).sub(1);
                    shift = ((row_width & 0x01) << 2) as c_int;
                    i = 0;
                    while i < row_width {
                        value = ((*sp as c_int) >> shift) & 0x0f;
                        *dp = value as png_byte;
                        if shift == 4 {
                            shift = 0;
                            sp = sp.sub(1);
                        } else {
                            shift += 4;
                        }

                        dp = dp.sub(1);
                        i = i.wrapping_add(1);
                    }
                }

                _ => {}
            }
            (*row_info).bit_depth = 8;
            (*row_info).pixel_depth = 8;
            (*row_info).rowbytes = row_width as usize;
        }

        if (*row_info).bit_depth == 8 {
            {
                if num_trans > 0 {
                    sp = row.add(row_width as usize).sub(1);
                    dp = row.add(((row_width as usize) << 2)).sub(1);

                    i = 0;
                    let _ = png_ptr;

                    while i < row_width {
                        if (*sp as c_int) >= num_trans {
                            *dp = 0xff;
                            dp = dp.sub(1);
                        } else {
                            *dp = *trans_alpha.add(*sp as usize);
                            dp = dp.sub(1);
                        }
                        *dp = (*palette.add(*sp as usize)).blue;
                        dp = dp.sub(1);
                        *dp = (*palette.add(*sp as usize)).green;
                        dp = dp.sub(1);
                        *dp = (*palette.add(*sp as usize)).red;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        i = i.wrapping_add(1);
                    }
                    (*row_info).bit_depth = 8;
                    (*row_info).pixel_depth = 32;
                    (*row_info).rowbytes = (row_width as usize) * 4;
                    (*row_info).color_type = 6;
                    (*row_info).channels = 4;
                } else {
                    sp = row.add(row_width as usize).sub(1);
                    dp = row.add((row_width as usize) * 3).sub(1);
                    i = 0;
                    let _ = png_ptr;

                    while i < row_width {
                        *dp = (*palette.add(*sp as usize)).blue;
                        dp = dp.sub(1);
                        *dp = (*palette.add(*sp as usize)).green;
                        dp = dp.sub(1);
                        *dp = (*palette.add(*sp as usize)).red;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        i = i.wrapping_add(1);
                    }

                    (*row_info).bit_depth = 8;
                    (*row_info).pixel_depth = 24;
                    (*row_info).rowbytes = (row_width as usize) * 3;
                    (*row_info).color_type = 2;
                    (*row_info).channels = 3;
                }
            }
        }
    }
}

/* If the bit depth < 8, it is expanded to 8.  Also, if the already
 * expanded transparency value is supplied, an alpha channel is built.
 */
pub unsafe fn png_do_expand(
    row_info: png_row_infop,
    row: png_bytep,
    trans_color: png_const_color_16p,
) {
    let mut shift: c_int;
    let mut value: c_int;
    let mut sp: png_bytep;
    let mut dp: png_bytep;
    let mut i: png_uint_32;
    let row_width: png_uint_32 = (*row_info).width;

    if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY {
        let mut gray: c_uint = if !trans_color.is_null() {
            (*trans_color).gray as c_uint
        } else {
            0
        };

        if ((*row_info).bit_depth as c_int) < 8 {
            match (*row_info).bit_depth as c_int {
                1 => {
                    gray = (gray & 0x01).wrapping_mul(0xff);
                    sp = row.add((row_width.wrapping_sub(1) >> 3) as usize);
                    dp = row.add(row_width as usize).sub(1);
                    shift = 7 - ((row_width.wrapping_add(7) & 0x07) as c_int);
                    i = 0;
                    while i < row_width {
                        if ((*sp as c_int) >> shift) & 0x01 != 0 {
                            *dp = 0xff;
                        } else {
                            *dp = 0;
                        }

                        if shift == 7 {
                            shift = 0;
                            sp = sp.sub(1);
                        } else {
                            shift += 1;
                        }

                        dp = dp.sub(1);
                        i = i.wrapping_add(1);
                    }
                }

                2 => {
                    gray = (gray & 0x03).wrapping_mul(0x55);
                    sp = row.add((row_width.wrapping_sub(1) >> 2) as usize);
                    dp = row.add(row_width as usize).sub(1);
                    shift = ((3u32.wrapping_sub(row_width.wrapping_add(3) & 0x03)) << 1) as c_int;
                    i = 0;
                    while i < row_width {
                        value = ((*sp as c_int) >> shift) & 0x03;
                        *dp = (value | (value << 2) | (value << 4) | (value << 6)) as png_byte;
                        if shift == 6 {
                            shift = 0;
                            sp = sp.sub(1);
                        } else {
                            shift += 2;
                        }

                        dp = dp.sub(1);
                        i = i.wrapping_add(1);
                    }
                }

                4 => {
                    gray = (gray & 0x0f).wrapping_mul(0x11);
                    sp = row.add((row_width.wrapping_sub(1) >> 1) as usize);
                    dp = row.add(row_width as usize).sub(1);
                    shift = ((1u32.wrapping_sub(row_width.wrapping_add(1) & 0x01)) << 2) as c_int;
                    i = 0;
                    while i < row_width {
                        value = ((*sp as c_int) >> shift) & 0x0f;
                        *dp = (value | (value << 4)) as png_byte;
                        if shift == 4 {
                            shift = 0;
                            sp = sp.sub(1);
                        } else {
                            shift = 4;
                        }

                        dp = dp.sub(1);
                        i = i.wrapping_add(1);
                    }
                }

                _ => {}
            }

            (*row_info).bit_depth = 8;
            (*row_info).pixel_depth = 8;
            (*row_info).rowbytes = row_width as usize;
        }

        if !trans_color.is_null() {
            if (*row_info).bit_depth == 8 {
                gray = gray & 0xff;
                sp = row.add(row_width as usize).sub(1);
                dp = row.add((row_width as usize) << 1).sub(1);

                i = 0;
                while i < row_width {
                    if ((*sp as c_uint) & 0xffu32) == gray {
                        *dp = 0;
                        dp = dp.sub(1);
                    } else {
                        *dp = 0xff;
                        dp = dp.sub(1);
                    }

                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    i = i.wrapping_add(1);
                }
            } else if (*row_info).bit_depth == 16 {
                let gray_high: c_uint = (gray >> 8) & 0xff;
                let gray_low: c_uint = gray & 0xff;
                sp = row.add((*row_info).rowbytes).sub(1);
                dp = row.add((*row_info).rowbytes << 1).sub(1);
                i = 0;
                while i < row_width {
                    if ((*sp.sub(1) as c_uint) & 0xffu32) == gray_high
                        && ((*sp as c_uint) & 0xffu32) == gray_low
                    {
                        *dp = 0;
                        dp = dp.sub(1);
                        *dp = 0;
                        dp = dp.sub(1);
                    } else {
                        *dp = 0xff;
                        dp = dp.sub(1);
                        *dp = 0xff;
                        dp = dp.sub(1);
                    }

                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    i = i.wrapping_add(1);
                }
            }

            (*row_info).color_type = PNG_COLOR_TYPE_GRAY_ALPHA as png_byte;
            (*row_info).channels = 2;
            (*row_info).pixel_depth = (((*row_info).bit_depth as c_int) << 1) as png_byte;
            (*row_info).rowbytes = PNG_ROWBYTES((*row_info).pixel_depth as u32, row_width);
        }
    } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB && !trans_color.is_null() {
        if (*row_info).bit_depth == 8 {
            let red: png_byte = ((*trans_color).red as c_int & 0xff) as png_byte;
            let green: png_byte = ((*trans_color).green as c_int & 0xff) as png_byte;
            let blue: png_byte = ((*trans_color).blue as c_int & 0xff) as png_byte;
            sp = row.add((*row_info).rowbytes).sub(1);
            dp = row.add((row_width as usize) << 2).sub(1);
            i = 0;
            while i < row_width {
                if *sp.sub(2) == red && *sp.sub(1) == green && *sp == blue {
                    *dp = 0;
                    dp = dp.sub(1);
                } else {
                    *dp = 0xff;
                    dp = dp.sub(1);
                }

                *dp = *sp;
                dp = dp.sub(1);
                sp = sp.sub(1);
                *dp = *sp;
                dp = dp.sub(1);
                sp = sp.sub(1);
                *dp = *sp;
                dp = dp.sub(1);
                sp = sp.sub(1);
                i = i.wrapping_add(1);
            }
        } else if (*row_info).bit_depth == 16 {
            let red_high: png_byte = (((*trans_color).red as c_int >> 8) & 0xff) as png_byte;
            let green_high: png_byte = (((*trans_color).green as c_int >> 8) & 0xff) as png_byte;
            let blue_high: png_byte = (((*trans_color).blue as c_int >> 8) & 0xff) as png_byte;
            let red_low: png_byte = ((*trans_color).red as c_int & 0xff) as png_byte;
            let green_low: png_byte = ((*trans_color).green as c_int & 0xff) as png_byte;
            let blue_low: png_byte = ((*trans_color).blue as c_int & 0xff) as png_byte;
            sp = row.add((*row_info).rowbytes).sub(1);
            dp = row.add((row_width as usize) << 3).sub(1);
            i = 0;
            while i < row_width {
                if *sp.sub(5) == red_high
                    && *sp.sub(4) == red_low
                    && *sp.sub(3) == green_high
                    && *sp.sub(2) == green_low
                    && *sp.sub(1) == blue_high
                    && *sp == blue_low
                {
                    *dp = 0;
                    dp = dp.sub(1);
                    *dp = 0;
                    dp = dp.sub(1);
                } else {
                    *dp = 0xff;
                    dp = dp.sub(1);
                    *dp = 0xff;
                    dp = dp.sub(1);
                }

                *dp = *sp;
                dp = dp.sub(1);
                sp = sp.sub(1);
                *dp = *sp;
                dp = dp.sub(1);
                sp = sp.sub(1);
                *dp = *sp;
                dp = dp.sub(1);
                sp = sp.sub(1);
                *dp = *sp;
                dp = dp.sub(1);
                sp = sp.sub(1);
                *dp = *sp;
                dp = dp.sub(1);
                sp = sp.sub(1);
                *dp = *sp;
                dp = dp.sub(1);
                sp = sp.sub(1);
                i = i.wrapping_add(1);
            }
        }
        (*row_info).color_type = PNG_COLOR_TYPE_RGB_ALPHA as png_byte;
        (*row_info).channels = 4;
        (*row_info).pixel_depth = (((*row_info).bit_depth as c_int) << 2) as png_byte;
        (*row_info).rowbytes = PNG_ROWBYTES((*row_info).pixel_depth as u32, row_width);
    }
}

/* If the bit depth is 8 and the color type is not a palette type expand the
 * whole row to 16 bits.  Has no effect otherwise.
 */
pub unsafe fn png_do_expand_16(row_info: png_row_infop, row: png_bytep) {
    if (*row_info).bit_depth == 8 && (*row_info).color_type as c_int != PNG_COLOR_TYPE_PALETTE {
        /* The row have a sequence of bytes containing [0..255] and we need
         * to turn it into another row containing [0..65535], to do this we
         * calculate:
         *
         *  (input / 255) * 65535
         *
         *  Which happens to be exactly input * 257 and this can be achieved
         *  simply by byte replication in place (copying backwards).
         */
        let mut sp: *mut png_byte = row.add((*row_info).rowbytes); /* source, last byte + 1 */
        let mut dp: *mut png_byte = sp.add((*row_info).rowbytes); /* destination, end + 1 */
        while dp > sp {
            sp = sp.sub(1);
            let v: png_byte = *sp;
            *dp.sub(1) = v;
            *dp.sub(2) = v;
            dp = dp.sub(2);
        }

        (*row_info).rowbytes = (*row_info).rowbytes.wrapping_mul(2);
        (*row_info).bit_depth = 16;
        (*row_info).pixel_depth = (((*row_info).channels as c_int) * 16) as png_byte;
    }
}

pub unsafe fn png_do_quantize(
    row_info: png_row_infop,
    row: png_bytep,
    palette_lookup: png_const_bytep,
    quantize_lookup: png_const_bytep,
) {
    let mut sp: png_bytep;
    let mut dp: png_bytep;
    let mut i: png_uint_32;
    let row_width: png_uint_32 = (*row_info).width;

    if (*row_info).bit_depth == 8 {
        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB && !palette_lookup.is_null() {
            let mut r: c_int;
            let mut g: c_int;
            let mut b: c_int;
            let mut p: c_int;
            sp = row;
            dp = row;
            i = 0;
            while i < row_width {
                r = *sp as c_int;
                sp = sp.add(1);
                g = *sp as c_int;
                sp = sp.add(1);
                b = *sp as c_int;
                sp = sp.add(1);

                /* This looks real messy, but the compiler will reduce
                 * it down to a reasonable formula.  For example, with
                 * 5 bits per color, we get:
                 * p = (((r >> 3) & 0x1f) << 10) |
                 *    (((g >> 3) & 0x1f) << 5) |
                 *    ((b >> 3) & 0x1f);
                 */
                p = (((r >> (8 - PNG_QUANTIZE_RED_BITS)) & ((1 << PNG_QUANTIZE_RED_BITS) - 1))
                    << (PNG_QUANTIZE_GREEN_BITS + PNG_QUANTIZE_BLUE_BITS))
                    | (((g >> (8 - PNG_QUANTIZE_GREEN_BITS))
                        & ((1 << PNG_QUANTIZE_GREEN_BITS) - 1))
                        << (PNG_QUANTIZE_BLUE_BITS))
                    | ((b >> (8 - PNG_QUANTIZE_BLUE_BITS)) & ((1 << PNG_QUANTIZE_BLUE_BITS) - 1));

                *dp = *palette_lookup.add(p as usize);
                dp = dp.add(1);
                i = i.wrapping_add(1);
            }

            (*row_info).color_type = PNG_COLOR_TYPE_PALETTE as png_byte;
            (*row_info).channels = 1;
            (*row_info).pixel_depth = (*row_info).bit_depth;
            (*row_info).rowbytes = PNG_ROWBYTES((*row_info).pixel_depth as u32, row_width);
        } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
            && !palette_lookup.is_null()
        {
            let mut r: c_int;
            let mut g: c_int;
            let mut b: c_int;
            let mut p: c_int;
            sp = row;
            dp = row;
            i = 0;
            while i < row_width {
                r = *sp as c_int;
                sp = sp.add(1);
                g = *sp as c_int;
                sp = sp.add(1);
                b = *sp as c_int;
                sp = sp.add(1);
                sp = sp.add(1);

                p = (((r >> (8 - PNG_QUANTIZE_RED_BITS)) & ((1 << PNG_QUANTIZE_RED_BITS) - 1))
                    << (PNG_QUANTIZE_GREEN_BITS + PNG_QUANTIZE_BLUE_BITS))
                    | (((g >> (8 - PNG_QUANTIZE_GREEN_BITS))
                        & ((1 << PNG_QUANTIZE_GREEN_BITS) - 1))
                        << (PNG_QUANTIZE_BLUE_BITS))
                    | ((b >> (8 - PNG_QUANTIZE_BLUE_BITS)) & ((1 << PNG_QUANTIZE_BLUE_BITS) - 1));

                *dp = *palette_lookup.add(p as usize);
                dp = dp.add(1);
                i = i.wrapping_add(1);
            }

            (*row_info).color_type = PNG_COLOR_TYPE_PALETTE as png_byte;
            (*row_info).channels = 1;
            (*row_info).pixel_depth = (*row_info).bit_depth;
            (*row_info).rowbytes = PNG_ROWBYTES((*row_info).pixel_depth as u32, row_width);
        } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_PALETTE
            && !quantize_lookup.is_null()
        {
            sp = row;

            i = 0;
            while i < row_width {
                *sp = *quantize_lookup.add(*sp as usize);

                i = i.wrapping_add(1);
                sp = sp.add(1);
            }
        }
    }
}

/* Transform the row.  The order of transformations is significant,
 * and is very touchy.  If you add a transformation, take care to
 * decide how it fits in with the other transformations here.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_do_read_transformations(
    png_ptr: png_structrp,
    row_info: png_row_infop,
) {
    if (*png_ptr).row_buf.is_null() {
        /* Prior to 1.5.4 this output row/pass where the NULL pointer is, but this
         * error is incredibly rare and incredibly easy to debug without this
         * information.
         */
        png_error(png_ptr, c"NULL row buffer".as_ptr());
    }

    /* The following is debugging; prior to 1.5.4 the code was never compiled in;
     * in 1.5.4 PNG_FLAG_DETECT_UNINITIALIZED was added and the macro
     * PNG_WARN_UNINITIALIZED_ROW removed.  In 1.6 the new flag is set only for
     * all transformations, however in practice the ROW_INIT always gets done on
     * demand, if necessary.
     */
    if ((*png_ptr).flags & PNG_FLAG_DETECT_UNINITIALIZED) != 0
        && ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0
    {
        /* Application has failed to call either png_read_start_image() or
         * png_read_update_info() after setting transforms that expand pixels.
         * This check added to libpng-1.2.19 (but not enabled until 1.5.4).
         */
        png_error(png_ptr, c"Uninitialized row".as_ptr());
    }

    if ((*png_ptr).transformations & PNG_EXPAND) != 0 {
        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            png_do_expand_palette(
                png_ptr,
                row_info,
                (*png_ptr).row_buf.add(1),
                (*png_ptr).palette,
                (*png_ptr).trans_alpha,
                (*png_ptr).num_trans as c_int,
            );
        } else {
            if (*png_ptr).num_trans != 0 && ((*png_ptr).transformations & PNG_EXPAND_tRNS) != 0 {
                png_do_expand(
                    row_info,
                    (*png_ptr).row_buf.add(1),
                    core::ptr::addr_of!((*png_ptr).trans_color),
                );
            } else {
                png_do_expand(row_info, (*png_ptr).row_buf.add(1), core::ptr::null());
            }
        }
    }

    if ((*png_ptr).transformations & PNG_STRIP_ALPHA) != 0
        && ((*png_ptr).transformations & PNG_COMPOSE) == 0
        && ((*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
            || (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA)
    {
        png_do_strip_channel(
            row_info,
            (*png_ptr).row_buf.add(1),
            0, /* at_start == false, because SWAP_ALPHA happens later */
        );
    }

    if ((*png_ptr).transformations & PNG_RGB_TO_GRAY) != 0 {
        let rgb_error: c_int = png_do_rgb_to_gray(png_ptr, row_info, (*png_ptr).row_buf.add(1));

        if rgb_error != 0 {
            (*png_ptr).rgb_to_gray_status = 1;
            if ((*png_ptr).transformations & PNG_RGB_TO_GRAY) == PNG_RGB_TO_GRAY_WARN {
                png_warning(
                    png_ptr,
                    c"png_do_rgb_to_gray found nongray pixel".as_ptr(),
                );
            }

            if ((*png_ptr).transformations & PNG_RGB_TO_GRAY) == PNG_RGB_TO_GRAY_ERR {
                png_error(png_ptr, c"png_do_rgb_to_gray found nongray pixel".as_ptr());
            }
        }
    }

    /* From Andreas Dilger e-mail to png-implement, 26 March 1998:
     *
     *   In most cases, the "simple transparency" should be done prior to doing
     *   gray-to-RGB, or you will have to test 3x as many bytes to check if a
     *   pixel is transparent.  You would also need to make sure that the
     *   transparency information is upgraded to RGB.
     *
     *   To summarize, the current flow is:
     *   - Gray + simple transparency -> compare 1 or 2 gray bytes and composite
     *                                   with background "in place" if transparent,
     *                                   convert to RGB if necessary
     *   - Gray + alpha -> composite with gray background and remove alpha bytes,
     *                                   convert to RGB if necessary
     *
     *   To support RGB backgrounds for gray images we need:
     *   - Gray + simple transparency -> convert to RGB + simple transparency,
     *                                   compare 3 or 6 bytes and composite with
     *                                   background "in place" if transparent
     *                                   (3x compare/pixel compared to doing
     *                                   composite with gray bkgrnd)
     *   - Gray + alpha -> convert to RGB + alpha, composite with background and
     *                                   remove alpha bytes (3x float
     *                                   operations/pixel compared with composite
     *                                   on gray background)
     *
     *  Greg's change will do this.  The reason it wasn't done before is for
     *  performance, as this increases the per-pixel operations.  If we would check
     *  in advance if the background was gray or RGB, and position the gray-to-RGB
     *  transform appropriately, then it would save a lot of work/time.
     */

    /* If gray -> RGB, do so now only if background is non-gray; else do later
     * for performance reasons
     */
    if ((*png_ptr).transformations & PNG_GRAY_TO_RGB) != 0
        && ((*png_ptr).mode & PNG_BACKGROUND_IS_GRAY) == 0
    {
        png_do_gray_to_rgb(row_info, (*png_ptr).row_buf.add(1));
    }

    if ((*png_ptr).transformations & PNG_COMPOSE) != 0 {
        png_do_compose(row_info, (*png_ptr).row_buf.add(1), png_ptr);
    }

    if ((*png_ptr).transformations & PNG_GAMMA) != 0
        /* Because RGB_TO_GRAY does the gamma transform. */
        && ((*png_ptr).transformations & PNG_RGB_TO_GRAY) == 0
        /* Because PNG_COMPOSE does the gamma transform if there is something to
         * do (if there is an alpha channel or transparency.)
         */
        && !(((*png_ptr).transformations & PNG_COMPOSE) != 0
            && (((*png_ptr).num_trans != 0)
                || ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0))
        /* Because png_init_read_transformations transforms the palette, unless
         * RGB_TO_GRAY will do the transform.
         */
        && ((*png_ptr).color_type as c_int != PNG_COLOR_TYPE_PALETTE)
    {
        png_do_gamma(row_info, (*png_ptr).row_buf.add(1), png_ptr);
    }

    if ((*png_ptr).transformations & PNG_STRIP_ALPHA) != 0
        && ((*png_ptr).transformations & PNG_COMPOSE) != 0
        && ((*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
            || (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA)
    {
        png_do_strip_channel(
            row_info,
            (*png_ptr).row_buf.add(1),
            0, /* at_start == false, because SWAP_ALPHA happens later */
        );
    }

    if ((*png_ptr).transformations & PNG_ENCODE_ALPHA) != 0
        && ((*row_info).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0
    {
        png_do_encode_alpha(row_info, (*png_ptr).row_buf.add(1), png_ptr);
    }

    if ((*png_ptr).transformations & PNG_SCALE_16_TO_8) != 0 {
        png_do_scale_16_to_8(row_info, (*png_ptr).row_buf.add(1));
    }

    /* There is no harm in doing both of these because only one has any effect,
     * by putting the 'scale' option first if the app asks for scale (either by
     * calling the API or in a TRANSFORM flag) this is what happens.
     */
    if ((*png_ptr).transformations & PNG_16_TO_8) != 0 {
        png_do_chop(row_info, (*png_ptr).row_buf.add(1));
    }

    if ((*png_ptr).transformations & PNG_QUANTIZE) != 0 {
        png_do_quantize(
            row_info,
            (*png_ptr).row_buf.add(1),
            (*png_ptr).palette_lookup,
            (*png_ptr).quantize_index,
        );
    }

    /* Do the expansion now, after all the arithmetic has been done.  Notice
     * that previous transformations can handle the PNG_EXPAND_16 flag if this
     * is efficient (particularly true in the case of gamma correction, where
     * better accuracy results faster!)
     */
    if ((*png_ptr).transformations & PNG_EXPAND_16) != 0 {
        png_do_expand_16(row_info, (*png_ptr).row_buf.add(1));
    }

    /* NOTE: moved here in 1.5.4 (from much later in this list.) */
    if ((*png_ptr).transformations & PNG_GRAY_TO_RGB) != 0
        && ((*png_ptr).mode & PNG_BACKGROUND_IS_GRAY) != 0
    {
        png_do_gray_to_rgb(row_info, (*png_ptr).row_buf.add(1));
    }

    if ((*png_ptr).transformations & PNG_INVERT_MONO) != 0 {
        png_do_invert(row_info, (*png_ptr).row_buf.add(1));
    }

    if ((*png_ptr).transformations & PNG_INVERT_ALPHA) != 0 {
        png_do_read_invert_alpha(row_info, (*png_ptr).row_buf.add(1));
    }

    if ((*png_ptr).transformations & PNG_SHIFT) != 0 {
        png_do_unshift(
            row_info,
            (*png_ptr).row_buf.add(1),
            core::ptr::addr_of!((*png_ptr).shift),
        );
    }

    if ((*png_ptr).transformations & PNG_PACK) != 0 {
        png_do_unpack(row_info, (*png_ptr).row_buf.add(1));
    }

    /* Added at libpng-1.5.10 */
    if (*row_info).color_type as c_int == PNG_COLOR_TYPE_PALETTE
        && (*png_ptr).num_palette_max >= 0
    {
        png_do_check_palette_indexes(png_ptr, row_info);
    }

    if ((*png_ptr).transformations & PNG_BGR) != 0 {
        png_do_bgr(row_info, (*png_ptr).row_buf.add(1));
    }

    if ((*png_ptr).transformations & PNG_PACKSWAP) != 0 {
        png_do_packswap(row_info, (*png_ptr).row_buf.add(1));
    }

    if ((*png_ptr).transformations & PNG_FILLER) != 0 {
        png_do_read_filler(
            row_info,
            (*png_ptr).row_buf.add(1),
            (*png_ptr).filler as png_uint_32,
            (*png_ptr).flags,
        );
    }

    if ((*png_ptr).transformations & PNG_SWAP_ALPHA) != 0 {
        png_do_read_swap_alpha(row_info, (*png_ptr).row_buf.add(1));
    }

    if ((*png_ptr).transformations & PNG_SWAP_BYTES) != 0 {
        png_do_swap(row_info, (*png_ptr).row_buf.add(1));
    }

    if ((*png_ptr).transformations & PNG_USER_TRANSFORM) != 0 {
        if (*png_ptr).read_user_transform_fn.is_some() {
            ((*png_ptr).read_user_transform_fn.unwrap())(
                /* User read transform function */
                png_ptr,  /* png_ptr */
                row_info, /* row_info: */
                /*  png_uint_32 width;       width of row */
                /*  size_t rowbytes;         number of bytes in row */
                /*  png_byte color_type;     color type of pixels */
                /*  png_byte bit_depth;      bit depth of samples */
                /*  png_byte channels;       number of channels (1-4) */
                /*  png_byte pixel_depth;    bits per pixel (depth*channels) */
                (*png_ptr).row_buf.add(1),
            ); /* start of pixel data for row */
        }
        if (*png_ptr).user_transform_depth != 0 {
            (*row_info).bit_depth = (*png_ptr).user_transform_depth;
        }

        if (*png_ptr).user_transform_channels != 0 {
            (*row_info).channels = (*png_ptr).user_transform_channels;
        }
        (*row_info).pixel_depth =
            (((*row_info).bit_depth as c_int) * ((*row_info).channels as c_int)) as png_byte;

        (*row_info).rowbytes = PNG_ROWBYTES((*row_info).pixel_depth as u32, (*row_info).width);
    }
}
