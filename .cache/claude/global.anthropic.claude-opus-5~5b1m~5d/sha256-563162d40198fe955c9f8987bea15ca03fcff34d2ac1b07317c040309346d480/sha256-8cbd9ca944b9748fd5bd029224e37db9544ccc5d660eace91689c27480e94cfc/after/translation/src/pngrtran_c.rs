//! pngrtran.c lines 2293-3333: the low-level read row transforms
//! (`png_do_unpack`, `png_do_unshift`, `png_do_scale_16_to_8`, `png_do_chop`,
//! `png_do_read_swap_alpha`, `png_do_read_invert_alpha`, `png_do_read_filler`,
//! `png_do_gray_to_rgb`, `png_do_rgb_to_gray`).
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

/* Unpack pixels of 1, 2, or 4 bits per pixel into 1 byte per pixel,
 * without changing the actual values.  Thus, if you had a row with
 * a bit depth of 1, you would end up with bytes that only contained
 * the numbers 0 or 1.  If you would rather they contain 0 and 255, use
 * png_do_shift() after this.
 */
pub unsafe fn png_do_unpack(row_info: png_row_infop, row: png_bytep) {
    if (*row_info).bit_depth < 8 {
        let mut i: png_uint_32;
        let row_width: png_uint_32 = (*row_info).width;

        match (*row_info).bit_depth as c_int {
            1 => {
                let mut sp: png_bytep =
                    row.wrapping_add(((row_width.wrapping_sub(1)) >> 3) as usize);
                let mut dp: png_bytep = row
                    .wrapping_add(row_width as usize)
                    .wrapping_sub(1);
                let mut shift: png_uint_32 = 7u32.wrapping_sub((row_width.wrapping_add(7)) & 0x07);
                i = 0;
                while i < row_width {
                    *dp = (((*sp as c_int) >> shift) & 0x01) as png_byte;

                    if shift == 7 {
                        shift = 0;
                        sp = sp.wrapping_sub(1);
                    } else {
                        shift += 1;
                    }

                    dp = dp.wrapping_sub(1);

                    i += 1;
                }
            }

            2 => {
                let mut sp: png_bytep =
                    row.wrapping_add(((row_width.wrapping_sub(1)) >> 2) as usize);
                let mut dp: png_bytep = row
                    .wrapping_add(row_width as usize)
                    .wrapping_sub(1);
                let mut shift: png_uint_32 =
                    (3u32.wrapping_sub((row_width.wrapping_add(3)) & 0x03)) << 1;
                i = 0;
                while i < row_width {
                    *dp = (((*sp as c_int) >> shift) & 0x03) as png_byte;

                    if shift == 6 {
                        shift = 0;
                        sp = sp.wrapping_sub(1);
                    } else {
                        shift += 2;
                    }

                    dp = dp.wrapping_sub(1);

                    i += 1;
                }
            }

            4 => {
                let mut sp: png_bytep =
                    row.wrapping_add(((row_width.wrapping_sub(1)) >> 1) as usize);
                let mut dp: png_bytep = row
                    .wrapping_add(row_width as usize)
                    .wrapping_sub(1);
                let mut shift: png_uint_32 =
                    (1u32.wrapping_sub((row_width.wrapping_add(1)) & 0x01)) << 2;
                i = 0;
                while i < row_width {
                    *dp = (((*sp as c_int) >> shift) & 0x0f) as png_byte;

                    if shift == 4 {
                        shift = 0;
                        sp = sp.wrapping_sub(1);
                    } else {
                        shift = 4;
                    }

                    dp = dp.wrapping_sub(1);

                    i += 1;
                }
            }

            _ => {}
        }
        (*row_info).bit_depth = 8;
        (*row_info).pixel_depth = (8 * ((*row_info).channels as c_int)) as png_byte;
        (*row_info).rowbytes = (row_width as usize).wrapping_mul((*row_info).channels as usize);
    }
}

/* Reverse the effects of png_do_shift.  This routine merely shifts the
 * pixels back to their significant bits values.  Thus, if you have
 * a row of bit depth 8, but only 5 are significant, this will shift
 * the values back to 0 through 31.
 */
