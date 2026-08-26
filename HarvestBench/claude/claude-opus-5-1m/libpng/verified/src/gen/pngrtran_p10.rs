/* Gamma correct the image, avoiding the alpha channel.  Make sure
 * you do this after you deal with the transparency issue on grayscale
 * or RGB images. If your bit depth is 8, use gamma_table, if it
 * is 16, use gamma_16_table and gamma_shift.  Build these with
 * build_gamma_table().
 */
unsafe fn png_do_gamma(row_info: png_row_infop, row: png_bytep, png_ptr: png_structrp) {
    let gamma_table: png_const_bytep = (*png_ptr).gamma_table as png_const_bytep;
    let gamma_16_table: png_const_uint_16pp = (*png_ptr).gamma_16_table as png_const_uint_16pp;
    let gamma_shift: c_int = (*png_ptr).gamma_shift;

    let mut sp: png_bytep;
    let mut i: png_uint_32;
    let row_width: png_uint_32 = (*row_info).width;

    if (((*row_info).bit_depth as c_int) <= 8 && !gamma_table.is_null())
        || (((*row_info).bit_depth as c_int) == 16 && !gamma_16_table.is_null())
    {
        match (*row_info).color_type as c_int {
            PNG_COLOR_TYPE_RGB => {
                if (*row_info).bit_depth as c_int == 8 {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        *sp = *gamma_table.add(*sp as usize);
                        sp = sp.add(1);
                        *sp = *gamma_table.add(*sp as usize);
                        sp = sp.add(1);
                        *sp = *gamma_table.add(*sp as usize);
                        sp = sp.add(1);
                        i += 1;
                    }
                } else
                /* if (row_info->bit_depth == 16) */
                {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let mut v: png_uint_16;

                        v = *(*gamma_16_table
                            .add(((*sp.add(1) as c_int) >> gamma_shift) as usize))
                        .add(*sp as usize);
                        *sp = (((v as c_int) >> 8) & 0xff) as png_byte;
                        *sp.add(1) = ((v as c_int) & 0xff) as png_byte;
                        sp = sp.add(2);

                        v = *(*gamma_16_table
                            .add(((*sp.add(1) as c_int) >> gamma_shift) as usize))
                        .add(*sp as usize);
                        *sp = (((v as c_int) >> 8) & 0xff) as png_byte;
                        *sp.add(1) = ((v as c_int) & 0xff) as png_byte;
                        sp = sp.add(2);

                        v = *(*gamma_16_table
                            .add(((*sp.add(1) as c_int) >> gamma_shift) as usize))
                        .add(*sp as usize);
                        *sp = (((v as c_int) >> 8) & 0xff) as png_byte;
                        *sp.add(1) = ((v as c_int) & 0xff) as png_byte;
                        sp = sp.add(2);
                        i += 1;
                    }
                }
            }

            PNG_COLOR_TYPE_RGB_ALPHA => {
                if (*row_info).bit_depth as c_int == 8 {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        *sp = *gamma_table.add(*sp as usize);
                        sp = sp.add(1);

                        *sp = *gamma_table.add(*sp as usize);
                        sp = sp.add(1);

                        *sp = *gamma_table.add(*sp as usize);
                        sp = sp.add(1);

                        sp = sp.add(1);
                        i += 1;
                    }
                } else
                /* if (row_info->bit_depth == 16) */
                {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let mut v: png_uint_16 = *(*gamma_16_table
                            .add(((*sp.add(1) as c_int) >> gamma_shift) as usize))
                        .add(*sp as usize);
                        *sp = (((v as c_int) >> 8) & 0xff) as png_byte;
                        *sp.add(1) = ((v as c_int) & 0xff) as png_byte;
                        sp = sp.add(2);

                        v = *(*gamma_16_table
                            .add(((*sp.add(1) as c_int) >> gamma_shift) as usize))
                        .add(*sp as usize);
                        *sp = (((v as c_int) >> 8) & 0xff) as png_byte;
                        *sp.add(1) = ((v as c_int) & 0xff) as png_byte;
                        sp = sp.add(2);

                        v = *(*gamma_16_table
                            .add(((*sp.add(1) as c_int) >> gamma_shift) as usize))
                        .add(*sp as usize);
                        *sp = (((v as c_int) >> 8) & 0xff) as png_byte;
                        *sp.add(1) = ((v as c_int) & 0xff) as png_byte;
                        sp = sp.add(4);
                        i += 1;
                    }
                }
            }

            PNG_COLOR_TYPE_GRAY_ALPHA => {
                if (*row_info).bit_depth as c_int == 8 {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        *sp = *gamma_table.add(*sp as usize);
                        sp = sp.add(2);
                        i += 1;
                    }
                } else
                /* if (row_info->bit_depth == 16) */
                {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let v: png_uint_16 = *(*gamma_16_table
                            .add(((*sp.add(1) as c_int) >> gamma_shift) as usize))
                        .add(*sp as usize);
                        *sp = (((v as c_int) >> 8) & 0xff) as png_byte;
                        *sp.add(1) = ((v as c_int) & 0xff) as png_byte;
                        sp = sp.add(4);
                        i += 1;
                    }
                }
            }

            PNG_COLOR_TYPE_GRAY => {
                if (*row_info).bit_depth as c_int == 2 {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: c_int = (*sp as c_int) & 0xc0;
                        let b: c_int = (*sp as c_int) & 0x30;
                        let c: c_int = (*sp as c_int) & 0x0c;
                        let d: c_int = (*sp as c_int) & 0x03;

                        let ga: c_int =
                            *gamma_table.add((a | (a >> 2) | (a >> 4) | (a >> 6)) as usize) as c_int;
                        let gb: c_int = *gamma_table
                            .add(((b << 2) | b | (b >> 2) | (b >> 4)) as usize)
                            as c_int;
                        let gc: c_int = *gamma_table
                            .add(((c << 4) | (c << 2) | c | (c >> 2)) as usize)
                            as c_int;
                        let gd: c_int = *gamma_table
                            .add(((d << 6) | (d << 4) | (d << 2) | d) as usize)
                            as c_int;

                        *sp = ((ga & 0xc0)
                            | ((gb >> 2) & 0x30)
                            | ((gc >> 4) & 0x0c)
                            | (gd >> 6)) as png_byte;
                        sp = sp.add(1);
                        i += 4;
                    }
                }

                if (*row_info).bit_depth as c_int == 4 {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let msb: c_int = (*sp as c_int) & 0xf0;
                        let lsb: c_int = (*sp as c_int) & 0x0f;

                        let gmsb: c_int =
                            *gamma_table.add((msb | (msb >> 4)) as usize) as c_int;
                        let glsb: c_int =
                            *gamma_table.add(((lsb << 4) | lsb) as usize) as c_int;

                        *sp = ((gmsb & 0xf0) | (glsb >> 4)) as png_byte;
                        sp = sp.add(1);
                        i += 2;
                    }
                } else if (*row_info).bit_depth as c_int == 8 {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        *sp = *gamma_table.add(*sp as usize);
                        sp = sp.add(1);
                        i += 1;
                    }
                } else if (*row_info).bit_depth as c_int == 16 {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let v: png_uint_16 = *(*gamma_16_table
                            .add(((*sp.add(1) as c_int) >> gamma_shift) as usize))
                        .add(*sp as usize);
                        *sp = (((v as c_int) >> 8) & 0xff) as png_byte;
                        *sp.add(1) = ((v as c_int) & 0xff) as png_byte;
                        sp = sp.add(2);
                        i += 1;
                    }
                }
            }

            _ => {}
        }
    }
}

