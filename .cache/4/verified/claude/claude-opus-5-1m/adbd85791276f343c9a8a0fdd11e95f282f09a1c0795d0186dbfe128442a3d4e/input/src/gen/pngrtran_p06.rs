/* pngrtran.c lines 2070..2529 */

/* Modify the info structure to reflect the transformations.  The
 * info should be updated so a PNG file could be written with it,
 * assuming the transformations result in valid PNG data.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_transform_info(png_ptr: png_structrp, info_ptr: png_inforp) {
    if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
        && !(*info_ptr).palette.is_null()
        && !(*png_ptr).palette.is_null()
    {
        /* Sync info_ptr->palette with png_ptr->palette, which may
         * have been modified by png_init_read_transformations
         * (e.g. for gamma correction or background compositing).
         */
        memcpy(
            (*info_ptr).palette as *mut c_void,
            (*png_ptr).palette as *const c_void,
            (PNG_MAX_PALETTE_LENGTH as usize).wrapping_mul(core::mem::size_of::<png_color>()),
        );
    }

    if ((*png_ptr).transformations & PNG_EXPAND) != 0 {
        if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            /* This check must match what actually happens in
             * png_do_expand_palette; if it ever checks the tRNS chunk to see if
             * it is all opaque we must do the same (at present it does not.)
             */
            if (*png_ptr).num_trans > 0 {
                (*info_ptr).color_type = PNG_COLOR_TYPE_RGB_ALPHA as png_byte;
            } else {
                (*info_ptr).color_type = PNG_COLOR_TYPE_RGB as png_byte;
            }

            (*info_ptr).bit_depth = 8;
            (*info_ptr).num_trans = 0;

            if (*png_ptr).palette.is_null() {
                png_error(
                    png_ptr,
                    b"Palette is NULL in indexed image\0".as_ptr() as png_const_charp,
                );
            }
        } else {
            if (*png_ptr).num_trans != 0 {
                if ((*png_ptr).transformations & PNG_EXPAND_tRNS) != 0 {
                    (*info_ptr).color_type |= PNG_COLOR_MASK_ALPHA as png_byte;
                }
            }
            if ((*info_ptr).bit_depth as c_int) < 8 {
                (*info_ptr).bit_depth = 8;
            }

            (*info_ptr).num_trans = 0;
        }
    }

    /* The following is almost certainly wrong unless the background value is in
     * the screen space!
     */
    if ((*png_ptr).transformations & PNG_COMPOSE) != 0 {
        (*info_ptr).background = (*png_ptr).background;
    }

    /* The following used to be conditional on PNG_GAMMA (prior to 1.5.4),
     * however it seems that the code in png_init_read_transformations, which has
     * been called before this from png_read_update_info->png_read_start_row
     * sometimes does the gamma transform and cancels the flag.
     *
     * TODO: this is confusing.  It only changes the result of png_get_gAMA and,
     * yes, it does return the value that the transformed data effectively has
     * but does any app really understand this?
     */
    (*info_ptr).gamma = (*png_ptr).file_gamma;

    if (*info_ptr).bit_depth as c_int == 16 {
        if ((*png_ptr).transformations & PNG_SCALE_16_TO_8) != 0 {
            (*info_ptr).bit_depth = 8;
        }

        if ((*png_ptr).transformations & PNG_16_TO_8) != 0 {
            (*info_ptr).bit_depth = 8;
        }
    }

    if ((*png_ptr).transformations & PNG_GRAY_TO_RGB) != 0 {
        (*info_ptr).color_type =
            ((*info_ptr).color_type as c_int | PNG_COLOR_MASK_COLOR) as png_byte;
    }

    if ((*png_ptr).transformations & PNG_RGB_TO_GRAY) != 0 {
        (*info_ptr).color_type =
            ((*info_ptr).color_type as c_int & !PNG_COLOR_MASK_COLOR) as png_byte;
    }

    if ((*png_ptr).transformations & PNG_QUANTIZE) != 0 {
        if ((*info_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB
            || (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA)
            && !(*png_ptr).palette_lookup.is_null()
            && (*info_ptr).bit_depth as c_int == 8
        {
            (*info_ptr).color_type = PNG_COLOR_TYPE_PALETTE as png_byte;
        }
    }

    if ((*png_ptr).transformations & PNG_EXPAND_16) != 0
        && (*info_ptr).bit_depth as c_int == 8
        && (*info_ptr).color_type as c_int != PNG_COLOR_TYPE_PALETTE
    {
        (*info_ptr).bit_depth = 16;
    }

    if ((*png_ptr).transformations & PNG_PACK) != 0 && ((*info_ptr).bit_depth as c_int) < 8 {
        (*info_ptr).bit_depth = 8;
    }

    if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        (*info_ptr).channels = 1;
    } else if ((*info_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        (*info_ptr).channels = 3;
    } else {
        (*info_ptr).channels = 1;
    }

    if ((*png_ptr).transformations & PNG_STRIP_ALPHA) != 0 {
        (*info_ptr).color_type =
            ((*info_ptr).color_type as c_int & !PNG_COLOR_MASK_ALPHA) as png_byte;
        (*info_ptr).num_trans = 0;
    }

    if ((*info_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0 {
        (*info_ptr).channels = (*info_ptr).channels.wrapping_add(1);
    }

    /* STRIP_ALPHA and FILLER allowed:  MASK_ALPHA bit stripped above */
    if ((*png_ptr).transformations & PNG_FILLER) != 0
        && ((*info_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB
            || (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY)
    {
        (*info_ptr).channels = (*info_ptr).channels.wrapping_add(1);
        /* If adding a true alpha channel not just filler */
        if ((*png_ptr).transformations & PNG_ADD_ALPHA) != 0 {
            (*info_ptr).color_type |= PNG_COLOR_MASK_ALPHA as png_byte;
        }
    }

    if ((*png_ptr).transformations & PNG_USER_TRANSFORM) != 0 {
        if (*png_ptr).user_transform_depth != 0 {
            (*info_ptr).bit_depth = (*png_ptr).user_transform_depth;
        }

        if (*png_ptr).user_transform_channels != 0 {
            (*info_ptr).channels = (*png_ptr).user_transform_channels;
        }
    }

    (*info_ptr).pixel_depth =
        (((*info_ptr).channels as c_int) * ((*info_ptr).bit_depth as c_int)) as png_byte;

    (*info_ptr).rowbytes = PNG_ROWBYTES(
        (*info_ptr).pixel_depth as usize,
        (*info_ptr).width as usize,
    );

    /* Adding in 1.5.4: cache the above value in png_struct so that we can later
     * check in png_rowbytes that the user buffer won't get overwritten.  Note
     * that the field is not always set - if png_read_update_info isn't called
     * the application has to either not do any transforms or get the calculation
     * right itself.
     */
    (*png_ptr).info_rowbytes = (*info_ptr).rowbytes;
}

/* Unpack pixels of 1, 2, or 4 bits per pixel into 1 byte per pixel,
 * without changing the actual values.  Thus, if you had a row with
 * a bit depth of 1, you would end up with bytes that only contained
 * the numbers 0 or 1.  If you would rather they contain 0 and 255, use
 * png_do_shift() after this.
 */
unsafe fn png_do_unpack(row_info: png_row_infop, row: png_bytep) {
    if ((*row_info).bit_depth as c_int) < 8 {
        let mut i: png_uint_32;
        let row_width: png_uint_32 = (*row_info).width;

        match (*row_info).bit_depth as c_int {
            1 => {
                let mut sp: png_bytep = row.add((row_width.wrapping_sub(1) >> 3) as usize);
                let mut dp: png_bytep = row.add(row_width as usize).sub(1);
                let mut shift: png_uint_32 = 7u32.wrapping_sub(row_width.wrapping_add(7) & 0x07);
                i = 0;
                while i < row_width {
                    *dp = ((((*sp) as c_int) >> shift) & 0x01) as png_byte;

                    if shift == 7 {
                        shift = 0;
                        sp = sp.sub(1);
                    } else {
                        shift += 1;
                    }

                    dp = dp.sub(1);
                    i += 1;
                }
            }

            2 => {
                let mut sp: png_bytep = row.add((row_width.wrapping_sub(1) >> 2) as usize);
                let mut dp: png_bytep = row.add(row_width as usize).sub(1);
                let mut shift: png_uint_32 =
                    3u32.wrapping_sub(row_width.wrapping_add(3) & 0x03) << 1;
                i = 0;
                while i < row_width {
                    *dp = ((((*sp) as c_int) >> shift) & 0x03) as png_byte;

                    if shift == 6 {
                        shift = 0;
                        sp = sp.sub(1);
                    } else {
                        shift += 2;
                    }

                    dp = dp.sub(1);
                    i += 1;
                }
            }

            4 => {
                let mut sp: png_bytep = row.add((row_width.wrapping_sub(1) >> 1) as usize);
                let mut dp: png_bytep = row.add(row_width as usize).sub(1);
                let mut shift: png_uint_32 =
                    1u32.wrapping_sub(row_width.wrapping_add(1) & 0x01) << 2;
                i = 0;
                while i < row_width {
                    *dp = ((((*sp) as c_int) >> shift) & 0x0f) as png_byte;

                    if shift == 4 {
                        shift = 0;
                        sp = sp.sub(1);
                    } else {
                        shift = 4;
                    }

                    dp = dp.sub(1);
                    i += 1;
                }
            }

            _ => {}
        }
        (*row_info).bit_depth = 8;
        (*row_info).pixel_depth = (8 * (*row_info).channels as c_int) as png_byte;
        (*row_info).rowbytes = (row_width as usize).wrapping_mul((*row_info).channels as usize);
    }
}

/* Reverse the effects of png_do_shift.  This routine merely shifts the
 * pixels back to their significant bits values.  Thus, if you have
 * a row of bit depth 8, but only 5 are significant, this will shift
 * the values back to 0 through 31.
 */
unsafe fn png_do_unshift(
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
            shift[channels as usize] = bit_depth - (*sig_bits).red as c_int;
            channels += 1;
            shift[channels as usize] = bit_depth - (*sig_bits).green as c_int;
            channels += 1;
            shift[channels as usize] = bit_depth - (*sig_bits).blue as c_int;
            channels += 1;
        } else {
            shift[channels as usize] = bit_depth - (*sig_bits).gray as c_int;
            channels += 1;
        }

        if (color_type & PNG_COLOR_MASK_ALPHA) != 0 {
            shift[channels as usize] = bit_depth - (*sig_bits).alpha as c_int;
            channels += 1;
        }

        {
            let mut c: c_int;
            let mut have_shift: c_int;

            c = 0;
            have_shift = 0;
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
                let bp_end: png_bytep = bp.add((*row_info).rowbytes);

                while bp < bp_end {
                    let b: c_int = (((*bp) as c_int) >> 1) & 0x55;
                    *bp = b as png_byte;
                    bp = bp.add(1);
                }
            }

            /* Must be 4bpp gray */
            /* assert(channels == 1) */
            4 => {
                let mut bp: png_bytep = row;
                let bp_end: png_bytep = bp.add((*row_info).rowbytes);
                let gray_shift: c_int = shift[0];
                let mut mask: c_int = 0xf >> gray_shift;

                mask |= mask << 4;

                while bp < bp_end {
                    let b: c_int = (((*bp) as c_int) >> gray_shift) & mask;
                    *bp = b as png_byte;
                    bp = bp.add(1);
                }
            }

            /* Single byte components, G, GA, RGB, RGBA */
            8 => {
                let mut bp: png_bytep = row;
                let bp_end: png_bytep = bp.add((*row_info).rowbytes);
                let mut channel: c_int = 0;

                while bp < bp_end {
                    let b: c_int = ((*bp) as c_int) >> shift[channel as usize];
                    channel += 1;
                    if channel >= channels {
                        channel = 0;
                    }
                    *bp = b as png_byte;
                    bp = bp.add(1);
                }
            }

            /* Double byte components, G, GA, RGB, RGBA */
            16 => {
                let mut bp: png_bytep = row;
                let bp_end: png_bytep = bp.add((*row_info).rowbytes);
                let mut channel: c_int = 0;

                while bp < bp_end {
                    let mut value: c_int = ((*bp.add(0) as c_int) << 8) + (*bp.add(1) as c_int);

                    value >>= shift[channel as usize];
                    channel += 1;
                    if channel >= channels {
                        channel = 0;
                    }
                    *bp = (value >> 8) as png_byte;
                    bp = bp.add(1);
                    *bp = value as png_byte;
                    bp = bp.add(1);
                }
            }

            /* Must be 1bpp gray: should not be here! */
            /* NOTREACHED */
            _ => {}
        }
    }
}