pub unsafe fn png_do_unshift(
    row_info: png_row_infop,
    row: png_bytep,
    sig_bits: png_const_color_8p,
) {
    let color_type: c_int;

    /* The palette case has already been handled in the _init routine. */
    color_type = (*row_info).color_type as c_int;

    if color_type != PNG_COLOR_TYPE_PALETTE {
        let mut shift: [c_int; 4] = [0; 4];
        let mut channels: c_int = 0;
        let bit_depth: c_int = (*row_info).bit_depth as c_int;

        if (color_type & PNG_COLOR_MASK_COLOR) != 0 {
            shift[channels as usize] = bit_depth - ((*sig_bits).red as c_int);
            channels += 1;
            shift[channels as usize] = bit_depth - ((*sig_bits).green as c_int);
            channels += 1;
            shift[channels as usize] = bit_depth - ((*sig_bits).blue as c_int);
            channels += 1;
        } else {
            shift[channels as usize] = bit_depth - ((*sig_bits).gray as c_int);
            channels += 1;
        }

        if (color_type & PNG_COLOR_MASK_ALPHA) != 0 {
            shift[channels as usize] = bit_depth - ((*sig_bits).alpha as c_int);
            channels += 1;
        }

        {
            let mut c: c_int;
            let mut have_shift: c_int;

            have_shift = 0;
            c = 0;
            while c < channels {
                /* A shift of more than the bit depth is an error condition but it
                 * gets ignored here.
                 */
                if shift[c as usize] <= 0 || shift[c as usize] >= bit_depth {
                    shift[c as usize] = 0;
                } else {
                    have_shift = 1;
                }

                c += 1;
            }

            if have_shift == 0 {
                return;
            }
        }

        match bit_depth {
            /* Must be 2bpp gray */
            /* assert(channels == 1 && shift[0] == 1) */
            2 => {
                let mut bp: png_bytep = row;
                let bp_end: png_bytep = bp.wrapping_add((*row_info).rowbytes);

                while bp < bp_end {
                    let b: c_int = ((*bp as c_int) >> 1) & 0x55;
                    *bp = b as png_byte;
                    bp = bp.wrapping_add(1);
                }
            }

            /* Must be 4bpp gray */
            /* assert(channels == 1) */
            4 => {
                let mut bp: png_bytep = row;
                let bp_end: png_bytep = bp.wrapping_add((*row_info).rowbytes);
                let gray_shift: c_int = shift[0];
                let mut mask: c_int = 0xf >> gray_shift;

                mask |= mask << 4;

                while bp < bp_end {
                    let b: c_int = ((*bp as c_int) >> gray_shift) & mask;
                    *bp = b as png_byte;
                    bp = bp.wrapping_add(1);
                }
            }

            /* Single byte components, G, GA, RGB, RGBA */
            8 => {
                let mut bp: png_bytep = row;
                let bp_end: png_bytep = bp.wrapping_add((*row_info).rowbytes);
                let mut channel: c_int = 0;

                while bp < bp_end {
                    let b: c_int = (*bp as c_int) >> shift[channel as usize];
                    channel += 1;
                    if channel >= channels {
                        channel = 0;
                    }
                    *bp = b as png_byte;
                    bp = bp.wrapping_add(1);
                }
            }

            /* Double byte components, G, GA, RGB, RGBA */
            16 => {
                let mut bp: png_bytep = row;
                let bp_end: png_bytep = bp.wrapping_add((*row_info).rowbytes);
                let mut channel: c_int = 0;

                while bp < bp_end {
                    let mut value: c_int =
                        ((*bp.wrapping_add(0) as c_int) << 8) + (*bp.wrapping_add(1) as c_int);

                    value >>= shift[channel as usize];
                    channel += 1;
                    if channel >= channels {
                        channel = 0;
                    }
                    *bp = (value >> 8) as png_byte;
                    bp = bp.wrapping_add(1);
                    *bp = value as png_byte;
                    bp = bp.wrapping_add(1);
                }
            }

            _ => {
                /* Must be 1bpp gray: should not be here! */
                /* NOTREACHED */
            }
        }
    }
}

/* Scale rows of bit depth 16 down to 8 accurately */
pub unsafe fn png_do_scale_16_to_8(row_info: png_row_infop, row: png_bytep) {
    if (*row_info).bit_depth == 16 {
        let mut sp: png_bytep = row; /* source */
        let mut dp: png_bytep = row; /* destination */
        let ep: png_bytep = sp.wrapping_add((*row_info).rowbytes); /* end+1 */

        while sp < ep {
            /* The input is an array of 16-bit components, these must be scaled to
             * 8 bits each.  For a 16-bit value V the required value (from the PNG
             * specification) is:
             *
             *    (V * 255) / 65535
             *
             * This reduces to round(V / 257), or floor((V + 128.5)/257)
             *
             * Represent V as the two byte value vhi.vlo.  Make a guess that the
             * result is the top byte of V, vhi, then the correction to this value
             * is:
             *
             *    error = floor(((V-vhi.vhi) + 128.5) / 257)
             *          = floor(((vlo-vhi) + 128.5) / 257)
             *
             * This can be approximated using integer arithmetic (and a signed
             * shift):
             *
             *    error = (vlo-vhi+128) >> 8;
             *
             * The approximate differs from the exact answer only when (vlo-vhi) is
             * 128; it then gives a correction of +1 when the exact correction is
             * 0.  This gives 128 errors.  The exact answer (correct for all 16-bit
             * input values) is:
             *
             *    error = (vlo-vhi+128)*65535 >> 24;
             *
             * An alternative arithmetic calculation which also gives no errors is:
             *
             *    (V * 255 + 32895) >> 16
             */

            let mut tmp: png_int_32 = *sp as png_int_32; /* must be signed! */
            sp = sp.wrapping_add(1);
            let lo: c_int = *sp as c_int;
            sp = sp.wrapping_add(1);
            tmp = tmp.wrapping_add(
                (lo.wrapping_sub(tmp).wrapping_add(128).wrapping_mul(65535)) >> 24,
            );
            *dp = tmp as png_byte;
            dp = dp.wrapping_add(1);
        }

        (*row_info).bit_depth = 8;
        (*row_info).pixel_depth = (8 * ((*row_info).channels as c_int)) as png_byte;
        (*row_info).rowbytes =
            ((*row_info).width as usize).wrapping_mul((*row_info).channels as usize);
    }
}

