/* pngrutil.c part 9: interlace expansion and the row filter (un)filtering */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_read_interlace(
    row_info: png_row_infop,
    row: png_bytep,
    pass: c_int,
    transformations: png_uint_32, /* Because these may affect the byte layout */
) {
    if row != core::ptr::null_mut() && row_info != core::ptr::null_mut() {
        let final_width: png_uint_32;

        final_width = (*row_info)
            .width
            .wrapping_mul(png_pass_inc[pass as usize] as png_uint_32);

        match (*row_info).pixel_depth {
            1 => {
                let mut sp: png_bytep = row.add((((*row_info).width.wrapping_sub(1)) >> 3) as usize);
                let mut dp: png_bytep = row.add(((final_width.wrapping_sub(1)) >> 3) as usize);
                let mut sshift: c_uint;
                let mut dshift: c_uint;
                let s_start: c_uint;
                let s_end: c_uint;
                let s_inc: c_int;
                let jstop: c_int = png_pass_inc[pass as usize] as c_int;
                let mut v: png_byte;
                let mut i: png_uint_32;
                let mut j: c_int;

                if (transformations & PNG_PACKSWAP) != 0 {
                    sshift = ((*row_info).width.wrapping_add(7)) & 0x07;
                    dshift = (final_width.wrapping_add(7)) & 0x07;
                    s_start = 7;
                    s_end = 0;
                    s_inc = -1;
                } else {
                    sshift = 7u32.wrapping_sub(((*row_info).width.wrapping_add(7)) & 0x07);
                    dshift = 7u32.wrapping_sub((final_width.wrapping_add(7)) & 0x07);
                    s_start = 0;
                    s_end = 7;
                    s_inc = 1;
                }

                i = 0;
                while i < (*row_info).width {
                    v = (((*sp as c_uint) >> sshift) & 0x01) as png_byte;
                    j = 0;
                    while j < jstop {
                        let mut tmp: c_uint =
                            (*dp as c_uint) & (0x7f7fu32 >> (7u32.wrapping_sub(dshift)));
                        tmp |= (((v as c_int) << dshift) as c_uint);
                        *dp = (tmp & 0xff) as png_byte;

                        if dshift == s_end {
                            dshift = s_start;
                            dp = dp.sub(1);
                        } else {
                            dshift = ((dshift as c_int).wrapping_add(s_inc)) as c_uint;
                        }

                        j += 1;
                    }

                    if sshift == s_end {
                        sshift = s_start;
                        sp = sp.sub(1);
                    } else {
                        sshift = ((sshift as c_int).wrapping_add(s_inc)) as c_uint;
                    }

                    i = i.wrapping_add(1);
                }
            }

            2 => {
                let mut sp: png_bytep = row.add((((*row_info).width.wrapping_sub(1)) >> 2) as usize);
                let mut dp: png_bytep = row.add(((final_width.wrapping_sub(1)) >> 2) as usize);
                let mut sshift: c_uint;
                let mut dshift: c_uint;
                let s_start: c_uint;
                let s_end: c_uint;
                let s_inc: c_int;
                let jstop: c_int = png_pass_inc[pass as usize] as c_int;
                let mut i: png_uint_32;

                if (transformations & PNG_PACKSWAP) != 0 {
                    sshift = (((*row_info).width.wrapping_add(3)) & 0x03) << 1;
                    dshift = ((final_width.wrapping_add(3)) & 0x03) << 1;
                    s_start = 6;
                    s_end = 0;
                    s_inc = -2;
                } else {
                    sshift = (3u32.wrapping_sub(((*row_info).width.wrapping_add(3)) & 0x03)) << 1;
                    dshift = (3u32.wrapping_sub((final_width.wrapping_add(3)) & 0x03)) << 1;
                    s_start = 0;
                    s_end = 6;
                    s_inc = 2;
                }

                i = 0;
                while i < (*row_info).width {
                    let v: png_byte;
                    let mut j: c_int;

                    v = (((*sp as c_uint) >> sshift) & 0x03) as png_byte;
                    j = 0;
                    while j < jstop {
                        let mut tmp: c_uint =
                            (*dp as c_uint) & (0x3f3fu32 >> (6u32.wrapping_sub(dshift)));
                        tmp |= (((v as c_int) << dshift) as c_uint);
                        *dp = (tmp & 0xff) as png_byte;

                        if dshift == s_end {
                            dshift = s_start;
                            dp = dp.sub(1);
                        } else {
                            dshift = ((dshift as c_int).wrapping_add(s_inc)) as c_uint;
                        }

                        j += 1;
                    }

                    if sshift == s_end {
                        sshift = s_start;
                        sp = sp.sub(1);
                    } else {
                        sshift = ((sshift as c_int).wrapping_add(s_inc)) as c_uint;
                    }

                    i = i.wrapping_add(1);
                }
            }

            4 => {
                let mut sp: png_bytep = row.add((((*row_info).width.wrapping_sub(1)) >> 1) as usize);
                let mut dp: png_bytep = row.add(((final_width.wrapping_sub(1)) >> 1) as usize);
                let mut sshift: c_uint;
                let mut dshift: c_uint;
                let s_start: c_uint;
                let s_end: c_uint;
                let s_inc: c_int;
                let mut i: png_uint_32;
                let jstop: c_int = png_pass_inc[pass as usize] as c_int;

                if (transformations & PNG_PACKSWAP) != 0 {
                    sshift = (((*row_info).width.wrapping_add(1)) & 0x01) << 2;
                    dshift = ((final_width.wrapping_add(1)) & 0x01) << 2;
                    s_start = 4;
                    s_end = 0;
                    s_inc = -4;
                } else {
                    sshift = (1u32.wrapping_sub(((*row_info).width.wrapping_add(1)) & 0x01)) << 2;
                    dshift = (1u32.wrapping_sub((final_width.wrapping_add(1)) & 0x01)) << 2;
                    s_start = 0;
                    s_end = 4;
                    s_inc = 4;
                }

                i = 0;
                while i < (*row_info).width {
                    let v: png_byte = (((*sp as c_uint) >> sshift) & 0x0f) as png_byte;
                    let mut j: c_int;

                    j = 0;
                    while j < jstop {
                        let mut tmp: c_uint =
                            (*dp as c_uint) & (0xf0fu32 >> (4u32.wrapping_sub(dshift)));
                        tmp |= (((v as c_int) << dshift) as c_uint);
                        *dp = (tmp & 0xff) as png_byte;

                        if dshift == s_end {
                            dshift = s_start;
                            dp = dp.sub(1);
                        } else {
                            dshift = ((dshift as c_int).wrapping_add(s_inc)) as c_uint;
                        }

                        j += 1;
                    }

                    if sshift == s_end {
                        sshift = s_start;
                        sp = sp.sub(1);
                    } else {
                        sshift = ((sshift as c_int).wrapping_add(s_inc)) as c_uint;
                    }

                    i = i.wrapping_add(1);
                }
            }

            _ => {
                let pixel_bytes: usize = ((*row_info).pixel_depth >> 3) as usize;

                let mut sp: png_bytep =
                    row.add(((*row_info).width.wrapping_sub(1) as usize).wrapping_mul(pixel_bytes));

                let mut dp: png_bytep =
                    row.add((final_width.wrapping_sub(1) as usize).wrapping_mul(pixel_bytes));

                let jstop: c_int = png_pass_inc[pass as usize] as c_int;
                let mut i: png_uint_32;

                i = 0;
                while i < (*row_info).width {
                    let mut v: [png_byte; 8] = [0; 8]; /* SAFE; pixel_depth does not exceed 64 */
                    let mut j: c_int;

                    memcpy(
                        v.as_mut_ptr() as *mut c_void,
                        sp as *const c_void,
                        pixel_bytes,
                    );

                    j = 0;
                    while j < jstop {
                        memcpy(
                            dp as *mut c_void,
                            v.as_ptr() as *const c_void,
                            pixel_bytes,
                        );
                        dp = dp.sub(pixel_bytes);
                        j += 1;
                    }

                    sp = sp.sub(pixel_bytes);
                    i = i.wrapping_add(1);
                }
            }
        }

        (*row_info).width = final_width;
        (*row_info).rowbytes = PNG_ROWBYTES((*row_info).pixel_depth as usize, final_width as usize);
    }
}

