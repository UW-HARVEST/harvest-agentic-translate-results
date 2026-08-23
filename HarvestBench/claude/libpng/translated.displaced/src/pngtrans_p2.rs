// pngtrans.c - transforms the data in a row (used by both readers and writers)
//
// Part 2: png_do_swap .. png_get_current_pass_number

use crate::*;

/* Swaps byte order on 16-bit depth images */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_swap(row_info: png_row_infop, row: png_bytep) {
    if (*row_info).bit_depth == 16 {
        let mut rp: png_bytep = row;
        let mut i: png_uint_32;
        let istop: png_uint_32 = (*row_info).width * (*row_info).channels as png_uint_32;

        i = 0;
        while i < istop {
            let t: png_byte = *rp;
            *rp = *rp.add(1);
            *rp.add(1) = t;

            i += 1;
            rp = rp.add(2);
        }
    }
}

/* The bit swap tables onebppswaptable, twobppswaptable and fourbppswaptable
 * live in src/pngtrans_tables.rs (same module).
 */

/* Swaps pixel packing order within bytes */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_packswap(row_info: png_row_infop, row: png_bytep) {
    if (*row_info).bit_depth < 8 {
        let table: png_const_bytep;
        let mut rp: png_bytep;
        let row_end: png_bytep = row.add((*row_info).rowbytes);

        if (*row_info).bit_depth == 1 {
            table = onebppswaptable.as_ptr();
        } else if (*row_info).bit_depth == 2 {
            table = twobppswaptable.as_ptr();
        } else if (*row_info).bit_depth == 4 {
            table = fourbppswaptable.as_ptr();
        } else {
            return;
        }

        rp = row;
        while rp < row_end {
            *rp = *table.add(*rp as usize);
            rp = rp.add(1);
        }
    }
}

/* Remove a channel - this used to be 'png_do_strip_filler' but it used a
 * somewhat weird combination of flags to determine what to do.  All the calls
 * to png_do_strip_filler are changed in 1.5.2 to call this instead with the
 * correct arguments.
 *
 * The routine isn't general - the channel must be the channel at the start or
 * end (not in the middle) of each pixel.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_strip_channel(
    row_info: png_row_infop,
    row: png_bytep,
    at_start: c_int,
) {
    let mut sp: png_bytep = row; /* source pointer */
    let mut dp: png_bytep = row; /* destination pointer */
    let ep: png_bytep = row.add((*row_info).rowbytes); /* One beyond end of row */

    /* At the start sp will point to the first byte to copy and dp to where
     * it is copied to.  ep always points just beyond the end of the row, so
     * the loop simply copies (channels-1) channels until sp reaches ep.
     *
     * at_start:        0 -- convert AG, XG, ARGB, XRGB, AAGG, XXGG, etc.
     *            nonzero -- convert GA, GX, RGBA, RGBX, GGAA, RRGGBBXX, etc.
     */

    /* GA, GX, XG cases */
    if (*row_info).channels == 2 {
        if (*row_info).bit_depth == 8 {
            if at_start != 0 {
                /* Skip initial filler */
                sp = sp.add(1);
            } else {
                /* Skip initial channel and, for sp, the filler */
                sp = sp.add(2);
                dp = dp.add(1);
            }

            /* For a 1 pixel wide image there is nothing to do */
            while sp < ep {
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(2);
            }

            (*row_info).pixel_depth = 8;
        } else if (*row_info).bit_depth == 16 {
            if at_start != 0 {
                /* Skip initial filler */
                sp = sp.add(2);
            } else {
                /* Skip initial channel and, for sp, the filler */
                sp = sp.add(4);
                dp = dp.add(2);
            }

            while sp < ep {
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(1);
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(3);
            }

            (*row_info).pixel_depth = 16;
        } else {
            return; /* bad bit depth */
        }

        (*row_info).channels = 1;

        /* Finally fix the color type if it records an alpha channel */
        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
            (*row_info).color_type = PNG_COLOR_TYPE_GRAY as png_byte;
        }
    }
    /* RGBA, RGBX, XRGB cases */
    else if (*row_info).channels == 4 {
        if (*row_info).bit_depth == 8 {
            if at_start != 0 {
                /* Skip initial filler */
                sp = sp.add(1);
            } else {
                /* Skip initial channels and, for sp, the filler */
                sp = sp.add(4);
                dp = dp.add(3);
            }

            /* Note that the loop adds 3 to dp and 4 to sp each time. */
            while sp < ep {
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(1);
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(1);
                *dp = *sp;
                dp = dp.add(1);
                sp = sp.add(2);
            }

            (*row_info).pixel_depth = 24;
        } else if (*row_info).bit_depth == 16 {
            if at_start != 0 {
                /* Skip initial filler */
                sp = sp.add(2);
            } else {
                /* Skip initial channels and, for sp, the filler */
                sp = sp.add(8);
                dp = dp.add(6);
            }

            while sp < ep {
                /* Copy 6 bytes, skip 2 */
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
                sp = sp.add(3);
            }

            (*row_info).pixel_depth = 48;
        } else {
            return; /* bad bit depth */
        }

        (*row_info).channels = 3;

        /* Finally fix the color type if it records an alpha channel */
        if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
            (*row_info).color_type = PNG_COLOR_TYPE_RGB as png_byte;
        }
    } else {
        return; /* The filler channel has gone already */
    }

    /* Fix the rowbytes value. */
    (*row_info).rowbytes = ((dp as isize) - (row as isize)) as usize;
}