/* Simply discard the low byte.  This was the default behavior prior
 * to libpng-1.5.4.
 */
pub unsafe fn png_do_chop(row_info: png_row_infop, row: png_bytep) {
    if (*row_info).bit_depth == 16 {
        let mut sp: png_bytep = row; /* source */
        let mut dp: png_bytep = row; /* destination */
        let ep: png_bytep = sp.wrapping_add((*row_info).rowbytes); /* end+1 */

        while sp < ep {
            *dp = *sp;
            dp = dp.wrapping_add(1);
            sp = sp.wrapping_add(2); /* skip low byte */
        }

        (*row_info).bit_depth = 8;
        (*row_info).pixel_depth = (8 * ((*row_info).channels as c_int)) as png_byte;
        (*row_info).rowbytes =
            ((*row_info).width as usize).wrapping_mul((*row_info).channels as usize);
    }
}

pub unsafe fn png_do_read_swap_alpha(row_info: png_row_infop, row: png_bytep) {
    let row_width: png_uint_32 = (*row_info).width;

    if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
        /* This converts from RGBA to ARGB */
        if (*row_info).bit_depth == 8 {
            let mut sp: png_bytep = row.wrapping_add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;
            let mut save: png_byte;
            let mut i: png_uint_32;

            i = 0;
            while i < row_width {
                sp = sp.wrapping_sub(1);
                save = *sp;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = *sp;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = *sp;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = *sp;
                dp = dp.wrapping_sub(1);
                *dp = save;

                i += 1;
            }
        }
        /* This converts from RRGGBBAA to AARRGGBB */
        else {
            let mut sp: png_bytep = row.wrapping_add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;
            let mut save: [png_byte; 2] = [0; 2];
            let mut i: png_uint_32;

            i = 0;
            while i < row_width {
                sp = sp.wrapping_sub(1);
                save[0] = *sp;
                sp = sp.wrapping_sub(1);
                save[1] = *sp;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = *sp;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = *sp;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = *sp;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = *sp;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = *sp;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = *sp;
                dp = dp.wrapping_sub(1);
                *dp = save[0];
                dp = dp.wrapping_sub(1);
                *dp = save[1];

                i += 1;
            }
        }
    } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
        /* This converts from GA to AG */
        if (*row_info).bit_depth == 8 {
            let mut sp: png_bytep = row.wrapping_add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;
            let mut save: png_byte;
            let mut i: png_uint_32;

            i = 0;
            while i < row_width {
                sp = sp.wrapping_sub(1);
                save = *sp;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = *sp;
                dp = dp.wrapping_sub(1);
                *dp = save;

                i += 1;
            }
        }
        /* This converts from GGAA to AAGG */
        else {
            let mut sp: png_bytep = row.wrapping_add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;
            let mut save: [png_byte; 2] = [0; 2];
            let mut i: png_uint_32;

            i = 0;
            while i < row_width {
                sp = sp.wrapping_sub(1);
                save[0] = *sp;
                sp = sp.wrapping_sub(1);
                save[1] = *sp;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = *sp;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = *sp;
                dp = dp.wrapping_sub(1);
                *dp = save[0];
                dp = dp.wrapping_sub(1);
                *dp = save[1];

                i += 1;
            }
        }
    }
}