unsafe extern "C" fn png_read_filter_row_sub(
    row_info: png_row_infop,
    row: png_bytep,
    prev_row: png_const_bytep,
) {
    let mut i: usize;
    let istop: usize = (*row_info).rowbytes;
    let bpp: c_uint = (((*row_info).pixel_depth as c_int + 7) >> 3) as c_uint;
    let mut rp: png_bytep = row.add(bpp as usize);

    i = bpp as usize;
    while i < istop {
        *rp = (((*rp as c_int) + (*rp.sub(bpp as usize) as c_int)) & 0xff) as png_byte;
        rp = rp.add(1);
        i = i.wrapping_add(1);
    }
}

unsafe extern "C" fn png_read_filter_row_up(
    row_info: png_row_infop,
    row: png_bytep,
    prev_row: png_const_bytep,
) {
    let mut i: usize;
    let istop: usize = (*row_info).rowbytes;
    let mut rp: png_bytep = row;
    let mut pp: png_const_bytep = prev_row;

    i = 0;
    while i < istop {
        let p: png_byte = *pp;
        pp = pp.add(1);
        *rp = (((*rp as c_int) + (p as c_int)) & 0xff) as png_byte;
        rp = rp.add(1);
        i = i.wrapping_add(1);
    }
}

unsafe extern "C" fn png_read_filter_row_avg(
    row_info: png_row_infop,
    row: png_bytep,
    prev_row: png_const_bytep,
) {
    let mut i: usize;
    let mut rp: png_bytep = row;
    let mut pp: png_const_bytep = prev_row;
    let bpp: c_uint = (((*row_info).pixel_depth as c_int + 7) >> 3) as c_uint;
    let istop: usize = (*row_info).rowbytes.wrapping_sub(bpp as usize);

    i = 0;
    while i < bpp as usize {
        let p: png_byte = *pp;
        pp = pp.add(1);
        *rp = (((*rp as c_int) + ((p as c_int) / 2)) & 0xff) as png_byte;

        rp = rp.add(1);
        i = i.wrapping_add(1);
    }

    i = 0;
    while i < istop {
        let p: png_byte = *pp;
        pp = pp.add(1);
        *rp = (((*rp as c_int) + ((p as c_int) + (*rp.sub(bpp as usize) as c_int)) / 2) & 0xff)
            as png_byte;

        rp = rp.add(1);
        i = i.wrapping_add(1);
    }
}