/* Encode the alpha channel to the output gamma (the input channel is always
 * linear.)  Called only with color types that have an alpha channel.  Needs the
 * from_1 tables.
 */
unsafe fn png_do_encode_alpha(row_info: png_row_infop, row: png_bytep, png_ptr: png_structrp) {
    let mut row: png_bytep = row;
    let mut row_width: png_uint_32 = (*row_info).width;

    if (((*row_info).color_type as c_int) & PNG_COLOR_MASK_ALPHA) != 0 {
        if (*row_info).bit_depth as c_int == 8 {
            let table: png_bytep = (*png_ptr).gamma_from_1;

            if !table.is_null() {
                let step: c_int = if (((*row_info).color_type as c_int) & PNG_COLOR_MASK_COLOR) != 0
                {
                    4
                } else {
                    2
                };

                /* The alpha channel is the last component: */
                row = row.offset((step - 1) as isize);

                while row_width > 0 {
                    *row = *table.add(*row as usize);
                    row_width -= 1;
                    row = row.offset(step as isize);
                }

                return;
            }
        } else if (*row_info).bit_depth as c_int == 16 {
            let table: png_uint_16pp = (*png_ptr).gamma_16_from_1;
            let gamma_shift: c_int = (*png_ptr).gamma_shift;

            if !table.is_null() {
                let step: c_int = if (((*row_info).color_type as c_int) & PNG_COLOR_MASK_COLOR) != 0
                {
                    8
                } else {
                    4
                };

                /* The alpha channel is the last component: */
                row = row.offset((step - 2) as isize);

                while row_width > 0 {
                    let v: png_uint_16;

                    v = *(*table.add(((*row.add(1) as c_int) >> gamma_shift) as usize))
                        .add(*row as usize);
                    *row = (((v as c_int) >> 8) & 0xff) as png_byte;
                    *row.add(1) = ((v as c_int) & 0xff) as png_byte;

                    row_width -= 1;
                    row = row.offset(step as isize);
                }

                return;
            }
        }
    }

    /* Only get to here if called with a weird row_info; no harm has been done,
     * so just issue a warning.
     */
    png_warning(
        png_ptr,
        b"png_do_encode_alpha: unexpected call\0".as_ptr() as png_const_charp,
    );
}

/* Expands a palette row to an RGB or RGBA row depending
 * upon whether you supply trans and num_trans.
 */
unsafe fn png_do_expand_palette(
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
                    shift = 7 - (row_width.wrapping_add(7) & 0x07) as c_int;
                    i = 0;
                    while i < row_width {
                        if (((*sp as c_int) >> shift) & 0x01) != 0 {
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
                        i += 1;
                    }
                }

                2 => {
                    sp = row.add((row_width.wrapping_sub(1) >> 2) as usize);
                    dp = row.add(row_width as usize).sub(1);
                    shift = ((3 - (row_width.wrapping_add(3) & 0x03)) << 1) as c_int;
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
                        i += 1;
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
                        i += 1;
                    }
                }

                _ => {}
            }
            (*row_info).bit_depth = 8;
            (*row_info).pixel_depth = 8;
            (*row_info).rowbytes = row_width as usize;
        }

        if (*row_info).bit_depth as c_int == 8 {
            {
                if num_trans > 0 {
                    sp = row.add(row_width as usize).sub(1);
                    dp = row.add((row_width as usize) << 2).sub(1);

                    i = 0;

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
                        i += 1;
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

                    while i < row_width {
                        *dp = (*palette.add(*sp as usize)).blue;
                        dp = dp.sub(1);
                        *dp = (*palette.add(*sp as usize)).green;
                        dp = dp.sub(1);
                        *dp = (*palette.add(*sp as usize)).red;
                        dp = dp.sub(1);
                        sp = sp.sub(1);
                        i += 1;
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
