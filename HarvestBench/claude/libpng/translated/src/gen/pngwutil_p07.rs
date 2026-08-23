/* This filters the row, chooses which filter to use, if it has not already
 * been specified by the application, and then writes the row out with the
 * chosen filter.
 */

/* png_setup_sub_row */
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

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_SUB as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    while i < bpp as usize {
        *dp = *rp;
        v = *dp as c_uint;
        sum = sum.wrapping_add(if v < 128 {
            v as usize
        } else {
            (256 as c_uint).wrapping_sub(v) as usize
        });

        i += 1;
        rp = rp.add(1);
        dp = dp.add(1);
    }

    lp = (*png_ptr).row_buf.add(1);
    while i < row_bytes {
        *dp = (((*rp as c_int) - (*lp as c_int)) & 0xff) as png_byte;
        v = *dp as c_uint;
        sum = sum.wrapping_add(if v < 128 {
            v as usize
        } else {
            (256 as c_uint).wrapping_sub(v) as usize
        });

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

/* png_setup_sub_row_only */
unsafe fn png_setup_sub_row_only(png_ptr: png_structrp, bpp: png_uint_32, row_bytes: usize) {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut lp: png_bytep;
    let mut i: usize;

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_SUB as png_byte;

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
        *dp = (((*rp as c_int) - (*lp as c_int)) & 0xff) as png_byte;

        i += 1;
        rp = rp.add(1);
        lp = lp.add(1);
        dp = dp.add(1);
    }
}

/* png_setup_up_row */
unsafe fn png_setup_up_row(png_ptr: png_structrp, row_bytes: usize, lmins: usize) -> usize {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut i: usize;
    let mut sum: usize = 0;
    let mut v: c_uint;

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_UP as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < row_bytes {
        *dp = (((*rp as c_int) - (*pp as c_int)) & 0xff) as png_byte;
        v = *dp as c_uint;
        sum = sum.wrapping_add(if v < 128 {
            v as usize
        } else {
            (256 as c_uint).wrapping_sub(v) as usize
        });

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

/* png_setup_up_row_only */
unsafe fn png_setup_up_row_only(png_ptr: png_structrp, row_bytes: usize) {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut i: usize;

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_UP as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < row_bytes {
        *dp = (((*rp as c_int) - (*pp as c_int)) & 0xff) as png_byte;

        i += 1;
        rp = rp.add(1);
        pp = pp.add(1);
        dp = dp.add(1);
    }
}

/* png_setup_avg_row */
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

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_AVG as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < bpp {
        let t: png_byte = (((*rp as c_int) - ((*pp as c_int) / 2)) & 0xff) as png_byte;
        *dp = t;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);
        v = t as c_uint;

        sum = sum.wrapping_add(if v < 128 {
            v as usize
        } else {
            (256 as c_uint).wrapping_sub(v) as usize
        });

        i = i.wrapping_add(1);
    }

    lp = (*png_ptr).row_buf.add(1);
    while (i as usize) < row_bytes {
        let t: png_byte =
            (((*rp as c_int) - (((*pp as c_int) + (*lp as c_int)) / 2)) & 0xff) as png_byte;
        *dp = t;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);
        lp = lp.add(1);
        v = t as c_uint;

        sum = sum.wrapping_add(if v < 128 {
            v as usize
        } else {
            (256 as c_uint).wrapping_sub(v) as usize
        });

        if sum > lmins {
            /* We are already worse, don't continue. */
            break;
        }

        i = i.wrapping_add(1);
    }

    sum
}

/* png_setup_avg_row_only */
unsafe fn png_setup_avg_row_only(png_ptr: png_structrp, bpp: png_uint_32, row_bytes: usize) {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut lp: png_bytep;
    let mut i: png_uint_32;

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_AVG as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < bpp {
        *dp = (((*rp as c_int) - ((*pp as c_int) / 2)) & 0xff) as png_byte;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);

        i = i.wrapping_add(1);
    }

    lp = (*png_ptr).row_buf.add(1);
    while (i as usize) < row_bytes {
        *dp = (((*rp as c_int) - (((*pp as c_int) + (*lp as c_int)) / 2)) & 0xff) as png_byte;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);
        lp = lp.add(1);

        i = i.wrapping_add(1);
    }
}

/* png_setup_paeth_row */
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

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_PAETH as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < bpp as usize {
        let t: png_byte = (((*rp as c_int) - (*pp as c_int)) & 0xff) as png_byte;
        *dp = t;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);
        v = t as c_uint;

        sum = sum.wrapping_add(if v < 128 {
            v as usize
        } else {
            (256 as c_uint).wrapping_sub(v) as usize
        });

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

        let t: png_byte = (((*rp as c_int) - p) & 0xff) as png_byte;
        *dp = t;
        dp = dp.add(1);
        rp = rp.add(1);
        v = t as c_uint;

        sum = sum.wrapping_add(if v < 128 {
            v as usize
        } else {
            (256 as c_uint).wrapping_sub(v) as usize
        });

        if sum > lmins {
            /* We are already worse, don't continue. */
            break;
        }

        i += 1;
    }

    sum
}

/* png_setup_paeth_row_only */
unsafe fn png_setup_paeth_row_only(png_ptr: png_structrp, bpp: png_uint_32, row_bytes: usize) {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut cp: png_bytep;
    let mut lp: png_bytep;
    let mut i: usize;

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_PAETH as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < bpp as usize {
        *dp = (((*rp as c_int) - (*pp as c_int)) & 0xff) as png_byte;
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

        *dp = (((*rp as c_int) - p) & 0xff) as png_byte;
        dp = dp.add(1);
        rp = rp.add(1);

        i += 1;
    }
}

/* png_write_find_filter */
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
    mins = PNG_SIZE_MAX.wrapping_sub(256); /* so we can detect potential overflow of the
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
                sum = sum.wrapping_add(if v < 128 {
                    v as usize
                } else {
                    (256 as c_uint).wrapping_sub(v) as usize
                });

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
            if (*png_ptr).tst_row != core::ptr::null_mut() {
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
            if (*png_ptr).tst_row != core::ptr::null_mut() {
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
            if (*png_ptr).tst_row != core::ptr::null_mut() {
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
            if (*png_ptr).tst_row != core::ptr::null_mut() {
                (*png_ptr).try_row = (*png_ptr).tst_row;
                (*png_ptr).tst_row = best_row;
            }
        }
    }

    /* Do the actual writing of the filtered row data from the chosen filter. */
    png_write_filtered_row(png_ptr, best_row, (*row_info).rowbytes.wrapping_add(1));
}

/* Do the actual writing of a previously filtered row. */
unsafe fn png_write_filtered_row(
    png_ptr: png_structrp,
    filtered_row: png_bytep,
    full_row_length: usize, /*includes filter byte*/
) {
    png_compress_IDAT(
        png_ptr,
        filtered_row,
        full_row_length as png_alloc_size_t,
        Z_NO_FLUSH,
    );

    /* Swap the current and previous rows */
    if (*png_ptr).prev_row != core::ptr::null_mut() {
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
