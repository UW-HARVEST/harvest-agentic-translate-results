/* Scale rows of bit depth 16 down to 8 accurately */
unsafe fn png_do_scale_16_to_8(row_info: png_row_infop, row: png_bytep) {
    if (*row_info).bit_depth as c_int == 16 {
        let mut sp: png_bytep = row; /* source */
        let mut dp: png_bytep = row; /* destination */
        let ep: png_bytep = sp.add((*row_info).rowbytes); /* end+1 */

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
            sp = sp.add(1);
            let lo: c_int = *sp as c_int;
            sp = sp.add(1);
            tmp += ((lo - tmp + 128) * 65535) >> 24;
            *dp = tmp as png_byte;
            dp = dp.add(1);
        }

        (*row_info).bit_depth = 8;
        (*row_info).pixel_depth = (8 * (*row_info).channels as c_int) as png_byte;
        (*row_info).rowbytes = (*row_info).width as usize * (*row_info).channels as usize;
    }
}

/* Simply discard the low byte.  This was the default behavior prior
 * to libpng-1.5.4.
 */
unsafe fn png_do_chop(row_info: png_row_infop, row: png_bytep) {
    if (*row_info).bit_depth as c_int == 16 {
        let mut sp: png_bytep = row; /* source */
        let mut dp: png_bytep = row; /* destination */
        let ep: png_bytep = sp.add((*row_info).rowbytes); /* end+1 */

        while sp < ep {
            *dp = *sp;
            dp = dp.add(1);
            sp = sp.add(2); /* skip low byte */
        }

        (*row_info).bit_depth = 8;
        (*row_info).pixel_depth = (8 * (*row_info).channels as c_int) as png_byte;
        (*row_info).rowbytes = (*row_info).width as usize * (*row_info).channels as usize;
    }
}

unsafe fn png_do_read_swap_alpha(row_info: png_row_infop, row: png_bytep) {
    let row_width: png_uint_32 = (*row_info).width;

    if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
        /* This converts from RGBA to ARGB */
        if (*row_info).bit_depth as c_int == 8 {
            let mut sp: png_bytep = row.add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;
            let mut save: png_byte;

            let mut i: png_uint_32 = 0;
            while i < row_width {
                sp = sp.sub(1);
                save = *sp;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = *sp;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = *sp;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = *sp;
                dp = dp.sub(1);
                *dp = save;
                i += 1;
            }
        }
        /* This converts from RRGGBBAA to AARRGGBB */
        else {
            let mut sp: png_bytep = row.add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;
            let mut save: [png_byte; 2] = [0; 2];

            let mut i: png_uint_32 = 0;
            while i < row_width {
                sp = sp.sub(1);
                save[0] = *sp;
                sp = sp.sub(1);
                save[1] = *sp;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = *sp;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = *sp;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = *sp;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = *sp;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = *sp;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = *sp;
                dp = dp.sub(1);
                *dp = save[0];
                dp = dp.sub(1);
                *dp = save[1];
                i += 1;
            }
        }
    } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
        /* This converts from GA to AG */
        if (*row_info).bit_depth as c_int == 8 {
            let mut sp: png_bytep = row.add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;
            let mut save: png_byte;

            let mut i: png_uint_32 = 0;
            while i < row_width {
                sp = sp.sub(1);
                save = *sp;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = *sp;
                dp = dp.sub(1);
                *dp = save;
                i += 1;
            }
        }
        /* This converts from GGAA to AAGG */
        else {
            let mut sp: png_bytep = row.add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;
            let mut save: [png_byte; 2] = [0; 2];

            let mut i: png_uint_32 = 0;
            while i < row_width {
                sp = sp.sub(1);
                save[0] = *sp;
                sp = sp.sub(1);
                save[1] = *sp;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = *sp;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = *sp;
                dp = dp.sub(1);
                *dp = save[0];
                dp = dp.sub(1);
                *dp = save[1];
                i += 1;
            }
        }
    }
}

unsafe fn png_do_read_invert_alpha(row_info: png_row_infop, row: png_bytep) {
    let row_width: png_uint_32;

    row_width = (*row_info).width;
    if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
        if (*row_info).bit_depth as c_int == 8 {
            /* This inverts the alpha channel in RGBA */
            let mut sp: png_bytep = row.add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;

            let mut i: png_uint_32 = 0;
            while i < row_width {
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = (255 - *sp as c_int) as png_byte;

                /*          This does nothing:
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                            We can replace it with:
                */
                sp = sp.sub(3);
                dp = sp;
                i += 1;
            }
        }
        /* This inverts the alpha channel in RRGGBBAA */
        else {
            let mut sp: png_bytep = row.add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;

            let mut i: png_uint_32 = 0;
            while i < row_width {
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = (255 - *sp as c_int) as png_byte;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = (255 - *sp as c_int) as png_byte;

                /*          This does nothing:
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                            We can replace it with:
                */
                sp = sp.sub(6);
                dp = sp;
                i += 1;
            }
        }
    } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
        if (*row_info).bit_depth as c_int == 8 {
            /* This inverts the alpha channel in GA */
            let mut sp: png_bytep = row.add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;

            let mut i: png_uint_32 = 0;
            while i < row_width {
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = (255 - *sp as c_int) as png_byte;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = *sp;
                i += 1;
            }
        } else {
            /* This inverts the alpha channel in GGAA */
            let mut sp: png_bytep = row.add((*row_info).rowbytes);
            let mut dp: png_bytep = sp;

            let mut i: png_uint_32 = 0;
            while i < row_width {
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = (255 - *sp as c_int) as png_byte;
                sp = sp.sub(1);
                dp = dp.sub(1);
                *dp = (255 - *sp as c_int) as png_byte;
                /*
                            *(--dp) = *(--sp);
                            *(--dp) = *(--sp);
                */
                sp = sp.sub(2);
                dp = sp;
                i += 1;
            }
        }
    }
}

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
                    sp = sp.sub(1);
                    dp = dp.sub(1);
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
                    sp = sp.sub(1);
                    dp = dp.sub(1);
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
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
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
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
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
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
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
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
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
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
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
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
                    *dp = *sp;
                    sp = sp.sub(1);
                    dp = dp.sub(1);
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
