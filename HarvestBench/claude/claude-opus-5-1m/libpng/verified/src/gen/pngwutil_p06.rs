/* Initializes the row writing capability of libpng */
/* png_write_start_row */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_start_row(png_ptr: png_structrp) {
    let buf_size: png_alloc_size_t;
    let usr_pixel_depth: c_int;

    let mut filters: png_byte;

    usr_pixel_depth = (*png_ptr).usr_channels as c_int * (*png_ptr).usr_bit_depth as c_int;
    buf_size =
        PNG_ROWBYTES(usr_pixel_depth as usize, (*png_ptr).width as usize).wrapping_add(1) as
            png_alloc_size_t;

    /* 1.5.6: added to allow checking in the row write code. */
    (*png_ptr).transformed_pixel_depth = (*png_ptr).pixel_depth;
    (*png_ptr).maximum_pixel_depth = usr_pixel_depth as png_byte;

    /* Set up row buffer */
    (*png_ptr).row_buf = png_malloc(png_ptr, buf_size) as png_bytep;

    *(*png_ptr).row_buf.add(0) = PNG_FILTER_VALUE_NONE as png_byte;

    filters = (*png_ptr).do_filter;

    if (*png_ptr).height == 1 {
        filters = (filters as c_int
            & (0xff & !(PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH)))
            as png_byte;
    }

    if (*png_ptr).width == 1 {
        filters = (filters as c_int
            & (0xff & !(PNG_FILTER_SUB | PNG_FILTER_AVG | PNG_FILTER_PAETH)))
            as png_byte;
    }

    if filters == 0 {
        filters = PNG_FILTER_NONE as png_byte;
    }

    (*png_ptr).do_filter = filters;

    if (filters as c_int & (PNG_FILTER_SUB | PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH))
        != 0
        && (*png_ptr).try_row == core::ptr::null_mut()
    {
        let mut num_filters: c_int = 0;

        (*png_ptr).try_row = png_malloc(png_ptr, buf_size) as png_bytep;

        if filters as c_int & PNG_FILTER_SUB != 0 {
            num_filters += 1;
        }

        if filters as c_int & PNG_FILTER_UP != 0 {
            num_filters += 1;
        }

        if filters as c_int & PNG_FILTER_AVG != 0 {
            num_filters += 1;
        }

        if filters as c_int & PNG_FILTER_PAETH != 0 {
            num_filters += 1;
        }

        if num_filters > 1 {
            (*png_ptr).tst_row = png_malloc(png_ptr, buf_size) as png_bytep;
        }
    }

    /* We only need to keep the previous row if we are using one of the following
     * filters.
     */
    if (filters as c_int & (PNG_FILTER_AVG | PNG_FILTER_UP | PNG_FILTER_PAETH)) != 0 {
        (*png_ptr).prev_row = png_calloc(png_ptr, buf_size) as png_bytep;
    }

    /* If interlaced, we need to set up width and height of pass */
    if (*png_ptr).interlaced != 0 {
        if ((*png_ptr).transformations & PNG_INTERLACE) == 0 {
            (*png_ptr).num_rows = (*png_ptr)
                .height
                .wrapping_add(png_pass_yinc[0] as png_uint_32)
                .wrapping_sub(1)
                .wrapping_sub(png_pass_ystart[0] as png_uint_32)
                / png_pass_yinc[0] as png_uint_32;

            (*png_ptr).usr_width = (*png_ptr)
                .width
                .wrapping_add(png_pass_inc[0] as png_uint_32)
                .wrapping_sub(1)
                .wrapping_sub(png_pass_start[0] as png_uint_32)
                / png_pass_inc[0] as png_uint_32;
        } else {
            (*png_ptr).num_rows = (*png_ptr).height;
            (*png_ptr).usr_width = (*png_ptr).width;
        }
    } else {
        (*png_ptr).num_rows = (*png_ptr).height;
        (*png_ptr).usr_width = (*png_ptr).width;
    }
}

/* Internal use only.  Called when finished processing a row of data. */
/* png_write_finish_row */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_finish_row(png_ptr: png_structrp) {
    /* Next row */
    (*png_ptr).row_number = (*png_ptr).row_number.wrapping_add(1);

    /* See if we are done */
    if (*png_ptr).row_number < (*png_ptr).num_rows {
        return;
    }

    /* If interlaced, go to next pass */
    if (*png_ptr).interlaced != 0 {
        (*png_ptr).row_number = 0;
        if ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
            (*png_ptr).pass = (*png_ptr).pass.wrapping_add(1);
        } else {
            /* Loop until we find a non-zero width or height pass */
            loop {
                (*png_ptr).pass = (*png_ptr).pass.wrapping_add(1);

                if (*png_ptr).pass >= 7 {
                    break;
                }

                (*png_ptr).usr_width = (*png_ptr)
                    .width
                    .wrapping_add(png_pass_inc[(*png_ptr).pass as usize] as png_uint_32)
                    .wrapping_sub(1)
                    .wrapping_sub(png_pass_start[(*png_ptr).pass as usize] as png_uint_32)
                    / png_pass_inc[(*png_ptr).pass as usize] as png_uint_32;

                (*png_ptr).num_rows = (*png_ptr)
                    .height
                    .wrapping_add(png_pass_yinc[(*png_ptr).pass as usize] as png_uint_32)
                    .wrapping_sub(1)
                    .wrapping_sub(png_pass_ystart[(*png_ptr).pass as usize] as png_uint_32)
                    / png_pass_yinc[(*png_ptr).pass as usize] as png_uint_32;

                if ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
                    break;
                }

                if !((*png_ptr).usr_width == 0 || (*png_ptr).num_rows == 0) {
                    break;
                }
            }
        }

        /* Reset the row above the image for the next pass */
        if (*png_ptr).pass < 7 {
            if (*png_ptr).prev_row != core::ptr::null_mut() {
                memset(
                    (*png_ptr).prev_row as *mut c_void,
                    0,
                    PNG_ROWBYTES(
                        ((*png_ptr).usr_channels as c_int * (*png_ptr).usr_bit_depth as c_int)
                            as usize,
                        (*png_ptr).width as usize,
                    )
                    .wrapping_add(1),
                );
            }

            return;
        }
    }

    /* If we get here, we've just written the last row, so we need
       to flush the compressor */
    png_compress_IDAT(png_ptr, core::ptr::null(), 0, Z_FINISH);
}