pub unsafe fn png_do_read_invert_alpha(row_info: png_row_infop, row: png_bytep) {
    let row_width: png_uint_32;

    row_width = (*row_info).width;
    if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
        if (*row_info).bit_depth == 8 {
            /* This inverts the alpha channel in RGBA */
            let mut sp: png_bytep = row.wrapping_add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;
            let mut i: png_uint_32;

            i = 0;
            while i < row_width {
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = (255 - (*sp as c_int)) as png_byte;

                /*          This does nothing:
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                            We can replace it with:
                */
                sp = sp.wrapping_sub(3);
                dp = sp;

                i += 1;
            }
        }
        /* This inverts the alpha channel in RRGGBBAA */
        else {
            let mut sp: png_bytep = row.wrapping_add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;
            let mut i: png_uint_32;

            i = 0;
            while i < row_width {
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = (255 - (*sp as c_int)) as png_byte;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = (255 - (*sp as c_int)) as png_byte;

                /*          This does nothing:
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                            We can replace it with:
                */
                sp = sp.wrapping_sub(6);
                dp = sp;

                i += 1;
            }
        }
    } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
        if (*row_info).bit_depth == 8 {
            /* This inverts the alpha channel in GA */
            let mut sp: png_bytep = row.wrapping_add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;
            let mut i: png_uint_32;

            i = 0;
            while i < row_width {
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = (255 - (*sp as c_int)) as png_byte;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = *sp;

                i += 1;
            }
        } else {
            /* This inverts the alpha channel in GGAA */
            let mut sp: png_bytep = row.wrapping_add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;
            let mut i: png_uint_32;

            i = 0;
            while i < row_width {
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = (255 - (*sp as c_int)) as png_byte;
                dp = dp.wrapping_sub(1);
                sp = sp.wrapping_sub(1);
                *dp = (255 - (*sp as c_int)) as png_byte;
                /*
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                */
                sp = sp.wrapping_sub(2);
                dp = sp;

                i += 1;
            }
        }
    }
}

