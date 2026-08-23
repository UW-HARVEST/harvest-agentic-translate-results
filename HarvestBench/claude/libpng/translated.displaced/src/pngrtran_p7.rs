use crate::*;

/* Replace any alpha or transparency with the supplied background color.
 * "background" is already in the screen gamma, while "background_1" is
 * at a gamma of 1.0.  Paletted files have already been taken care of.
 */
unsafe fn png_do_compose(row_info: png_row_infop, row: png_bytep, png_ptr: png_structrp) {
    let gamma_table: png_const_bytep = (*png_ptr).gamma_table as png_const_bytep;
    let gamma_from_1: png_const_bytep = (*png_ptr).gamma_from_1 as png_const_bytep;
    let gamma_to_1: png_const_bytep = (*png_ptr).gamma_to_1 as png_const_bytep;
    let gamma_16: png_const_uint_16pp = (*png_ptr).gamma_16_table as png_const_uint_16pp;
    let gamma_16_from_1: png_const_uint_16pp = (*png_ptr).gamma_16_from_1 as png_const_uint_16pp;
    let gamma_16_to_1: png_const_uint_16pp = (*png_ptr).gamma_16_to_1 as png_const_uint_16pp;
    let gamma_shift: c_int = (*png_ptr).gamma_shift;
    let optimize: c_int = (((*png_ptr).flags & PNG_FLAG_OPTIMIZE_ALPHA) != 0) as c_int;

    let mut sp: png_bytep;
    let mut i: png_uint_32;
    let row_width: png_uint_32 = (*row_info).width;
    let mut shift: c_int;

    match (*row_info).color_type as c_int {
        PNG_COLOR_TYPE_GRAY => {
            match (*row_info).bit_depth as c_int {
                1 => {
                    sp = row;
                    shift = 7;
                    i = 0;
                    while i < row_width {
                        if (((*sp as c_int >> shift) & 0x01) as png_uint_16)
                            == (*png_ptr).trans_color.gray
                        {
                            let mut tmp: c_uint =
                                (*sp as c_int & (0x7f7f >> (7 - shift))) as c_uint;
                            tmp |= (((*png_ptr).background.gray as c_int) << shift) as c_uint;
                            *sp = (tmp & 0xff) as png_byte;
                        }

                        if shift == 0 {
                            shift = 7;
                            sp = sp.offset(1);
                        } else {
                            shift -= 1;
                        }

                        i += 1;
                    }
                }

                2 => {
                    if !gamma_table.is_null() {
                        sp = row;
                        shift = 6;
                        i = 0;
                        while i < row_width {
                            if (((*sp as c_int >> shift) & 0x03) as png_uint_16)
                                == (*png_ptr).trans_color.gray
                            {
                                let mut tmp: c_uint =
                                    (*sp as c_int & (0x3f3f >> (6 - shift))) as c_uint;
                                tmp |= ((*png_ptr).background.gray as c_uint) << shift;
                                *sp = (tmp & 0xff) as png_byte;
                            } else {
                                let p: c_uint = ((*sp as c_int >> shift) & 0x03) as c_uint;
                                let g: c_uint = ((*gamma_table
                                    .offset((p | (p << 2) | (p << 4) | (p << 6)) as isize)
                                    as c_int
                                    >> 6)
                                    & 0x03) as c_uint;
                                let mut tmp: c_uint =
                                    (*sp as c_int & (0x3f3f >> (6 - shift))) as c_uint;
                                tmp |= g << shift;
                                *sp = (tmp & 0xff) as png_byte;
                            }

                            if shift == 0 {
                                shift = 6;
                                sp = sp.offset(1);
                            } else {
                                shift -= 2;
                            }

                            i += 1;
                        }
                    } else {
                        sp = row;
                        shift = 6;
                        i = 0;
                        while i < row_width {
                            if (((*sp as c_int >> shift) & 0x03) as png_uint_16)
                                == (*png_ptr).trans_color.gray
                            {
                                let mut tmp: c_uint =
                                    (*sp as c_int & (0x3f3f >> (6 - shift))) as c_uint;
                                tmp |= ((*png_ptr).background.gray as c_uint) << shift;
                                *sp = (tmp & 0xff) as png_byte;
                            }

                            if shift == 0 {
                                shift = 6;
                                sp = sp.offset(1);
                            } else {
                                shift -= 2;
                            }

                            i += 1;
                        }
                    }
                }

                4 => {
                    if !gamma_table.is_null() {
                        sp = row;
                        shift = 4;
                        i = 0;
                        while i < row_width {
                            if (((*sp as c_int >> shift) & 0x0f) as png_uint_16)
                                == (*png_ptr).trans_color.gray
                            {
                                let mut tmp: c_uint =
                                    (*sp as c_int & (0x0f0f >> (4 - shift))) as c_uint;
                                tmp |= (((*png_ptr).background.gray as c_int) << shift) as c_uint;
                                *sp = (tmp & 0xff) as png_byte;
                            } else {
                                let p: c_uint = ((*sp as c_int >> shift) & 0x0f) as c_uint;
                                let g: c_uint = ((*gamma_table.offset((p | (p << 4)) as isize)
                                    as c_int
                                    >> 4)
                                    & 0x0f) as c_uint;
                                let mut tmp: c_uint =
                                    (*sp as c_int & (0x0f0f >> (4 - shift))) as c_uint;
                                tmp |= (g << shift) as c_uint;
                                *sp = (tmp & 0xff) as png_byte;
                            }

                            if shift == 0 {
                                shift = 4;
                                sp = sp.offset(1);
                            } else {
                                shift -= 4;
                            }

                            i += 1;
                        }
                    } else {
                        sp = row;
                        shift = 4;
                        i = 0;
                        while i < row_width {
                            if (((*sp as c_int >> shift) & 0x0f) as png_uint_16)
                                == (*png_ptr).trans_color.gray
                            {
                                let mut tmp: c_uint =
                                    (*sp as c_int & (0x0f0f >> (4 - shift))) as c_uint;
                                tmp |= (((*png_ptr).background.gray as c_int) << shift) as c_uint;
                                *sp = (tmp & 0xff) as png_byte;
                            }

                            if shift == 0 {
                                shift = 4;
                                sp = sp.offset(1);
                            } else {
                                shift -= 4;
                            }

                            i += 1;
                        }
                    }
                }

                8 => {
                    if !gamma_table.is_null() {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            if *sp as c_int == (*png_ptr).trans_color.gray as c_int {
                                *sp = (*png_ptr).background.gray as png_byte;
                            } else {
                                *sp = *gamma_table.offset(*sp as isize);
                            }

                            i += 1;
                            sp = sp.offset(1);
                        }
                    } else {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            if *sp as c_int == (*png_ptr).trans_color.gray as c_int {
                                *sp = (*png_ptr).background.gray as png_byte;
                            }

                            i += 1;
                            sp = sp.offset(1);
                        }
                    }
                }

                16 => {
                    if !gamma_16.is_null() {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let mut v: png_uint_16;

                            v = (((*sp as c_int) << 8) + *sp.offset(1) as c_int) as png_uint_16;

                            if v == (*png_ptr).trans_color.gray {
                                /* Background is already in screen gamma */
                                *sp = (((*png_ptr).background.gray as c_int >> 8) & 0xff) as png_byte;
                                *sp.offset(1) =
                                    ((*png_ptr).background.gray as c_int & 0xff) as png_byte;
                            } else {
                                v = *(*gamma_16
                                    .offset((*sp.offset(1) as c_int >> gamma_shift) as isize))
                                .offset(*sp as isize);
                                *sp = ((v as c_int >> 8) & 0xff) as png_byte;
                                *sp.offset(1) = (v as c_int & 0xff) as png_byte;
                            }

                            i += 1;
                            sp = sp.offset(2);
                        }
                    } else {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let v: png_uint_16;

                            v = (((*sp as c_int) << 8) + *sp.offset(1) as c_int) as png_uint_16;

                            if v == (*png_ptr).trans_color.gray {
                                *sp = (((*png_ptr).background.gray as c_int >> 8) & 0xff) as png_byte;
                                *sp.offset(1) =
                                    ((*png_ptr).background.gray as c_int & 0xff) as png_byte;
                            }

                            i += 1;
                            sp = sp.offset(2);
                        }
                    }
                }

                _ => {}
            }
        }

        PNG_COLOR_TYPE_RGB => {
            if (*row_info).bit_depth as c_int == 8 {
                if !gamma_table.is_null() {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        if *sp as c_int == (*png_ptr).trans_color.red as c_int
                            && *sp.offset(1) as c_int == (*png_ptr).trans_color.green as c_int
                            && *sp.offset(2) as c_int == (*png_ptr).trans_color.blue as c_int
                        {
                            *sp = (*png_ptr).background.red as png_byte;
                            *sp.offset(1) = (*png_ptr).background.green as png_byte;
                            *sp.offset(2) = (*png_ptr).background.blue as png_byte;
                        } else {
                            *sp = *gamma_table.offset(*sp as isize);
                            *sp.offset(1) = *gamma_table.offset(*sp.offset(1) as isize);
                            *sp.offset(2) = *gamma_table.offset(*sp.offset(2) as isize);
                        }

                        i += 1;
                        sp = sp.offset(3);
                    }
                } else {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        if *sp as c_int == (*png_ptr).trans_color.red as c_int
                            && *sp.offset(1) as c_int == (*png_ptr).trans_color.green as c_int
                            && *sp.offset(2) as c_int == (*png_ptr).trans_color.blue as c_int
                        {
                            *sp = (*png_ptr).background.red as png_byte;
                            *sp.offset(1) = (*png_ptr).background.green as png_byte;
                            *sp.offset(2) = (*png_ptr).background.blue as png_byte;
                        }

                        i += 1;
                        sp = sp.offset(3);
                    }
                }
            } else
            /* if (row_info->bit_depth == 16) */
            {
                if !gamma_16.is_null() {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let r: png_uint_16 =
                            (((*sp as c_int) << 8) + *sp.offset(1) as c_int) as png_uint_16;

                        let g: png_uint_16 = (((*sp.offset(2) as c_int) << 8)
                            + *sp.offset(3) as c_int) as png_uint_16;

                        let b: png_uint_16 = (((*sp.offset(4) as c_int) << 8)
                            + *sp.offset(5) as c_int) as png_uint_16;

                        if r == (*png_ptr).trans_color.red
                            && g == (*png_ptr).trans_color.green
                            && b == (*png_ptr).trans_color.blue
                        {
                            /* Background is already in screen gamma */
                            *sp = (((*png_ptr).background.red as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(1) = ((*png_ptr).background.red as c_int & 0xff) as png_byte;
                            *sp.offset(2) =
                                (((*png_ptr).background.green as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(3) =
                                ((*png_ptr).background.green as c_int & 0xff) as png_byte;
                            *sp.offset(4) =
                                (((*png_ptr).background.blue as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(5) = ((*png_ptr).background.blue as c_int & 0xff) as png_byte;
                        } else {
                            let mut v: png_uint_16 = *(*gamma_16
                                .offset((*sp.offset(1) as c_int >> gamma_shift) as isize))
                            .offset(*sp as isize);
                            *sp = ((v as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(1) = (v as c_int & 0xff) as png_byte;

                            v = *(*gamma_16
                                .offset((*sp.offset(3) as c_int >> gamma_shift) as isize))
                            .offset(*sp.offset(2) as isize);
                            *sp.offset(2) = ((v as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(3) = (v as c_int & 0xff) as png_byte;

                            v = *(*gamma_16
                                .offset((*sp.offset(5) as c_int >> gamma_shift) as isize))
                            .offset(*sp.offset(4) as isize);
                            *sp.offset(4) = ((v as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(5) = (v as c_int & 0xff) as png_byte;
                        }

                        i += 1;
                        sp = sp.offset(6);
                    }
                } else {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let r: png_uint_16 =
                            (((*sp as c_int) << 8) + *sp.offset(1) as c_int) as png_uint_16;

                        let g: png_uint_16 = (((*sp.offset(2) as c_int) << 8)
                            + *sp.offset(3) as c_int) as png_uint_16;

                        let b: png_uint_16 = (((*sp.offset(4) as c_int) << 8)
                            + *sp.offset(5) as c_int) as png_uint_16;

                        if r == (*png_ptr).trans_color.red
                            && g == (*png_ptr).trans_color.green
                            && b == (*png_ptr).trans_color.blue
                        {
                            *sp = (((*png_ptr).background.red as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(1) = ((*png_ptr).background.red as c_int & 0xff) as png_byte;
                            *sp.offset(2) =
                                (((*png_ptr).background.green as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(3) =
                                ((*png_ptr).background.green as c_int & 0xff) as png_byte;
                            *sp.offset(4) =
                                (((*png_ptr).background.blue as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(5) = ((*png_ptr).background.blue as c_int & 0xff) as png_byte;
                        }

                        i += 1;
                        sp = sp.offset(6);
                    }
                }
            }
        }

        PNG_COLOR_TYPE_GRAY_ALPHA => {
            if (*row_info).bit_depth as c_int == 8 {
                if !gamma_to_1.is_null() && !gamma_from_1.is_null() && !gamma_table.is_null() {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_uint_16 = *sp.offset(1) as png_uint_16;

                        if a == 0xff {
                            *sp = *gamma_table.offset(*sp as isize);
                        } else if a == 0 {
                            /* Background is already in screen gamma */
                            *sp = (*png_ptr).background.gray as png_byte;
                        } else {
                            let v: png_byte;
                            let mut w: png_byte;

                            v = *gamma_to_1.offset(*sp as isize);
                            /* png_composite(w, v, a, png_ptr->background_1.gray) */
                            {
                                let temp: png_uint_16 = ((v as png_uint_16 as c_int)
                                    * (a as png_uint_16 as c_int)
                                    + ((*png_ptr).background_1.gray as png_uint_16 as c_int)
                                        * ((255 - (a as png_uint_16 as c_int)) as png_uint_16
                                            as c_int)
                                    + 128) as png_uint_16;
                                w = (((temp as c_int + (temp as c_int >> 8)) >> 8) & 0xff)
                                    as png_byte;
                            }
                            if optimize == 0 {
                                w = *gamma_from_1.offset(w as isize);
                            }
                            *sp = w;
                        }

                        i += 1;
                        sp = sp.offset(2);
                    }
                } else {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_byte = *sp.offset(1);

                        if a == 0 {
                            *sp = (*png_ptr).background.gray as png_byte;
                        } else if (a as c_int) < 0xff {
                            /* png_composite(*sp, *sp, a, png_ptr->background.gray) */
                            let temp: png_uint_16 = ((*sp as png_uint_16 as c_int)
                                * (a as png_uint_16 as c_int)
                                + ((*png_ptr).background.gray as png_uint_16 as c_int)
                                    * ((255 - (a as png_uint_16 as c_int)) as png_uint_16 as c_int)
                                + 128) as png_uint_16;
                            *sp = (((temp as c_int + (temp as c_int >> 8)) >> 8) & 0xff) as png_byte;
                        }

                        i += 1;
                        sp = sp.offset(2);
                    }
                }
            } else
            /* if (png_ptr->bit_depth == 16) */
            {
                if !gamma_16.is_null() && !gamma_16_from_1.is_null() && !gamma_16_to_1.is_null() {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_uint_16 = (((*sp.offset(2) as c_int) << 8)
                            + *sp.offset(3) as c_int) as png_uint_16;

                        if a == 0xffff_u16 {
                            let v: png_uint_16;

                            v = *(*gamma_16
                                .offset((*sp.offset(1) as c_int >> gamma_shift) as isize))
                            .offset(*sp as isize);
                            *sp = ((v as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(1) = (v as c_int & 0xff) as png_byte;
                        } else if a == 0 {
                            /* Background is already in screen gamma */
                            *sp = (((*png_ptr).background.gray as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(1) = ((*png_ptr).background.gray as c_int & 0xff) as png_byte;
                        } else {
                            let g: png_uint_16;
                            let v: png_uint_16;
                            let w: png_uint_16;

                            g = *(*gamma_16_to_1
                                .offset((*sp.offset(1) as c_int >> gamma_shift) as isize))
                            .offset(*sp as isize);
                            /* png_composite_16(v, g, a, png_ptr->background_1.gray) */
                            {
                                let temp: png_uint_32 = (g as png_uint_32)
                                    .wrapping_mul(a as png_uint_32)
                                    .wrapping_add(
                                        ((*png_ptr).background_1.gray as png_uint_32).wrapping_mul(
                                            (65535_u32).wrapping_sub(a as png_uint_32),
                                        ),
                                    )
                                    .wrapping_add(32768);
                                v = (0xffff_u32
                                    & (temp.wrapping_add(temp >> 16) >> 16))
                                    as png_uint_16;
                            }
                            if optimize != 0 {
                                w = v;
                            } else {
                                w = *(*gamma_16_from_1
                                    .offset(((v as c_int & 0xff) >> gamma_shift) as isize))
                                .offset((v as c_int >> 8) as isize);
                            }
                            *sp = ((w as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(1) = (w as c_int & 0xff) as png_byte;
                        }

                        i += 1;
                        sp = sp.offset(4);
                    }
                } else {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_uint_16 = (((*sp.offset(2) as c_int) << 8)
                            + *sp.offset(3) as c_int) as png_uint_16;

                        if a == 0 {
                            *sp = (((*png_ptr).background.gray as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(1) = ((*png_ptr).background.gray as c_int & 0xff) as png_byte;
                        } else if (a as c_int) < 0xffff {
                            let g: png_uint_16;
                            let v: png_uint_16;

                            g = (((*sp as c_int) << 8) + *sp.offset(1) as c_int) as png_uint_16;
                            /* png_composite_16(v, g, a, png_ptr->background.gray) */
                            {
                                let temp: png_uint_32 = (g as png_uint_32)
                                    .wrapping_mul(a as png_uint_32)
                                    .wrapping_add(
                                        ((*png_ptr).background.gray as png_uint_32).wrapping_mul(
                                            (65535_u32).wrapping_sub(a as png_uint_32),
                                        ),
                                    )
                                    .wrapping_add(32768);
                                v = (0xffff_u32
                                    & (temp.wrapping_add(temp >> 16) >> 16))
                                    as png_uint_16;
                            }
                            *sp = ((v as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(1) = (v as c_int & 0xff) as png_byte;
                        }

                        i += 1;
                        sp = sp.offset(4);
                    }
                }
            }
        }

        PNG_COLOR_TYPE_RGB_ALPHA => {
            if (*row_info).bit_depth as c_int == 8 {
                if !gamma_to_1.is_null() && !gamma_from_1.is_null() && !gamma_table.is_null() {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_byte = *sp.offset(3);

                        if a as c_int == 0xff {
                            *sp = *gamma_table.offset(*sp as isize);
                            *sp.offset(1) = *gamma_table.offset(*sp.offset(1) as isize);
                            *sp.offset(2) = *gamma_table.offset(*sp.offset(2) as isize);
                        } else if a == 0 {
                            /* Background is already in screen gamma */
                            *sp = (*png_ptr).background.red as png_byte;
                            *sp.offset(1) = (*png_ptr).background.green as png_byte;
                            *sp.offset(2) = (*png_ptr).background.blue as png_byte;
                        } else {
                            let mut v: png_byte;
                            let mut w: png_byte;

                            v = *gamma_to_1.offset(*sp as isize);
                            /* png_composite(w, v, a, png_ptr->background_1.red) */
                            {
                                let temp: png_uint_16 = ((v as png_uint_16 as c_int)
                                    * (a as png_uint_16 as c_int)
                                    + ((*png_ptr).background_1.red as png_uint_16 as c_int)
                                        * ((255 - (a as png_uint_16 as c_int)) as png_uint_16
                                            as c_int)
                                    + 128) as png_uint_16;
                                w = (((temp as c_int + (temp as c_int >> 8)) >> 8) & 0xff)
                                    as png_byte;
                            }
                            if optimize == 0 {
                                w = *gamma_from_1.offset(w as isize);
                            }
                            *sp = w;

                            v = *gamma_to_1.offset(*sp.offset(1) as isize);
                            /* png_composite(w, v, a, png_ptr->background_1.green) */
                            {
                                let temp: png_uint_16 = ((v as png_uint_16 as c_int)
                                    * (a as png_uint_16 as c_int)
                                    + ((*png_ptr).background_1.green as png_uint_16 as c_int)
                                        * ((255 - (a as png_uint_16 as c_int)) as png_uint_16
                                            as c_int)
                                    + 128) as png_uint_16;
                                w = (((temp as c_int + (temp as c_int >> 8)) >> 8) & 0xff)
                                    as png_byte;
                            }
                            if optimize == 0 {
                                w = *gamma_from_1.offset(w as isize);
                            }
                            *sp.offset(1) = w;

                            v = *gamma_to_1.offset(*sp.offset(2) as isize);
                            /* png_composite(w, v, a, png_ptr->background_1.blue) */
                            {
                                let temp: png_uint_16 = ((v as png_uint_16 as c_int)
                                    * (a as png_uint_16 as c_int)
                                    + ((*png_ptr).background_1.blue as png_uint_16 as c_int)
                                        * ((255 - (a as png_uint_16 as c_int)) as png_uint_16
                                            as c_int)
                                    + 128) as png_uint_16;
                                w = (((temp as c_int + (temp as c_int >> 8)) >> 8) & 0xff)
                                    as png_byte;
                            }
                            if optimize == 0 {
                                w = *gamma_from_1.offset(w as isize);
                            }
                            *sp.offset(2) = w;
                        }

                        i += 1;
                        sp = sp.offset(4);
                    }
                } else {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_byte = *sp.offset(3);

                        if a == 0 {
                            *sp = (*png_ptr).background.red as png_byte;
                            *sp.offset(1) = (*png_ptr).background.green as png_byte;
                            *sp.offset(2) = (*png_ptr).background.blue as png_byte;
                        } else if (a as c_int) < 0xff {
                            /* png_composite(*sp, *sp, a, png_ptr->background.red) */
                            {
                                let temp: png_uint_16 = ((*sp as png_uint_16 as c_int)
                                    * (a as png_uint_16 as c_int)
                                    + ((*png_ptr).background.red as png_uint_16 as c_int)
                                        * ((255 - (a as png_uint_16 as c_int)) as png_uint_16
                                            as c_int)
                                    + 128) as png_uint_16;
                                *sp = (((temp as c_int + (temp as c_int >> 8)) >> 8) & 0xff)
                                    as png_byte;
                            }

                            /* png_composite(*(sp + 1), *(sp + 1), a,
                             *     png_ptr->background.green)
                             */
                            {
                                let temp: png_uint_16 = ((*sp.offset(1) as png_uint_16 as c_int)
                                    * (a as png_uint_16 as c_int)
                                    + ((*png_ptr).background.green as png_uint_16 as c_int)
                                        * ((255 - (a as png_uint_16 as c_int)) as png_uint_16
                                            as c_int)
                                    + 128) as png_uint_16;
                                *sp.offset(1) = (((temp as c_int + (temp as c_int >> 8)) >> 8)
                                    & 0xff) as png_byte;
                            }

                            /* png_composite(*(sp + 2), *(sp + 2), a,
                             *     png_ptr->background.blue)
                             */
                            {
                                let temp: png_uint_16 = ((*sp.offset(2) as png_uint_16 as c_int)
                                    * (a as png_uint_16 as c_int)
                                    + ((*png_ptr).background.blue as png_uint_16 as c_int)
                                        * ((255 - (a as png_uint_16 as c_int)) as png_uint_16
                                            as c_int)
                                    + 128) as png_uint_16;
                                *sp.offset(2) = (((temp as c_int + (temp as c_int >> 8)) >> 8)
                                    & 0xff) as png_byte;
                            }
                        }

                        i += 1;
                        sp = sp.offset(4);
                    }
                }
            } else
            /* if (row_info->bit_depth == 16) */
            {
                if !gamma_16.is_null() && !gamma_16_from_1.is_null() && !gamma_16_to_1.is_null() {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_uint_16 = (((*sp.offset(6) as png_uint_16 as c_int) << 8)
                            + (*sp.offset(7) as png_uint_16 as c_int))
                            as png_uint_16;

                        if a == 0xffff_u16 {
                            let mut v: png_uint_16;

                            v = *(*gamma_16
                                .offset((*sp.offset(1) as c_int >> gamma_shift) as isize))
                            .offset(*sp as isize);
                            *sp = ((v as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(1) = (v as c_int & 0xff) as png_byte;

                            v = *(*gamma_16
                                .offset((*sp.offset(3) as c_int >> gamma_shift) as isize))
                            .offset(*sp.offset(2) as isize);
                            *sp.offset(2) = ((v as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(3) = (v as c_int & 0xff) as png_byte;

                            v = *(*gamma_16
                                .offset((*sp.offset(5) as c_int >> gamma_shift) as isize))
                            .offset(*sp.offset(4) as isize);
                            *sp.offset(4) = ((v as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(5) = (v as c_int & 0xff) as png_byte;
                        } else if a == 0 {
                            /* Background is already in screen gamma */
                            *sp = (((*png_ptr).background.red as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(1) = ((*png_ptr).background.red as c_int & 0xff) as png_byte;
                            *sp.offset(2) =
                                (((*png_ptr).background.green as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(3) =
                                ((*png_ptr).background.green as c_int & 0xff) as png_byte;
                            *sp.offset(4) =
                                (((*png_ptr).background.blue as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(5) = ((*png_ptr).background.blue as c_int & 0xff) as png_byte;
                        } else {
                            let mut v: png_uint_16;
                            let mut w: png_uint_16;

                            v = *(*gamma_16_to_1
                                .offset((*sp.offset(1) as c_int >> gamma_shift) as isize))
                            .offset(*sp as isize);
                            /* png_composite_16(w, v, a, png_ptr->background_1.red) */
                            {
                                let temp: png_uint_32 = (v as png_uint_32)
                                    .wrapping_mul(a as png_uint_32)
                                    .wrapping_add(
                                        ((*png_ptr).background_1.red as png_uint_32).wrapping_mul(
                                            (65535_u32).wrapping_sub(a as png_uint_32),
                                        ),
                                    )
                                    .wrapping_add(32768);
                                w = (0xffff_u32 & (temp.wrapping_add(temp >> 16) >> 16))
                                    as png_uint_16;
                            }
                            if optimize == 0 {
                                w = *(*gamma_16_from_1
                                    .offset(((w as c_int & 0xff) >> gamma_shift) as isize))
                                .offset((w as c_int >> 8) as isize);
                            }
                            *sp = ((w as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(1) = (w as c_int & 0xff) as png_byte;

                            v = *(*gamma_16_to_1
                                .offset((*sp.offset(3) as c_int >> gamma_shift) as isize))
                            .offset(*sp.offset(2) as isize);
                            /* png_composite_16(w, v, a, png_ptr->background_1.green) */
                            {
                                let temp: png_uint_32 = (v as png_uint_32)
                                    .wrapping_mul(a as png_uint_32)
                                    .wrapping_add(
                                        ((*png_ptr).background_1.green as png_uint_32)
                                            .wrapping_mul(
                                                (65535_u32).wrapping_sub(a as png_uint_32),
                                            ),
                                    )
                                    .wrapping_add(32768);
                                w = (0xffff_u32 & (temp.wrapping_add(temp >> 16) >> 16))
                                    as png_uint_16;
                            }
                            if optimize == 0 {
                                w = *(*gamma_16_from_1
                                    .offset(((w as c_int & 0xff) >> gamma_shift) as isize))
                                .offset((w as c_int >> 8) as isize);
                            }

                            *sp.offset(2) = ((w as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(3) = (w as c_int & 0xff) as png_byte;

                            v = *(*gamma_16_to_1
                                .offset((*sp.offset(5) as c_int >> gamma_shift) as isize))
                            .offset(*sp.offset(4) as isize);
                            /* png_composite_16(w, v, a, png_ptr->background_1.blue) */
                            {
                                let temp: png_uint_32 = (v as png_uint_32)
                                    .wrapping_mul(a as png_uint_32)
                                    .wrapping_add(
                                        ((*png_ptr).background_1.blue as png_uint_32).wrapping_mul(
                                            (65535_u32).wrapping_sub(a as png_uint_32),
                                        ),
                                    )
                                    .wrapping_add(32768);
                                w = (0xffff_u32 & (temp.wrapping_add(temp >> 16) >> 16))
                                    as png_uint_16;
                            }
                            if optimize == 0 {
                                w = *(*gamma_16_from_1
                                    .offset(((w as c_int & 0xff) >> gamma_shift) as isize))
                                .offset((w as c_int >> 8) as isize);
                            }

                            *sp.offset(4) = ((w as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(5) = (w as c_int & 0xff) as png_byte;
                        }

                        i += 1;
                        sp = sp.offset(8);
                    }
                } else {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_uint_16 = (((*sp.offset(6) as png_uint_16 as c_int) << 8)
                            + (*sp.offset(7) as png_uint_16 as c_int))
                            as png_uint_16;

                        if a == 0 {
                            *sp = (((*png_ptr).background.red as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(1) = ((*png_ptr).background.red as c_int & 0xff) as png_byte;
                            *sp.offset(2) =
                                (((*png_ptr).background.green as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(3) =
                                ((*png_ptr).background.green as c_int & 0xff) as png_byte;
                            *sp.offset(4) =
                                (((*png_ptr).background.blue as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(5) = ((*png_ptr).background.blue as c_int & 0xff) as png_byte;
                        } else if (a as c_int) < 0xffff {
                            let mut v: png_uint_16;

                            let r: png_uint_16 =
                                (((*sp as c_int) << 8) + *sp.offset(1) as c_int) as png_uint_16;
                            let g: png_uint_16 = (((*sp.offset(2) as c_int) << 8)
                                + *sp.offset(3) as c_int) as png_uint_16;
                            let b: png_uint_16 = (((*sp.offset(4) as c_int) << 8)
                                + *sp.offset(5) as c_int) as png_uint_16;

                            /* png_composite_16(v, r, a, png_ptr->background.red) */
                            {
                                let temp: png_uint_32 = (r as png_uint_32)
                                    .wrapping_mul(a as png_uint_32)
                                    .wrapping_add(
                                        ((*png_ptr).background.red as png_uint_32).wrapping_mul(
                                            (65535_u32).wrapping_sub(a as png_uint_32),
                                        ),
                                    )
                                    .wrapping_add(32768);
                                v = (0xffff_u32 & (temp.wrapping_add(temp >> 16) >> 16))
                                    as png_uint_16;
                            }
                            *sp = ((v as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(1) = (v as c_int & 0xff) as png_byte;

                            /* png_composite_16(v, g, a, png_ptr->background.green) */
                            {
                                let temp: png_uint_32 = (g as png_uint_32)
                                    .wrapping_mul(a as png_uint_32)
                                    .wrapping_add(
                                        ((*png_ptr).background.green as png_uint_32).wrapping_mul(
                                            (65535_u32).wrapping_sub(a as png_uint_32),
                                        ),
                                    )
                                    .wrapping_add(32768);
                                v = (0xffff_u32 & (temp.wrapping_add(temp >> 16) >> 16))
                                    as png_uint_16;
                            }
                            *sp.offset(2) = ((v as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(3) = (v as c_int & 0xff) as png_byte;

                            /* png_composite_16(v, b, a, png_ptr->background.blue) */
                            {
                                let temp: png_uint_32 = (b as png_uint_32)
                                    .wrapping_mul(a as png_uint_32)
                                    .wrapping_add(
                                        ((*png_ptr).background.blue as png_uint_32).wrapping_mul(
                                            (65535_u32).wrapping_sub(a as png_uint_32),
                                        ),
                                    )
                                    .wrapping_add(32768);
                                v = (0xffff_u32 & (temp.wrapping_add(temp >> 16) >> 16))
                                    as png_uint_16;
                            }
                            *sp.offset(4) = ((v as c_int >> 8) & 0xff) as png_byte;
                            *sp.offset(5) = (v as c_int & 0xff) as png_byte;
                        }

                        i += 1;
                        sp = sp.offset(8);
                    }
                }
            }
        }

        _ => {}
    }
}

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

    if ((*row_info).bit_depth as c_int <= 8 && !gamma_table.is_null())
        || ((*row_info).bit_depth as c_int == 16 && !gamma_16_table.is_null())
    {
        match (*row_info).color_type as c_int {
            PNG_COLOR_TYPE_RGB => {
                if (*row_info).bit_depth as c_int == 8 {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(1);
                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(1);
                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(1);
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
                            .offset((*sp.offset(1) as c_int >> gamma_shift) as isize))
                        .offset(*sp as isize);
                        *sp = ((v as c_int >> 8) & 0xff) as png_byte;
                        *sp.offset(1) = (v as c_int & 0xff) as png_byte;
                        sp = sp.offset(2);

                        v = *(*gamma_16_table
                            .offset((*sp.offset(1) as c_int >> gamma_shift) as isize))
                        .offset(*sp as isize);
                        *sp = ((v as c_int >> 8) & 0xff) as png_byte;
                        *sp.offset(1) = (v as c_int & 0xff) as png_byte;
                        sp = sp.offset(2);

                        v = *(*gamma_16_table
                            .offset((*sp.offset(1) as c_int >> gamma_shift) as isize))
                        .offset(*sp as isize);
                        *sp = ((v as c_int >> 8) & 0xff) as png_byte;
                        *sp.offset(1) = (v as c_int & 0xff) as png_byte;
                        sp = sp.offset(2);

                        i += 1;
                    }
                }
            }

            PNG_COLOR_TYPE_RGB_ALPHA => {
                if (*row_info).bit_depth as c_int == 8 {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(1);

                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(1);

                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(1);

                        sp = sp.offset(1);
                        i += 1;
                    }
                } else
                /* if (row_info->bit_depth == 16) */
                {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let mut v: png_uint_16 = *(*gamma_16_table
                            .offset((*sp.offset(1) as c_int >> gamma_shift) as isize))
                        .offset(*sp as isize);
                        *sp = ((v as c_int >> 8) & 0xff) as png_byte;
                        *sp.offset(1) = (v as c_int & 0xff) as png_byte;
                        sp = sp.offset(2);

                        v = *(*gamma_16_table
                            .offset((*sp.offset(1) as c_int >> gamma_shift) as isize))
                        .offset(*sp as isize);
                        *sp = ((v as c_int >> 8) & 0xff) as png_byte;
                        *sp.offset(1) = (v as c_int & 0xff) as png_byte;
                        sp = sp.offset(2);

                        v = *(*gamma_16_table
                            .offset((*sp.offset(1) as c_int >> gamma_shift) as isize))
                        .offset(*sp as isize);
                        *sp = ((v as c_int >> 8) & 0xff) as png_byte;
                        *sp.offset(1) = (v as c_int & 0xff) as png_byte;
                        sp = sp.offset(4);

                        i += 1;
                    }
                }
            }

            PNG_COLOR_TYPE_GRAY_ALPHA => {
                if (*row_info).bit_depth as c_int == 8 {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(2);
                        i += 1;
                    }
                } else
                /* if (row_info->bit_depth == 16) */
                {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let v: png_uint_16 = *(*gamma_16_table
                            .offset((*sp.offset(1) as c_int >> gamma_shift) as isize))
                        .offset(*sp as isize);
                        *sp = ((v as c_int >> 8) & 0xff) as png_byte;
                        *sp.offset(1) = (v as c_int & 0xff) as png_byte;
                        sp = sp.offset(4);
                        i += 1;
                    }
                }
            }

            PNG_COLOR_TYPE_GRAY => {
                if (*row_info).bit_depth as c_int == 2 {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: c_int = *sp as c_int & 0xc0;
                        let b: c_int = *sp as c_int & 0x30;
                        let c: c_int = *sp as c_int & 0x0c;
                        let d: c_int = *sp as c_int & 0x03;

                        *sp = ((((*gamma_table
                            .offset((a | (a >> 2) | (a >> 4) | (a >> 6)) as isize)
                            as c_int))
                            & 0xc0)
                            | (((*gamma_table
                                .offset(((b << 2) | b | (b >> 2) | (b >> 4)) as isize)
                                as c_int)
                                >> 2)
                                & 0x30)
                            | (((*gamma_table
                                .offset(((c << 4) | (c << 2) | c | (c >> 2)) as isize)
                                as c_int)
                                >> 4)
                                & 0x0c)
                            | ((*gamma_table
                                .offset(((d << 6) | (d << 4) | (d << 2) | d) as isize)
                                as c_int)
                                >> 6)) as png_byte;
                        sp = sp.offset(1);
                        i += 4;
                    }
                }

                if (*row_info).bit_depth as c_int == 4 {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let msb: c_int = *sp as c_int & 0xf0;
                        let lsb: c_int = *sp as c_int & 0x0f;

                        *sp = (((*gamma_table.offset((msb | (msb >> 4)) as isize) as c_int) & 0xf0)
                            | ((*gamma_table.offset(((lsb << 4) | lsb) as isize) as c_int) >> 4))
                            as png_byte;
                        sp = sp.offset(1);
                        i += 2;
                    }
                } else if (*row_info).bit_depth as c_int == 8 {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(1);
                        i += 1;
                    }
                } else if (*row_info).bit_depth as c_int == 16 {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let v: png_uint_16 = *(*gamma_16_table
                            .offset((*sp.offset(1) as c_int >> gamma_shift) as isize))
                        .offset(*sp as isize);
                        *sp = ((v as c_int >> 8) & 0xff) as png_byte;
                        *sp.offset(1) = (v as c_int & 0xff) as png_byte;
                        sp = sp.offset(2);
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

    if ((*row_info).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0 {
        if (*row_info).bit_depth as c_int == 8 {
            let table: png_bytep = (*png_ptr).gamma_from_1;

            if !table.is_null() {
                let step: c_int = if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
                    4
                } else {
                    2
                };

                /* The alpha channel is the last component: */
                row = row.offset((step - 1) as isize);

                while row_width > 0 {
                    *row = *table.offset(*row as isize);
                    row_width -= 1;
                    row = row.offset(step as isize);
                }

                return;
            }
        } else if (*row_info).bit_depth as c_int == 16 {
            let table: png_uint_16pp = (*png_ptr).gamma_16_from_1;
            let gamma_shift: c_int = (*png_ptr).gamma_shift;

            if !table.is_null() {
                let step: c_int = if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
                    8
                } else {
                    4
                };

                /* The alpha channel is the last component: */
                row = row.offset((step - 2) as isize);

                while row_width > 0 {
                    let v: png_uint_16;

                    v = *(*table.offset((*row.offset(1) as c_int >> gamma_shift) as isize))
                        .offset(*row as isize);
                    *row = ((v as c_int >> 8) & 0xff) as png_byte;
                    *row.offset(1) = (v as c_int & 0xff) as png_byte;

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
    png_warning(png_ptr, cstr!("png_do_encode_alpha: unexpected call"));
}
