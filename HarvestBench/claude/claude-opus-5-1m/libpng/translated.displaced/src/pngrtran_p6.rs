// pngrtran.c - transforms the data in a row for PNG readers
//
// Part 6: png_do_read_filler .. png_do_rgb_to_gray

use crate::*;

/* Add filler channel if we have RGB color */
unsafe fn png_do_read_filler(
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
        if (*row_info).bit_depth as c_int == 8 {
            if (flags & PNG_FLAG_FILLER_AFTER) != 0 {
                /* This changes the data from G to GX */
                let mut sp: png_bytep = row.add(row_width as usize);
                let mut dp: png_bytep = sp.add(row_width as usize);
                i = 1;
                while i < row_width {
                    dp = dp.sub(1);
                    *dp = lo_filler;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    i += 1;
                }
                dp = dp.sub(1);
                *dp = lo_filler;
                (*row_info).channels = 2;
                (*row_info).pixel_depth = 16;
                (*row_info).rowbytes = row_width as usize * 2;
            } else {
                /* This changes the data from G to XG */
                let mut sp: png_bytep = row.add(row_width as usize);
                let mut dp: png_bytep = sp.add(row_width as usize);
                i = 0;
                while i < row_width {
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = lo_filler;
                    i += 1;
                }
                (*row_info).channels = 2;
                (*row_info).pixel_depth = 16;
                (*row_info).rowbytes = row_width as usize * 2;
            }
        } else if (*row_info).bit_depth as c_int == 16 {
            if (flags & PNG_FLAG_FILLER_AFTER) != 0 {
                /* This changes the data from GG to GGXX */
                let mut sp: png_bytep = row.add(row_width as usize * 2);
                let mut dp: png_bytep = sp.add(row_width as usize * 2);
                i = 1;
                while i < row_width {
                    dp = dp.sub(1);
                    *dp = lo_filler;
                    dp = dp.sub(1);
                    *dp = hi_filler;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    i += 1;
                }
                dp = dp.sub(1);
                *dp = lo_filler;
                dp = dp.sub(1);
                *dp = hi_filler;
                (*row_info).channels = 2;
                (*row_info).pixel_depth = 32;
                (*row_info).rowbytes = row_width as usize * 4;
            } else {
                /* This changes the data from GG to XXGG */
                let mut sp: png_bytep = row.add(row_width as usize * 2);
                let mut dp: png_bytep = sp.add(row_width as usize * 2);
                i = 0;
                while i < row_width {
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = lo_filler;
                    dp = dp.sub(1);
                    *dp = hi_filler;
                    i += 1;
                }
                (*row_info).channels = 2;
                (*row_info).pixel_depth = 32;
                (*row_info).rowbytes = row_width as usize * 4;
            }
        }
    }
    /* COLOR_TYPE == GRAY */
    else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB {
        if (*row_info).bit_depth as c_int == 8 {
            if (flags & PNG_FLAG_FILLER_AFTER) != 0 {
                /* This changes the data from RGB to RGBX */
                let mut sp: png_bytep = row.add(row_width as usize * 3);
                let mut dp: png_bytep = sp.add(row_width as usize);
                i = 1;
                while i < row_width {
                    dp = dp.sub(1);
                    *dp = lo_filler;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    i += 1;
                }
                dp = dp.sub(1);
                *dp = lo_filler;
                (*row_info).channels = 4;
                (*row_info).pixel_depth = 32;
                (*row_info).rowbytes = row_width as usize * 4;
            } else {
                /* This changes the data from RGB to XRGB */
                let mut sp: png_bytep = row.add(row_width as usize * 3);
                let mut dp: png_bytep = sp.add(row_width as usize);
                i = 0;
                while i < row_width {
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
                    *dp = lo_filler;
                    i += 1;
                }
                (*row_info).channels = 4;
                (*row_info).pixel_depth = 32;
                (*row_info).rowbytes = row_width as usize * 4;
            }
        } else if (*row_info).bit_depth as c_int == 16 {
            if (flags & PNG_FLAG_FILLER_AFTER) != 0 {
                /* This changes the data from RRGGBB to RRGGBBXX */
                let mut sp: png_bytep = row.add(row_width as usize * 6);
                let mut dp: png_bytep = sp.add(row_width as usize * 2);
                i = 1;
                while i < row_width {
                    dp = dp.sub(1);
                    *dp = lo_filler;
                    dp = dp.sub(1);
                    *dp = hi_filler;
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
                    *dp = *sp;
                    i += 1;
                }
                dp = dp.sub(1);
                *dp = lo_filler;
                dp = dp.sub(1);
                *dp = hi_filler;
                (*row_info).channels = 4;
                (*row_info).pixel_depth = 64;
                (*row_info).rowbytes = row_width as usize * 8;
            } else {
                /* This changes the data from RRGGBB to XXRRGGBB */
                let mut sp: png_bytep = row.add(row_width as usize * 6);
                let mut dp: png_bytep = sp.add(row_width as usize * 2);
                i = 0;
                while i < row_width {
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
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = lo_filler;
                    dp = dp.sub(1);
                    *dp = hi_filler;
                    i += 1;
                }

                (*row_info).channels = 4;
                (*row_info).pixel_depth = 64;
                (*row_info).rowbytes = row_width as usize * 8;
            }
        }
    } /* COLOR_TYPE == RGB */
}