/* Add filler channel if we have RGB color */
pub unsafe fn png_do_read_filler(
    row_info: png_row_infop,
    row: png_bytep,
    filler: png_uint_32,
    flags: png_uint_32,
) {
    let mut i: png_uint_32;
    let row_width: png_uint_32 = (*row_info).width;

    let hi_filler: png_byte = (filler >> 8) as png_byte;
    let lo_filler: png_byte = filler as png_byte;

    if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY {
        if (*row_info).bit_depth == 8 {
            if (flags & PNG_FLAG_FILLER_AFTER) != 0 {
                /* This changes the data from G to GX */
                let mut sp: png_bytep = row.wrapping_add(row_width as usize);
                let mut dp: png_bytep = sp.wrapping_add(row_width as usize);
                i = 1;
                while i < row_width {
                    dp = dp.wrapping_sub(1);
                    *dp = lo_filler;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;

                    i += 1;
                }
                dp = dp.wrapping_sub(1);
                *dp = lo_filler;
                (*row_info).channels = 2;
                (*row_info).pixel_depth = 16;
                (*row_info).rowbytes = (row_width as usize).wrapping_mul(2);
            } else {
                /* This changes the data from G to XG */
                let mut sp: png_bytep = row.wrapping_add(row_width as usize);
                let mut dp: png_bytep = sp.wrapping_add(row_width as usize);
                i = 0;
                while i < row_width {
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    *dp = lo_filler;

                    i += 1;
                }
                (*row_info).channels = 2;
                (*row_info).pixel_depth = 16;
                (*row_info).rowbytes = (row_width as usize).wrapping_mul(2);
            }
        } else if (*row_info).bit_depth == 16 {
            if (flags & PNG_FLAG_FILLER_AFTER) != 0 {
                /* This changes the data from GG to GGXX */
                let mut sp: png_bytep = row.wrapping_add((row_width as usize).wrapping_mul(2));
                let mut dp: png_bytep = sp.wrapping_add((row_width as usize).wrapping_mul(2));
                i = 1;
                while i < row_width {
                    dp = dp.wrapping_sub(1);
                    *dp = lo_filler;
                    dp = dp.wrapping_sub(1);
                    *dp = hi_filler;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;

                    i += 1;
                }
                dp = dp.wrapping_sub(1);
                *dp = lo_filler;
                dp = dp.wrapping_sub(1);
                *dp = hi_filler;
                (*row_info).channels = 2;
                (*row_info).pixel_depth = 32;
                (*row_info).rowbytes = (row_width as usize).wrapping_mul(4);
            } else {
                /* This changes the data from GG to XXGG */
                let mut sp: png_bytep = row.wrapping_add((row_width as usize).wrapping_mul(2));
                let mut dp: png_bytep = sp.wrapping_add((row_width as usize).wrapping_mul(2));
                i = 0;
                while i < row_width {
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    *dp = lo_filler;
                    dp = dp.wrapping_sub(1);
                    *dp = hi_filler;

                    i += 1;
                }
                (*row_info).channels = 2;
                (*row_info).pixel_depth = 32;
                (*row_info).rowbytes = (row_width as usize).wrapping_mul(4);
            }
        }
    }
    /* COLOR_TYPE == GRAY */
    else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB {
        if (*row_info).bit_depth == 8 {
            if (flags & PNG_FLAG_FILLER_AFTER) != 0 {
                /* This changes the data from RGB to RGBX */
                let mut sp: png_bytep = row.wrapping_add((row_width as usize).wrapping_mul(3));
                let mut dp: png_bytep = sp.wrapping_add(row_width as usize);
                i = 1;
                while i < row_width {
                    dp = dp.wrapping_sub(1);
                    *dp = lo_filler;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;

                    i += 1;
                }
                dp = dp.wrapping_sub(1);
                *dp = lo_filler;
                (*row_info).channels = 4;
                (*row_info).pixel_depth = 32;
                (*row_info).rowbytes = (row_width as usize).wrapping_mul(4);
            } else {
                /* This changes the data from RGB to XRGB */
                let mut sp: png_bytep = row.wrapping_add((row_width as usize).wrapping_mul(3));
                let mut dp: png_bytep = sp.wrapping_add(row_width as usize);
                i = 0;
                while i < row_width {
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    *dp = lo_filler;

                    i += 1;
                }
                (*row_info).channels = 4;
                (*row_info).pixel_depth = 32;
                (*row_info).rowbytes = (row_width as usize).wrapping_mul(4);
            }
        } else if (*row_info).bit_depth == 16 {
            if (flags & PNG_FLAG_FILLER_AFTER) != 0 {
                /* This changes the data from RRGGBB to RRGGBBXX */
                let mut sp: png_bytep = row.wrapping_add((row_width as usize).wrapping_mul(6));
                let mut dp: png_bytep = sp.wrapping_add((row_width as usize).wrapping_mul(2));
                i = 1;
                while i < row_width {
                    dp = dp.wrapping_sub(1);
                    *dp = lo_filler;
                    dp = dp.wrapping_sub(1);
                    *dp = hi_filler;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;

                    i += 1;
                }
                dp = dp.wrapping_sub(1);
                *dp = lo_filler;
                dp = dp.wrapping_sub(1);
                *dp = hi_filler;
                (*row_info).channels = 4;
                (*row_info).pixel_depth = 64;
                (*row_info).rowbytes = (row_width as usize).wrapping_mul(8);
            } else {
                /* This changes the data from RRGGBB to XXRRGGBB */
                let mut sp: png_bytep = row.wrapping_add((row_width as usize).wrapping_mul(6));
                let mut dp: png_bytep = sp.wrapping_add((row_width as usize).wrapping_mul(2));
                i = 0;
                while i < row_width {
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    *dp = lo_filler;
                    dp = dp.wrapping_sub(1);
                    *dp = hi_filler;

                    i += 1;
                }

                (*row_info).channels = 4;
                (*row_info).pixel_depth = 64;
                (*row_info).rowbytes = (row_width as usize).wrapping_mul(8);
            }
        }
    } /* COLOR_TYPE == RGB */
}

/* Expand grayscale files to RGB, with or without alpha */
pub unsafe fn png_do_gray_to_rgb(row_info: png_row_infop, row: png_bytep) {
    let mut i: png_uint_32;
    let row_width: png_uint_32 = (*row_info).width;

    if (*row_info).bit_depth >= 8 && ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0 {
        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY {
            if (*row_info).bit_depth == 8 {
                /* This changes G to RGB */
                let mut sp: png_bytep = row.wrapping_add(row_width as usize).wrapping_sub(1);
                let mut dp: png_bytep = sp.wrapping_add((row_width as usize).wrapping_mul(2));
                i = 0;
                while i < row_width {
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);

                    i += 1;
                }
            } else {
                /* This changes GG to RRGGBB */
                let mut sp: png_bytep = row
                    .wrapping_add((row_width as usize).wrapping_mul(2))
                    .wrapping_sub(1);
                let mut dp: png_bytep = sp.wrapping_add((row_width as usize).wrapping_mul(4));
                i = 0;
                while i < row_width {
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    *dp = *sp.wrapping_sub(1);
                    dp = dp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    *dp = *sp.wrapping_sub(1);
                    dp = dp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);

                    i += 1;
                }
            }
        } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
            if (*row_info).bit_depth == 8 {
                /* This changes GA to RGBA */
                let mut sp: png_bytep = row
                    .wrapping_add((row_width as usize).wrapping_mul(2))
                    .wrapping_sub(1);
                let mut dp: png_bytep = sp.wrapping_add((row_width as usize).wrapping_mul(2));
                i = 0;
                while i < row_width {
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);

                    i += 1;
                }
            } else {
                /* This changes GGAA to RRGGBBAA */
                let mut sp: png_bytep = row
                    .wrapping_add((row_width as usize).wrapping_mul(4))
                    .wrapping_sub(1);
                let mut dp: png_bytep = sp.wrapping_add((row_width as usize).wrapping_mul(4));
                i = 0;
                while i < row_width {
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    *dp = *sp.wrapping_sub(1);
                    dp = dp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    *dp = *sp.wrapping_sub(1);
                    dp = dp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);
                    *dp = *sp;
                    dp = dp.wrapping_sub(1);
                    sp = sp.wrapping_sub(1);

                    i += 1;
                }
            }
        }
        (*row_info).channels = (((*row_info).channels as c_int) + 2) as png_byte;
        (*row_info).color_type |= PNG_COLOR_MASK_COLOR as png_byte;
        (*row_info).pixel_depth =
            (((*row_info).channels as c_int) * ((*row_info).bit_depth as c_int)) as png_byte;
        (*row_info).rowbytes = PNG_ROWBYTES((*row_info).pixel_depth as u32, row_width);
    }
}

