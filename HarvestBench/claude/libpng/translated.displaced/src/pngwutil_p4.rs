use crate::*;

/* Initializes the row writing capability of libpng */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_start_row(png_ptr: png_structrp) {
    let buf_size: png_alloc_size_t;
    let usr_pixel_depth: c_int;

    let mut filters: png_byte;

    usr_pixel_depth = (*png_ptr).usr_channels as c_int * (*png_ptr).usr_bit_depth as c_int;
    buf_size = PNG_ROWBYTES(usr_pixel_depth as usize, (*png_ptr).width as usize) + 1;

    /* 1.5.6: added to allow checking in the row write code. */
    (*png_ptr).transformed_pixel_depth = (*png_ptr).pixel_depth;
    (*png_ptr).maximum_pixel_depth = usr_pixel_depth as png_byte;

    /* Set up row buffer */
    (*png_ptr).row_buf = png_malloc(png_ptr as png_const_structrp, buf_size) as png_bytep;

    *(*png_ptr).row_buf = PNG_FILTER_VALUE_NONE as png_byte;

    filters = (*png_ptr).do_filter;

    if (*png_ptr).height == 1 {
        filters = (filters as c_int
            & (0xff & !(PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH))) as png_byte;
    }

    if (*png_ptr).width == 1 {
        filters = (filters as c_int
            & (0xff & !(PNG_FILTER_SUB | PNG_FILTER_AVG | PNG_FILTER_PAETH))) as png_byte;
    }

    if filters == 0 {
        filters = PNG_FILTER_NONE as png_byte;
    }

    (*png_ptr).do_filter = filters;

    if (filters as c_int & (PNG_FILTER_SUB | PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH))
        != 0
        && (*png_ptr).try_row.is_null()
    {
        let mut num_filters: c_int = 0;

        (*png_ptr).try_row = png_malloc(png_ptr as png_const_structrp, buf_size) as png_bytep;

        if (filters as c_int & PNG_FILTER_SUB) != 0 {
            num_filters += 1;
        }

        if (filters as c_int & PNG_FILTER_UP) != 0 {
            num_filters += 1;
        }

        if (filters as c_int & PNG_FILTER_AVG) != 0 {
            num_filters += 1;
        }

        if (filters as c_int & PNG_FILTER_PAETH) != 0 {
            num_filters += 1;
        }

        if num_filters > 1 {
            (*png_ptr).tst_row = png_malloc(png_ptr as png_const_structrp, buf_size) as png_bytep;
        }
    }

    /* We only need to keep the previous row if we are using one of the following
     * filters.
     */
    if (filters as c_int & (PNG_FILTER_AVG | PNG_FILTER_UP | PNG_FILTER_PAETH)) != 0 {
        (*png_ptr).prev_row = png_calloc(png_ptr as png_const_structrp, buf_size) as png_bytep;
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
            if !(*png_ptr).prev_row.is_null() {
                memset(
                    (*png_ptr).prev_row as *mut c_void,
                    0,
                    PNG_ROWBYTES(
                        ((*png_ptr).usr_channels as c_int * (*png_ptr).usr_bit_depth as c_int)
                            as usize,
                        (*png_ptr).width as usize,
                    ) + 1,
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_write_interlace(
    row_info: png_row_infop,
    row: png_bytep,
    pass: c_int,
) {
    /* We don't have to do anything on the last pass (6) */
    if pass < 6 {
        /* Each pixel depth is handled separately */
        match (*row_info).pixel_depth as c_int {
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
                    sp = row.add((i as usize) * pixel_bytes);

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

/* This filters the row, chooses which filter to use, if it has not already
 * been specified by the application, and then writes the row out with the
 * chosen filter.
 */

unsafe fn png_setup_sub_row(
    png_ptr: png_structrp,
    bpp: png_uint_32,
    row_bytes: usize,
    lmins: usize,
) -> usize {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut lp: png_bytep;
    let mut i: usize;
    let mut sum: usize = 0;
    let mut v: c_uint;

    *(*png_ptr).try_row = PNG_FILTER_VALUE_SUB as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    while i < bpp as usize {
        *dp = *rp;
        v = *dp as c_uint;

        sum += (if v < 128 {
            v
        } else {
            (256 as c_uint).wrapping_sub(v)
        }) as usize;

        i += 1;
        rp = rp.add(1);
        dp = dp.add(1);
    }

    lp = (*png_ptr).row_buf.add(1);
    while i < row_bytes {
        *dp = ((*rp as c_int - *lp as c_int) & 0xff) as png_byte;
        v = *dp as c_uint;

        sum += (if v < 128 {
            v
        } else {
            (256 as c_uint).wrapping_sub(v)
        }) as usize;

        if sum > lmins {
            /* We are already worse, don't continue. */
            break;
        }

        i += 1;
        rp = rp.add(1);
        lp = lp.add(1);
        dp = dp.add(1);
    }

    sum
}

unsafe fn png_setup_sub_row_only(png_ptr: png_structrp, bpp: png_uint_32, row_bytes: usize) {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut lp: png_bytep;
    let mut i: usize;

    *(*png_ptr).try_row = PNG_FILTER_VALUE_SUB as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    while i < bpp as usize {
        *dp = *rp;

        i += 1;
        rp = rp.add(1);
        dp = dp.add(1);
    }

    lp = (*png_ptr).row_buf.add(1);
    while i < row_bytes {
        *dp = ((*rp as c_int - *lp as c_int) & 0xff) as png_byte;

        i += 1;
        rp = rp.add(1);
        lp = lp.add(1);
        dp = dp.add(1);
    }
}

unsafe fn png_setup_up_row(png_ptr: png_structrp, row_bytes: usize, lmins: usize) -> usize {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut i: usize;
    let mut sum: usize = 0;
    let mut v: c_uint;

    *(*png_ptr).try_row = PNG_FILTER_VALUE_UP as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < row_bytes {
        *dp = ((*rp as c_int - *pp as c_int) & 0xff) as png_byte;
        v = *dp as c_uint;

        sum += (if v < 128 {
            v
        } else {
            (256 as c_uint).wrapping_sub(v)
        }) as usize;

        if sum > lmins {
            /* We are already worse, don't continue. */
            break;
        }

        i += 1;
        rp = rp.add(1);
        pp = pp.add(1);
        dp = dp.add(1);
    }

    sum
}

unsafe fn png_setup_up_row_only(png_ptr: png_structrp, row_bytes: usize) {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut i: usize;

    *(*png_ptr).try_row = PNG_FILTER_VALUE_UP as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < row_bytes {
        *dp = ((*rp as c_int - *pp as c_int) & 0xff) as png_byte;

        i += 1;
        rp = rp.add(1);
        pp = pp.add(1);
        dp = dp.add(1);
    }
}

unsafe fn png_setup_avg_row(
    png_ptr: png_structrp,
    bpp: png_uint_32,
    row_bytes: usize,
    lmins: usize,
) -> usize {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut lp: png_bytep;
    let mut i: png_uint_32;
    let mut sum: usize = 0;
    let mut v: c_uint;

    *(*png_ptr).try_row = PNG_FILTER_VALUE_AVG as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < bpp {
        let b = ((*rp as c_int - (*pp as c_int / 2)) & 0xff) as png_byte;
        *dp = b;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);
        v = b as c_uint;

        sum += (if v < 128 {
            v
        } else {
            (256 as c_uint).wrapping_sub(v)
        }) as usize;

        i = i.wrapping_add(1);
    }

    lp = (*png_ptr).row_buf.add(1);
    while (i as usize) < row_bytes {
        let b = ((*rp as c_int - ((*pp as c_int + *lp as c_int) / 2)) & 0xff) as png_byte;
        *dp = b;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);
        lp = lp.add(1);
        v = b as c_uint;

        sum += (if v < 128 {
            v
        } else {
            (256 as c_uint).wrapping_sub(v)
        }) as usize;

        if sum > lmins {
            /* We are already worse, don't continue. */
            break;
        }

        i = i.wrapping_add(1);
    }

    sum
}

unsafe fn png_setup_avg_row_only(png_ptr: png_structrp, bpp: png_uint_32, row_bytes: usize) {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut lp: png_bytep;
    let mut i: png_uint_32;

    *(*png_ptr).try_row = PNG_FILTER_VALUE_AVG as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < bpp {
        *dp = ((*rp as c_int - (*pp as c_int / 2)) & 0xff) as png_byte;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);

        i = i.wrapping_add(1);
    }

    lp = (*png_ptr).row_buf.add(1);
    while (i as usize) < row_bytes {
        *dp = ((*rp as c_int - ((*pp as c_int + *lp as c_int) / 2)) & 0xff) as png_byte;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);
        lp = lp.add(1);

        i = i.wrapping_add(1);
    }
}

unsafe fn png_setup_paeth_row(
    png_ptr: png_structrp,
    bpp: png_uint_32,
    row_bytes: usize,
    lmins: usize,
) -> usize {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut cp: png_bytep;
    let mut lp: png_bytep;
    let mut i: usize;
    let mut sum: usize = 0;
    let mut v: c_uint;

    *(*png_ptr).try_row = PNG_FILTER_VALUE_PAETH as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < bpp as usize {
        let bv = ((*rp as c_int - *pp as c_int) & 0xff) as png_byte;
        *dp = bv;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);
        v = bv as c_uint;

        sum += (if v < 128 {
            v
        } else {
            (256 as c_uint).wrapping_sub(v)
        }) as usize;

        i += 1;
    }

    lp = (*png_ptr).row_buf.add(1);
    cp = (*png_ptr).prev_row.add(1);
    while i < row_bytes {
        let a: c_int;
        let b: c_int;
        let c: c_int;
        let pa: c_int;
        let pb: c_int;
        let mut pc: c_int;
        let mut p: c_int;

        b = *pp as c_int;
        pp = pp.add(1);
        c = *cp as c_int;
        cp = cp.add(1);
        a = *lp as c_int;
        lp = lp.add(1);

        p = b - c;
        pc = a - c;

        pa = if p < 0 { -p } else { p };
        pb = if pc < 0 { -pc } else { pc };
        pc = if (p + pc) < 0 { -(p + pc) } else { p + pc };

        p = if pa <= pb && pa <= pc {
            a
        } else if pb <= pc {
            b
        } else {
            c
        };

        let bv = ((*rp as c_int - p) & 0xff) as png_byte;
        *dp = bv;
        dp = dp.add(1);
        rp = rp.add(1);
        v = bv as c_uint;

        sum += (if v < 128 {
            v
        } else {
            (256 as c_uint).wrapping_sub(v)
        }) as usize;

        if sum > lmins {
            /* We are already worse, don't continue. */
            break;
        }

        i += 1;
    }

    sum
}

unsafe fn png_setup_paeth_row_only(png_ptr: png_structrp, bpp: png_uint_32, row_bytes: usize) {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut cp: png_bytep;
    let mut lp: png_bytep;
    let mut i: usize;

    *(*png_ptr).try_row = PNG_FILTER_VALUE_PAETH as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < bpp as usize {
        *dp = ((*rp as c_int - *pp as c_int) & 0xff) as png_byte;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);

        i += 1;
    }

    lp = (*png_ptr).row_buf.add(1);
    cp = (*png_ptr).prev_row.add(1);
    while i < row_bytes {
        let a: c_int;
        let b: c_int;
        let c: c_int;
        let pa: c_int;
        let pb: c_int;
        let mut pc: c_int;
        let mut p: c_int;

        b = *pp as c_int;
        pp = pp.add(1);
        c = *cp as c_int;
        cp = cp.add(1);
        a = *lp as c_int;
        lp = lp.add(1);

        p = b - c;
        pc = a - c;

        pa = if p < 0 { -p } else { p };
        pb = if pc < 0 { -pc } else { pc };
        pc = if (p + pc) < 0 { -(p + pc) } else { p + pc };

        p = if pa <= pb && pa <= pc {
            a
        } else if pb <= pc {
            b
        } else {
            c
        };

        *dp = ((*rp as c_int - p) & 0xff) as png_byte;
        dp = dp.add(1);
        rp = rp.add(1);

        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_find_filter(png_ptr: png_structrp, row_info: png_row_infop) {
    let mut filter_to_do: c_uint = (*png_ptr).do_filter as c_uint;
    let row_buf: png_bytep;
    let mut best_row: png_bytep;
    let bpp: png_uint_32;
    let mut mins: usize;
    let row_bytes: usize = (*row_info).rowbytes;

    /* Find out how many bytes offset each pixel is */
    bpp = (((*row_info).pixel_depth as c_int + 7) >> 3) as png_uint_32;

    row_buf = (*png_ptr).row_buf;
    mins = PNG_SIZE_MAX - 256; /* so we can detect potential overflow of the
                               running sum */

    /* The prediction method we use is to find which method provides the
     * smallest value when summing the absolute values of the distances
     * from zero, using anything >= 128 as negative numbers.  This is known
     * as the "minimum sum of absolute differences" heuristic.  Other
     * heuristics are the "weighted minimum sum of absolute differences"
     * (experimental and can in theory improve compression), and the "zlib
     * predictive" method (not implemented yet), which does test compressions
     * of lines using different filter methods, and then chooses the
     * (series of) filter(s) that give minimum compressed data size (VERY
     * computationally expensive).
     *
     * GRR 980525:  consider also
     *
     *   (1) minimum sum of absolute differences from running average (i.e.,
     *       keep running sum of non-absolute differences & count of bytes)
     *       [track dispersion, too?  restart average if dispersion too large?]
     *
     *  (1b) minimum sum of absolute differences from sliding average, probably
     *       with window size <= deflate window (usually 32K)
     *
     *   (2) minimum sum of squared differences from zero or running average
     *       (i.e., ~ root-mean-square approach)
     */

    /* We don't need to test the 'no filter' case if this is the only filter
     * that has been chosen, as it doesn't actually do anything to the data.
     */
    best_row = (*png_ptr).row_buf;

    if PNG_SIZE_MAX / 128 <= row_bytes {
        /* Overflow can occur in the calculation, just select the lowest set
         * filter.
         */
        filter_to_do &= (0 as c_uint).wrapping_sub(filter_to_do);
    } else if (filter_to_do & PNG_FILTER_NONE as c_uint) != 0
        && filter_to_do != PNG_FILTER_NONE as c_uint
    {
        /* Overflow not possible and multiple filters in the list, including the
         * 'none' filter.
         */
        let mut rp: png_bytep;
        let mut sum: usize = 0;
        let mut i: usize;
        let mut v: c_uint;

        {
            i = 0;
            rp = row_buf.add(1);
            while i < row_bytes {
                v = *rp as c_uint;

                sum += (if v < 128 {
                    v
                } else {
                    (256 as c_uint).wrapping_sub(v)
                }) as usize;

                i += 1;
                rp = rp.add(1);
            }
        }

        mins = sum;
    }

    /* Sub filter */
    if filter_to_do == PNG_FILTER_SUB as c_uint
    /* It's the only filter so no testing is needed */
    {
        png_setup_sub_row_only(png_ptr, bpp, row_bytes);
        best_row = (*png_ptr).try_row;
    } else if (filter_to_do & PNG_FILTER_SUB as c_uint) != 0 {
        let sum: usize;
        let lmins: usize = mins;

        sum = png_setup_sub_row(png_ptr, bpp, row_bytes, lmins);

        if sum < mins {
            mins = sum;
            best_row = (*png_ptr).try_row;
            if !(*png_ptr).tst_row.is_null() {
                (*png_ptr).try_row = (*png_ptr).tst_row;
                (*png_ptr).tst_row = best_row;
            }
        }
    }

    /* Up filter */
    if filter_to_do == PNG_FILTER_UP as c_uint {
        png_setup_up_row_only(png_ptr, row_bytes);
        best_row = (*png_ptr).try_row;
    } else if (filter_to_do & PNG_FILTER_UP as c_uint) != 0 {
        let sum: usize;
        let lmins: usize = mins;

        sum = png_setup_up_row(png_ptr, row_bytes, lmins);

        if sum < mins {
            mins = sum;
            best_row = (*png_ptr).try_row;
            if !(*png_ptr).tst_row.is_null() {
                (*png_ptr).try_row = (*png_ptr).tst_row;
                (*png_ptr).tst_row = best_row;
            }
        }
    }

    /* Avg filter */
    if filter_to_do == PNG_FILTER_AVG as c_uint {
        png_setup_avg_row_only(png_ptr, bpp, row_bytes);
        best_row = (*png_ptr).try_row;
    } else if (filter_to_do & PNG_FILTER_AVG as c_uint) != 0 {
        let sum: usize;
        let lmins: usize = mins;

        sum = png_setup_avg_row(png_ptr, bpp, row_bytes, lmins);

        if sum < mins {
            mins = sum;
            best_row = (*png_ptr).try_row;
            if !(*png_ptr).tst_row.is_null() {
                (*png_ptr).try_row = (*png_ptr).tst_row;
                (*png_ptr).tst_row = best_row;
            }
        }
    }

    /* Paeth filter */
    if filter_to_do == PNG_FILTER_PAETH as c_uint {
        png_setup_paeth_row_only(png_ptr, bpp, row_bytes);
        best_row = (*png_ptr).try_row;
    } else if (filter_to_do & PNG_FILTER_PAETH as c_uint) != 0 {
        let sum: usize;
        let lmins: usize = mins;

        sum = png_setup_paeth_row(png_ptr, bpp, row_bytes, lmins);

        if sum < mins {
            best_row = (*png_ptr).try_row;
            if !(*png_ptr).tst_row.is_null() {
                (*png_ptr).try_row = (*png_ptr).tst_row;
                (*png_ptr).tst_row = best_row;
            }
        }
    }

    /* Do the actual writing of the filtered row data from the chosen filter. */
    png_write_filtered_row(png_ptr, best_row, (*row_info).rowbytes + 1);
}

/* Do the actual writing of a previously filtered row. */
unsafe fn png_write_filtered_row(
    png_ptr: png_structrp,
    filtered_row: png_bytep,
    full_row_length: usize, /*includes filter byte*/
) {
    png_compress_IDAT(
        png_ptr,
        filtered_row as png_const_bytep,
        full_row_length,
        Z_NO_FLUSH,
    );

    /* Swap the current and previous rows */
    if !(*png_ptr).prev_row.is_null() {
        let tptr: png_bytep;

        tptr = (*png_ptr).prev_row;
        (*png_ptr).prev_row = (*png_ptr).row_buf;
        (*png_ptr).row_buf = tptr;
    }

    /* Finish row - updates counters and flushes zlib if last row */
    png_write_finish_row(png_ptr);

    (*png_ptr).flush_rows = (*png_ptr).flush_rows.wrapping_add(1);

    if (*png_ptr).flush_dist > 0 && (*png_ptr).flush_rows >= (*png_ptr).flush_dist {
        png_write_flush(png_ptr);
    }
}