/* Expand grayscale files to RGB, with or without alpha */
unsafe fn png_do_gray_to_rgb(row_info: png_row_infop, row: png_bytep) {
    let mut i: png_uint_32;
    let row_width: png_uint_32 = (*row_info).width;

    if (*row_info).bit_depth as c_int >= 8
        && ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0
    {
        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY {
            if (*row_info).bit_depth as c_int == 8 {
                /* This changes G to RGB */
                let mut sp: png_bytep = row.offset(row_width as isize - 1);
                let mut dp: png_bytep = sp.add(row_width as usize * 2);
                i = 0;
                while i < row_width {
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    i += 1;
                }
            } else {
                /* This changes GG to RRGGBB */
                let mut sp: png_bytep = row.offset(row_width as isize * 2 - 1);
                let mut dp: png_bytep = sp.add(row_width as usize * 4);
                i = 0;
                while i < row_width {
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = *sp.offset(-1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = *sp.offset(-1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    i += 1;
                }
            }
        } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
            if (*row_info).bit_depth as c_int == 8 {
                /* This changes GA to RGBA */
                let mut sp: png_bytep = row.offset(row_width as isize * 2 - 1);
                let mut dp: png_bytep = sp.add(row_width as usize * 2);
                i = 0;
                while i < row_width {
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    i += 1;
                }
            } else {
                /* This changes GGAA to RRGGBBAA */
                let mut sp: png_bytep = row.offset(row_width as isize * 4 - 1);
                let mut dp: png_bytep = sp.add(row_width as usize * 4);
                i = 0;
                while i < row_width {
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = *sp.offset(-1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    *dp = *sp.offset(-1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    *dp = *sp;
                    dp = dp.sub(1);
                    sp = sp.sub(1);
                    i += 1;
                }
            }
        }
        (*row_info).channels = ((*row_info).channels as c_int + 2) as png_byte;
        (*row_info).color_type =
            ((*row_info).color_type as c_int | PNG_COLOR_MASK_COLOR) as png_byte;
        (*row_info).pixel_depth =
            ((*row_info).channels as c_int * (*row_info).bit_depth as c_int) as png_byte;
        (*row_info).rowbytes = PNG_ROWBYTES((*row_info).pixel_depth as usize, row_width as usize);
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
unsafe fn png_do_rgb_to_gray(
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

        if (*row_info).bit_depth as c_int == 8 {
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
                    sp = sp.add(1);
                    let mut green: png_byte = *sp;
                    sp = sp.add(1);
                    let mut blue: png_byte = *sp;
                    sp = sp.add(1);

                    if red != green || red != blue {
                        red = *(*png_ptr).gamma_to_1.add(red as usize);
                        green = *(*png_ptr).gamma_to_1.add(green as usize);
                        blue = *(*png_ptr).gamma_to_1.add(blue as usize);

                        rgb_error |= 1;
                        *dp = *(*png_ptr).gamma_from_1.add(
                            (rc.wrapping_mul(red as png_uint_32)
                                .wrapping_add(gc.wrapping_mul(green as png_uint_32))
                                .wrapping_add(bc.wrapping_mul(blue as png_uint_32))
                                .wrapping_add(16384)
                                >> 15) as usize,
                        );
                        dp = dp.add(1);
                    } else {
                        /* If there is no overall correction the table will not be
                         * set.
                         */
                        if !(*png_ptr).gamma_table.is_null() {
                            red = *(*png_ptr).gamma_table.add(red as usize);
                        }

                        *dp = red;
                        dp = dp.add(1);
                    }

                    if have_alpha != 0 {
                        *dp = *sp;
                        dp = dp.add(1);
                        sp = sp.add(1);
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
                    sp = sp.add(1);
                    let green: png_byte = *sp;
                    sp = sp.add(1);
                    let blue: png_byte = *sp;
                    sp = sp.add(1);

                    if red != green || red != blue {
                        rgb_error |= 1;
                        /* NOTE: this is the historical approach which simply
                         * truncates the results.
                         */
                        *dp = (rc
                            .wrapping_mul(red as png_uint_32)
                            .wrapping_add(gc.wrapping_mul(green as png_uint_32))
                            .wrapping_add(bc.wrapping_mul(blue as png_uint_32))
                            >> 15) as png_byte;
                        dp = dp.add(1);
                    } else {
                        *dp = red;
                        dp = dp.add(1);
                    }

                    if have_alpha != 0 {
                        *dp = *sp;
                        dp = dp.add(1);
                        sp = sp.add(1);
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
                    sp = sp.add(1);
                    lo = *sp;
                    sp = sp.add(1);
                    red = (((hi as c_int) << 8) | (lo as c_int)) as png_uint_16;
                    hi = *sp;
                    sp = sp.add(1);
                    lo = *sp;
                    sp = sp.add(1);
                    green = (((hi as c_int) << 8) | (lo as c_int)) as png_uint_16;
                    hi = *sp;
                    sp = sp.add(1);
                    lo = *sp;
                    sp = sp.add(1);
                    blue = (((hi as c_int) << 8) | (lo as c_int)) as png_uint_16;

                    if red == green && red == blue {
                        if !(*png_ptr).gamma_16_table.is_null() {
                            w = *(*(*png_ptr)
                                .gamma_16_table
                                .add((((red as c_int) & 0xff) >> (*png_ptr).gamma_shift) as usize))
                            .add(((red as c_int) >> 8) as usize);
                        } else {
                            w = red;
                        }
                    } else {
                        let red_1: png_uint_16 = *(*(*png_ptr)
                            .gamma_16_to_1
                            .add((((red as c_int) & 0xff) >> (*png_ptr).gamma_shift) as usize))
                        .add(((red as c_int) >> 8) as usize);
                        let green_1: png_uint_16 = *(*(*png_ptr)
                            .gamma_16_to_1
                            .add((((green as c_int) & 0xff) >> (*png_ptr).gamma_shift) as usize))
                        .add(((green as c_int) >> 8) as usize);
                        let blue_1: png_uint_16 = *(*(*png_ptr)
                            .gamma_16_to_1
                            .add((((blue as c_int) & 0xff) >> (*png_ptr).gamma_shift) as usize))
                        .add(((blue as c_int) >> 8) as usize);
                        let gray16: png_uint_16 = (rc
                            .wrapping_mul(red_1 as png_uint_32)
                            .wrapping_add(gc.wrapping_mul(green_1 as png_uint_32))
                            .wrapping_add(bc.wrapping_mul(blue_1 as png_uint_32))
                            .wrapping_add(16384)
                            >> 15) as png_uint_16;
                        w = *(*(*png_ptr)
                            .gamma_16_from_1
                            .add((((gray16 as c_int) & 0xff) >> (*png_ptr).gamma_shift) as usize))
                        .add(((gray16 as c_int) >> 8) as usize);
                        rgb_error |= 1;
                    }

                    *dp = (((w as c_int) >> 8) & 0xff) as png_byte;
                    dp = dp.add(1);
                    *dp = ((w as c_int) & 0xff) as png_byte;
                    dp = dp.add(1);

                    if have_alpha != 0 {
                        *dp = *sp;
                        dp = dp.add(1);
                        sp = sp.add(1);
                        *dp = *sp;
                        dp = dp.add(1);
                        sp = sp.add(1);
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
                    sp = sp.add(1);
                    lo = *sp;
                    sp = sp.add(1);
                    red = (((hi as c_int) << 8) | (lo as c_int)) as png_uint_16;
                    hi = *sp;
                    sp = sp.add(1);
                    lo = *sp;
                    sp = sp.add(1);
                    green = (((hi as c_int) << 8) | (lo as c_int)) as png_uint_16;
                    hi = *sp;
                    sp = sp.add(1);
                    lo = *sp;
                    sp = sp.add(1);
                    blue = (((hi as c_int) << 8) | (lo as c_int)) as png_uint_16;

                    if red != green || red != blue {
                        rgb_error |= 1;
                    }

                    /* From 1.5.5 in the 16-bit case do the accurate conversion even
                     * in the 'fast' case - this is because this is where the code
                     * ends up when handling linear 16-bit data.
                     */
                    gray16 = (rc
                        .wrapping_mul(red as png_uint_32)
                        .wrapping_add(gc.wrapping_mul(green as png_uint_32))
                        .wrapping_add(bc.wrapping_mul(blue as png_uint_32))
                        .wrapping_add(16384)
                        >> 15) as png_uint_16;
                    *dp = (((gray16 as c_int) >> 8) & 0xff) as png_byte;
                    dp = dp.add(1);
                    *dp = ((gray16 as c_int) & 0xff) as png_byte;
                    dp = dp.add(1);

                    if have_alpha != 0 {
                        *dp = *sp;
                        dp = dp.add(1);
                        sp = sp.add(1);
                        *dp = *sp;
                        dp = dp.add(1);
                        sp = sp.add(1);
                    }

                    i += 1;
                }
            }
        }

        (*row_info).channels = ((*row_info).channels as c_int - 2) as png_byte;
        (*row_info).color_type =
            ((*row_info).color_type as c_int & !PNG_COLOR_MASK_COLOR) as png_byte;
        (*row_info).pixel_depth =
            ((*row_info).channels as c_int * (*row_info).bit_depth as c_int) as png_byte;
        (*row_info).rowbytes = PNG_ROWBYTES((*row_info).pixel_depth as usize, row_width as usize);
    }
    rgb_error
}