/* Reduce RGB files to grayscale, with or without alpha
 * using the equation given in Poynton's ColorFAQ of 1998-01-04 at
 * <http://www.inforamp.net/~poynton/>  (THIS LINK IS DEAD June 2008 but
 * versions dated 1998 through November 2002 have been archived at
 * https://web.archive.org/web/20000816232553/www.inforamp.net/
 * ~poynton/notes/colour_and_gamma/ColorFAQ.txt )
 * Charles Poynton poynton at poynton.com
 *
 *     Y = 0.212671 * R + 0.715160 * G + 0.072169 * B
 *
 *  which can be expressed with integers as
 *
 *     Y = (6969 * R + 23434 * G + 2365 * B)/32768
 *
 * Poynton's current link (as of January 2003 through July 2011):
 * <http://www.poynton.com/notes/colour_and_gamma/>
 * has changed the numbers slightly:
 *
 *     Y = 0.2126*R + 0.7152*G + 0.0722*B
 *
 *  which can be expressed with integers as
 *
 *     Y = (6966 * R + 23436 * G + 2366 * B)/32768
 *
 *  Historically, however, libpng uses numbers derived from the ITU-R Rec 709
 *  end point chromaticities and the D65 white point.  Depending on the
 *  precision used for the D65 white point this produces a variety of different
 *  numbers, however if the four decimal place value used in ITU-R Rec 709 is
 *  used (0.3127,0.3290) the Y calculation would be:
 *
 *     Y = (6968 * R + 23435 * G + 2366 * B)/32768
 *
 *  While this is correct the rounding results in an overflow for white, because
 *  the sum of the rounded coefficients is 32769, not 32768.  Consequently
 *  libpng uses, instead, the closest non-overflowing approximation:
 *
 *     Y = (6968 * R + 23434 * G + 2366 * B)/32768
 *
 *  Starting with libpng-1.5.5, if the image being converted has a cHRM chunk
 *  (including an sRGB chunk) then the chromaticities are used to calculate the
 *  coefficients.  See the chunk handling in pngrutil.c for more information.
 *
 *  In all cases the calculation is to be done in a linear colorspace.  If no
 *  gamma information is available to correct the encoding of the original RGB
 *  values this results in an implicit assumption that the original PNG RGB
 *  values were linear.
 *
 *  Other integer coefficients can be used via png_set_rgb_to_gray().  Because
 *  the API takes just red and green coefficients the blue coefficient is
 *  calculated to make the sum 32768.  This will result in different rounding
 *  to that used above.
 */