unsafe extern "C" fn png_read_filter_row_paeth_1byte_pixel(
    row_info: png_row_infop,
    mut row: png_bytep,
    mut prev_row: png_const_bytep,
) {
    let rp_end: png_bytep = row.add((*row_info).rowbytes);
    let mut a: c_int;
    let mut c: c_int;

    /* First pixel/byte */
    c = *prev_row as c_int;
    prev_row = prev_row.add(1);
    a = (*row as c_int) + c;
    *row = a as png_byte;
    row = row.add(1);

    /* Remainder */
    while row < rp_end {
        let b: c_int;
        let mut pa: c_int;
        let pb: c_int;
        let mut pc: c_int;
        let p: c_int;

        a &= 0xff; /* From previous iteration or start */
        b = *prev_row as c_int;
        prev_row = prev_row.add(1);

        p = b - c;
        pc = a - c;

        pa = if p < 0 { -p } else { p };
        pb = if pc < 0 { -pc } else { pc };
        pc = if (p + pc) < 0 { -(p + pc) } else { p + pc };

        /* Find the best predictor, the least of pa, pb, pc favoring the earlier
         * ones in the case of a tie.
         */
        if pb < pa {
            pa = pb;
            a = b;
        }
        if pc < pa {
            a = c;
        }

        /* Calculate the current pixel in a, and move the previous row pixel to c
         * for the next time round the loop
         */
        c = b;
        a += *row as c_int;
        *row = a as png_byte;
        row = row.add(1);
    }
}