/* Pick out the correct pixels for the interlace pass.
 * The basic idea here is to go through the row with a source
 * pointer and a destination pointer (sp and dp), and copy the
 * correct pixels for the pass.  As the row gets compacted,
 * sp will always be >= dp, so we should never overwrite anything.
 * See the default: case for the easiest code to understand.
 */
/* png_do_write_interlace */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_write_interlace(
    row_info: png_row_infop,
    row: png_bytep,
    pass: c_int,
) {
    /* We don't have to do anything on the last pass (6) */
    if pass < 6 {
        /* Each pixel depth is handled separately */
        match (*row_info).pixel_depth {
            1 => {
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut shift: c_uint;
                let mut d: c_int;
                let mut value: c_int;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                dp = row;
                d = 0;
                shift = 7;

                i = png_pass_start[pass as usize] as png_uint_32;
                while i < row_width {
                    sp = row.add((i >> 3) as usize);
                    value = ((*sp as c_int) >> (7 - (i & 0x07) as c_int)) & 0x01;
                    d |= value << shift;

                    if shift == 0 {
                        shift = 7;
                        *dp = d as png_byte;
                        dp = dp.add(1);
                        d = 0;
                    } else {
                        shift -= 1;
                    }

                    i = i.wrapping_add(png_pass_inc[pass as usize] as png_uint_32);
                }
                if shift != 7 {
                    *dp = d as png_byte;
                }
            }

            2 => {
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut shift: c_uint;
                let mut d: c_int;
                let mut value: c_int;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                dp = row;
                shift = 6;
                d = 0;

                i = png_pass_start[pass as usize] as png_uint_32;
                while i < row_width {
                    sp = row.add((i >> 2) as usize);
                    value = ((*sp as c_int) >> ((3 - (i & 0x03) as c_int) << 1)) & 0x03;
                    d |= value << shift;

                    if shift == 0 {
                        shift = 6;
                        *dp = d as png_byte;
                        dp = dp.add(1);
                        d = 0;
                    } else {
                        shift -= 2;
                    }

                    i = i.wrapping_add(png_pass_inc[pass as usize] as png_uint_32);
                }
                if shift != 6 {
                    *dp = d as png_byte;
                }
            }

            4 => {
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut shift: c_uint;
                let mut d: c_int;
                let mut value: c_int;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                dp = row;
                shift = 4;
                d = 0;

                i = png_pass_start[pass as usize] as png_uint_32;
                while i < row_width {
                    sp = row.add((i >> 1) as usize);
                    value = ((*sp as c_int) >> ((1 - (i & 0x01) as c_int) << 2)) & 0x0f;
                    d |= value << shift;

                    if shift == 0 {
                        shift = 4;
                        *dp = d as png_byte;
                        dp = dp.add(1);
                        d = 0;
                    } else {
                        shift -= 4;
                    }

                    i = i.wrapping_add(png_pass_inc[pass as usize] as png_uint_32);
                }
                if shift != 4 {
                    *dp = d as png_byte;
                }
            }

            _ => {
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;
                let pixel_bytes: usize;

                /* Start at the beginning */
                dp = row;

                /* Find out how many bytes each pixel takes up */
                pixel_bytes = ((*row_info).pixel_depth >> 3) as usize;

                /* Loop through the row, only looking at the pixels that matter */
                i = png_pass_start[pass as usize] as png_uint_32;
                while i < row_width {
                    /* Find out where the original pixel is */
                    sp = row.add((i as usize).wrapping_mul(pixel_bytes));

                    /* Move the pixel */
                    if dp != sp {
                        memcpy(dp as *mut c_void, sp as *const c_void, pixel_bytes);
                    }

                    /* Next pixel */
                    dp = dp.add(pixel_bytes);

                    i = i.wrapping_add(png_pass_inc[pass as usize] as png_uint_32);
                }
            }
        }
        /* Set new row width */
        (*row_info).width = (*row_info)
            .width
            .wrapping_add(png_pass_inc[pass as usize] as png_uint_32)
            .wrapping_sub(1)
            .wrapping_sub(png_pass_start[pass as usize] as png_uint_32)
            / png_pass_inc[pass as usize] as png_uint_32;

        (*row_info).rowbytes =
            PNG_ROWBYTES((*row_info).pixel_depth as usize, (*row_info).width as usize);
    }
}