pub unsafe fn png_do_rgb_to_gray(
    png_ptr: png_structrp,
    row_info: png_row_infop,
    row: png_bytep,
) -> c_int {
    let mut rgb_error: c_int = 0;

    if ((*row_info).color_type as c_int & PNG_COLOR_MASK_PALETTE) == 0
        && ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0
    {
        let rc: png_uint_32 = (*png_ptr).rgb_to_gray_red_coeff as png_uint_32;
        let gc: png_uint_32 = (*png_ptr).rgb_to_gray_green_coeff as png_uint_32;
        let bc: png_uint_32 = 32768u32.wrapping_sub(rc).wrapping_sub(gc);
        let row_width: png_uint_32 = (*row_info).width;
        let have_alpha: c_int =
            (((*row_info).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0) as c_int;

        if (*row_info).bit_depth == 8 {
            /* Notice that gamma to/from 1 are not necessarily inverses (if
             * there is an overall gamma correction).  Prior to 1.5.5 this code
             * checked the linearized values for equality; this doesn't match
             * the documentation, the original values must be checked.
             */
            if !(*png_ptr).gamma_from_1.is_null() && !(*png_ptr).gamma_to_1.is_null() {
                let mut sp: png_bytep = row;
                let mut dp: png_bytep = row;
                let mut i: png_uint_32;

                i = 0;
                while i < row_width {
                    let mut red: png_byte = *sp;
                    sp = sp.wrapping_add(1);
                    let mut green: png_byte = *sp;
                    sp = sp.wrapping_add(1);
                    let mut blue: png_byte = *sp;
                    sp = sp.wrapping_add(1);

                    if red != green || red != blue {
                        red = *(*png_ptr).gamma_to_1.wrapping_add(red as usize);
                        green = *(*png_ptr).gamma_to_1.wrapping_add(green as usize);
                        blue = *(*png_ptr).gamma_to_1.wrapping_add(blue as usize);

                        rgb_error |= 1;
                        *dp = *(*png_ptr).gamma_from_1.wrapping_add(
                            ((rc.wrapping_mul(red as png_uint_32))
                                .wrapping_add(gc.wrapping_mul(green as png_uint_32))
                                .wrapping_add(bc.wrapping_mul(blue as png_uint_32))
                                .wrapping_add(16384)
                                >> 15) as usize,
                        );
                        dp = dp.wrapping_add(1);
                    } else {
                        /* If there is no overall correction the table will not be
                         * set.
                         */
                        if !(*png_ptr).gamma_table.is_null() {
                            red = *(*png_ptr).gamma_table.wrapping_add(red as usize);
                        }

                        *dp = red;
                        dp = dp.wrapping_add(1);
                    }

                    if have_alpha != 0 {
                        *dp = *sp;
                        dp = dp.wrapping_add(1);
                        sp = sp.wrapping_add(1);
                    }

                    i += 1;
                }
            } else {
                let mut sp: png_bytep = row;
                let mut dp: png_bytep = row;
                let mut i: png_uint_32;

                i = 0;
                while i < row_width {
                    let red: png_byte = *sp;
                    sp = sp.wrapping_add(1);
                    let green: png_byte = *sp;
                    sp = sp.wrapping_add(1);
                    let blue: png_byte = *sp;
                    sp = sp.wrapping_add(1);

                    if red != green || red != blue {
                        rgb_error |= 1;
                        /* NOTE: this is the historical approach which simply
                         * truncates the results.
                         */
                        *dp = ((rc.wrapping_mul(red as png_uint_32))
                            .wrapping_add(gc.wrapping_mul(green as png_uint_32))
                            .wrapping_add(bc.wrapping_mul(blue as png_uint_32))
                            >> 15) as png_byte;
                        dp = dp.wrapping_add(1);
                    } else {
                        *dp = red;
                        dp = dp.wrapping_add(1);
                    }

                    if have_alpha != 0 {
                        *dp = *sp;
                        dp = dp.wrapping_add(1);
                        sp = sp.wrapping_add(1);
                    }

                    i += 1;
                }
            }
        } else
        /* RGB bit_depth == 16 */
        {
            if !(*png_ptr).gamma_16_to_1.is_null() && !(*png_ptr).gamma_16_from_1.is_null() {
                let mut sp: png_bytep = row;
                let mut dp: png_bytep = row;
                let mut i: png_uint_32;

                i = 0;
                while i < row_width {
                    let red: png_uint_16;
                    let green: png_uint_16;
                    let blue: png_uint_16;
                    let w: png_uint_16;
                    let mut hi: png_byte;
                    let mut lo: png_byte;

                    hi = *sp;
                    sp = sp.wrapping_add(1);
                    lo = *sp;
                    sp = sp.wrapping_add(1);
                    red = (((hi as c_int) << 8) | (lo as c_int)) as png_uint_16;
                    hi = *sp;
                    sp = sp.wrapping_add(1);
                    lo = *sp;
                    sp = sp.wrapping_add(1);
                    green = (((hi as c_int) << 8) | (lo as c_int)) as png_uint_16;
                    hi = *sp;
                    sp = sp.wrapping_add(1);
                    lo = *sp;
                    sp = sp.wrapping_add(1);
                    blue = (((hi as c_int) << 8) | (lo as c_int)) as png_uint_16;

                    if red == green && red == blue {
                        if !(*png_ptr).gamma_16_table.is_null() {
                            w = *(*(*png_ptr).gamma_16_table.wrapping_add(
                                (((red as c_int) & 0xff) >> (*png_ptr).gamma_shift) as usize,
                            ))
                            .wrapping_add(((red as c_int) >> 8) as usize);
                        } else {
                            w = red;
                        }
                    } else {
                        let red_1: png_uint_16 = *(*(*png_ptr).gamma_16_to_1.wrapping_add(
                            (((red as c_int) & 0xff) >> (*png_ptr).gamma_shift) as usize,
                        ))
                        .wrapping_add(((red as c_int) >> 8) as usize);
                        let green_1: png_uint_16 = *(*(*png_ptr).gamma_16_to_1.wrapping_add(
                            (((green as c_int) & 0xff) >> (*png_ptr).gamma_shift) as usize,
                        ))
                        .wrapping_add(((green as c_int) >> 8) as usize);
                        let blue_1: png_uint_16 = *(*(*png_ptr).gamma_16_to_1.wrapping_add(
                            (((blue as c_int) & 0xff) >> (*png_ptr).gamma_shift) as usize,
                        ))
                        .wrapping_add(((blue as c_int) >> 8) as usize);
                        let gray16: png_uint_16 = ((rc.wrapping_mul(red_1 as png_uint_32))
                            .wrapping_add(gc.wrapping_mul(green_1 as png_uint_32))
                            .wrapping_add(bc.wrapping_mul(blue_1 as png_uint_32))
                            .wrapping_add(16384)
                            >> 15) as png_uint_16;
                        w = *(*(*png_ptr).gamma_16_from_1.wrapping_add(
                            (((gray16 as c_int) & 0xff) >> (*png_ptr).gamma_shift) as usize,
                        ))
                        .wrapping_add(((gray16 as c_int) >> 8) as usize);
                        rgb_error |= 1;
                    }

                    *dp = (((w as c_int) >> 8) & 0xff) as png_byte;
                    dp = dp.wrapping_add(1);
                    *dp = ((w as c_int) & 0xff) as png_byte;
                    dp = dp.wrapping_add(1);

                    if have_alpha != 0 {
                        *dp = *sp;
                        dp = dp.wrapping_add(1);
                        sp = sp.wrapping_add(1);
                        *dp = *sp;
                        dp = dp.wrapping_add(1);
                        sp = sp.wrapping_add(1);
                    }

                    i += 1;
                }
            } else {
                let mut sp: png_bytep = row;
                let mut dp: png_bytep = row;
                let mut i: png_uint_32;

                i = 0;
                while i < row_width {
                    let red: png_uint_16;
                    let green: png_uint_16;
                    let blue: png_uint_16;
                    let gray16: png_uint_16;
                    let mut hi: png_byte;
                    let mut lo: png_byte;

                    hi = *sp;
                    sp = sp.wrapping_add(1);
                    lo = *sp;
                    sp = sp.wrapping_add(1);
                    red = (((hi as c_int) << 8) | (lo as c_int)) as png_uint_16;
                    hi = *sp;
                    sp = sp.wrapping_add(1);
                    lo = *sp;
                    sp = sp.wrapping_add(1);
                    green = (((hi as c_int) << 8) | (lo as c_int)) as png_uint_16;
                    hi = *sp;
                    sp = sp.wrapping_add(1);
                    lo = *sp;
                    sp = sp.wrapping_add(1);
                    blue = (((hi as c_int) << 8) | (lo as c_int)) as png_uint_16;

                    if red != green || red != blue {
                        rgb_error |= 1;
                    }

                    /* From 1.5.5 in the 16-bit case do the accurate conversion even
                     * in the 'fast' case - this is because this is where the code
                     * ends up when handling linear 16-bit data.
                     */
                    gray16 = ((rc.wrapping_mul(red as png_uint_32))
                        .wrapping_add(gc.wrapping_mul(green as png_uint_32))
                        .wrapping_add(bc.wrapping_mul(blue as png_uint_32))
                        .wrapping_add(16384)
                        >> 15) as png_uint_16;
                    *dp = (((gray16 as c_int) >> 8) & 0xff) as png_byte;
                    dp = dp.wrapping_add(1);
                    *dp = ((gray16 as c_int) & 0xff) as png_byte;
                    dp = dp.wrapping_add(1);

                    if have_alpha != 0 {
                        *dp = *sp;
                        dp = dp.wrapping_add(1);
                        sp = sp.wrapping_add(1);
                        *dp = *sp;
                        dp = dp.wrapping_add(1);
                        sp = sp.wrapping_add(1);
                    }

                    i += 1;
                }
            }
        }

        (*row_info).channels = (((*row_info).channels as c_int) - 2) as png_byte;
        (*row_info).color_type = (((*row_info).color_type as c_int) & !PNG_COLOR_MASK_COLOR) as png_byte;
        (*row_info).pixel_depth =
            (((*row_info).channels as c_int) * ((*row_info).bit_depth as c_int)) as png_byte;
        (*row_info).rowbytes = PNG_ROWBYTES((*row_info).pixel_depth as u32, row_width);
    }
    rgb_error
}