unsafe extern "C" fn png_read_filter_row_paeth_multibyte_pixel(
    row_info: png_row_infop,
    mut row: png_bytep,
    mut prev_row: png_const_bytep,
) {
    let bpp: c_uint = (((*row_info).pixel_depth as c_int + 7) >> 3) as c_uint;
    let mut rp_end: png_bytep = row.add(bpp as usize);

    /* Process the first pixel in the row completely (this is the same as 'up'
     * because there is only one candidate predictor for the first row).
     */
    while row < rp_end {
        let a: c_int = (*row as c_int) + (*prev_row as c_int);
        prev_row = prev_row.add(1);
        *row = a as png_byte;
        row = row.add(1);
    }

    /* Remainder */
    rp_end = rp_end.add((*row_info).rowbytes.wrapping_sub(bpp as usize));

    while row < rp_end {
        let mut a: c_int;
        let b: c_int;
        let c: c_int;
        let mut pa: c_int;
        let pb: c_int;
        let mut pc: c_int;
        let p: c_int;

        c = *prev_row.sub(bpp as usize) as c_int;
        a = *row.sub(bpp as usize) as c_int;
        b = *prev_row as c_int;
        prev_row = prev_row.add(1);

        p = b - c;
        pc = a - c;

        pa = if p < 0 { -p } else { p };
        pb = if pc < 0 { -pc } else { pc };
        pc = if (p + pc) < 0 { -(p + pc) } else { p + pc };

        if pb < pa {
            pa = pb;
            a = b;
        }
        if pc < pa {
            a = c;
        }

        a += *row as c_int;
        *row = a as png_byte;
        row = row.add(1);
    }
}

unsafe fn png_init_filter_functions(pp: png_structrp)
/* This function is called once for every PNG image (except for PNG images
 * that only use PNG_FILTER_VALUE_NONE for all rows) to set the
 * implementations required to reverse the filtering of PNG rows.  Reversing
 * the filter is the first transformation performed on the row data.  It is
 * performed in place, therefore an implementation can be selected based on
 * the image pixel format.  If the implementation depends on image width then
 * take care to ensure that it works correctly if the image is interlaced -
 * interlacing causes the actual row width to vary.
 */
{
    let bpp: c_uint = (((*pp).pixel_depth as c_int + 7) >> 3) as c_uint;

    (*pp).read_filter[(PNG_FILTER_VALUE_SUB - 1) as usize] = Some(png_read_filter_row_sub);
    (*pp).read_filter[(PNG_FILTER_VALUE_UP - 1) as usize] = Some(png_read_filter_row_up);
    (*pp).read_filter[(PNG_FILTER_VALUE_AVG - 1) as usize] = Some(png_read_filter_row_avg);
    if bpp == 1 {
        (*pp).read_filter[(PNG_FILTER_VALUE_PAETH - 1) as usize] =
            Some(png_read_filter_row_paeth_1byte_pixel);
    } else {
        (*pp).read_filter[(PNG_FILTER_VALUE_PAETH - 1) as usize] =
            Some(png_read_filter_row_paeth_multibyte_pixel);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_filter_row(
    pp: png_structrp,
    row_info: png_row_infop,
    row: png_bytep,
    prev_row: png_const_bytep,
    filter: c_int,
) {
    /* OPTIMIZATION: DO NOT MODIFY THIS FUNCTION, instead #define
     * PNG_FILTER_OPTIMIZATIONS to a function that overrides the generic
     * implementations.  See png_init_filter_functions above.
     */
    if filter > PNG_FILTER_VALUE_NONE && filter < PNG_FILTER_VALUE_LAST {
        if (*pp).read_filter[0].is_none() {
            png_init_filter_functions(pp);
        }

        ((*pp).read_filter[(filter - 1) as usize].unwrap())(row_info, row, prev_row);
    }
}