/* Swaps red and blue bytes within a pixel */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_bgr(row_info: png_row_infop, row: png_bytep) {
    if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        let row_width: png_uint_32 = (*row_info).width;
        if (*row_info).bit_depth == 8 {
            if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB {
                let mut rp: png_bytep;
                let mut i: png_uint_32;

                i = 0;
                rp = row;
                while i < row_width {
                    let save: png_byte = *rp;
                    *rp = *rp.add(2);
                    *rp.add(2) = save;

                    i += 1;
                    rp = rp.add(3);
                }
            } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                let mut rp: png_bytep;
                let mut i: png_uint_32;

                i = 0;
                rp = row;
                while i < row_width {
                    let save: png_byte = *rp;
                    *rp = *rp.add(2);
                    *rp.add(2) = save;

                    i += 1;
                    rp = rp.add(4);
                }
            }
        } else if (*row_info).bit_depth == 16 {
            if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB {
                let mut rp: png_bytep;
                let mut i: png_uint_32;

                i = 0;
                rp = row;
                while i < row_width {
                    let mut save: png_byte = *rp;
                    *rp = *rp.add(4);
                    *rp.add(4) = save;
                    save = *rp.add(1);
                    *rp.add(1) = *rp.add(5);
                    *rp.add(5) = save;

                    i += 1;
                    rp = rp.add(6);
                }
            } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                let mut rp: png_bytep;
                let mut i: png_uint_32;

                i = 0;
                rp = row;
                while i < row_width {
                    let mut save: png_byte = *rp;
                    *rp = *rp.add(4);
                    *rp.add(4) = save;
                    save = *rp.add(1);
                    *rp.add(1) = *rp.add(5);
                    *rp.add(5) = save;

                    i += 1;
                    rp = rp.add(8);
                }
            }
        }
    }
}

/* Added at libpng-1.5.10 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_check_palette_indexes(
    png_ptr: png_structrp,
    row_info: png_row_infop,
) {
    if ((*png_ptr).num_palette as c_int) < (1 << (*row_info).bit_depth as c_int)
        && (*png_ptr).num_palette as c_int > 0
    /* num_palette can be 0 in MNG files */
    {
        /* Calculations moved outside switch in an attempt to stop different
         * compiler warnings.  'padding' is in *bits* within the last byte, it is
         * an 'int' because pixel_depth becomes an 'int' in the expression below,
         * and this calculation is used because it avoids warnings that other
         * forms produced on either GCC or MSVC.
         */
        let mut padding: c_int =
            PNG_PADBITS((*row_info).pixel_depth as png_uint_32, (*row_info).width) as c_int;
        let mut rp: png_bytep = (*png_ptr).row_buf.add((*row_info).rowbytes);

        match (*row_info).bit_depth {
            1 => {
                /* in this case, all bytes must be 0 so we don't need
                 * to unpack the pixels except for the rightmost one.
                 */
                while rp > (*png_ptr).row_buf {
                    if ((*rp as c_int) >> padding) != 0 {
                        (*png_ptr).num_palette_max = 1;
                    }
                    padding = 0;

                    rp = rp.sub(1);
                }
            }

            2 => {
                while rp > (*png_ptr).row_buf {
                    let mut i: c_int = ((*rp as c_int) >> padding) & 0x03;

                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }

                    i = (((*rp as c_int) >> padding) >> 2) & 0x03;

                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }

                    i = (((*rp as c_int) >> padding) >> 4) & 0x03;

                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }

                    i = (((*rp as c_int) >> padding) >> 6) & 0x03;

                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }

                    padding = 0;

                    rp = rp.sub(1);
                }
            }

            4 => {
                while rp > (*png_ptr).row_buf {
                    let mut i: c_int = ((*rp as c_int) >> padding) & 0x0f;

                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }

                    i = (((*rp as c_int) >> padding) >> 4) & 0x0f;

                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }

                    padding = 0;

                    rp = rp.sub(1);
                }
            }

            8 => {
                while rp > (*png_ptr).row_buf {
                    if (*rp as c_int) > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = *rp as c_int;
                    }

                    rp = rp.sub(1);
                }
            }

            _ => {}
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_user_transform_info(
    png_ptr: png_structrp,
    user_transform_ptr: png_voidp,
    user_transform_depth: c_int,
    user_transform_channels: c_int,
) {
    if png_ptr.is_null() {
        return;
    }

    if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 && ((*png_ptr).flags & PNG_FLAG_ROW_INIT) != 0 {
        png_app_error(
            png_ptr,
            cstr!("info change after png_start_read_image or png_read_update_info"),
        );
        return;
    }

    (*png_ptr).user_transform_ptr = user_transform_ptr;
    (*png_ptr).user_transform_depth = user_transform_depth as png_byte;
    (*png_ptr).user_transform_channels = user_transform_channels as png_byte;
}

/* This function returns a pointer to the user_transform_ptr associated with
 * the user transform functions.  The application should free any memory
 * associated with this pointer before png_write_destroy and png_read_destroy
 * are called.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_user_transform_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr.is_null() {
        return core::ptr::null_mut();
    }

    (*png_ptr).user_transform_ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_current_row_number(arg: png_const_structrp) -> png_uint_32 {
    /* See the comments in png.h - this is the sub-image row when reading an
     * interlaced image.
     */
    if !arg.is_null() {
        return (*arg).row_number;
    }

    PNG_UINT_32_MAX /* help the app not to fail silently */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_current_pass_number(arg: png_const_structrp) -> png_byte {
    if !arg.is_null() {
        return (*arg).pass;
    }
    8 /* invalid */
}
