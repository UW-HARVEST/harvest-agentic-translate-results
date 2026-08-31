//! pngrtran.c lines 3334-4343: png_do_compose, png_do_gamma, png_do_encode_alpha.
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

/* Replace any alpha or transparency with the supplied background color.
 * "background" is already in the screen gamma, while "background_1" is
 * at a gamma of 1.0.  Paletted files have already been taken care of.
 */
pub unsafe fn png_do_compose(row_info: png_row_infop, row: png_bytep, png_ptr: png_structrp) {
    let gamma_table: png_const_bytep = (*png_ptr).gamma_table;
    let gamma_from_1: png_const_bytep = (*png_ptr).gamma_from_1;
    let gamma_to_1: png_const_bytep = (*png_ptr).gamma_to_1;
    let gamma_16: png_uint_16pp = (*png_ptr).gamma_16_table;
    let gamma_16_from_1: png_uint_16pp = (*png_ptr).gamma_16_from_1;
    let gamma_16_to_1: png_uint_16pp = (*png_ptr).gamma_16_to_1;
    let gamma_shift: c_int = (*png_ptr).gamma_shift;
    let optimize: c_int = if ((*png_ptr).flags & PNG_FLAG_OPTIMIZE_ALPHA) != 0 {
        1
    } else {
        0
    };

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
                        if ((((*sp as c_int) >> shift) & 0x01) as png_uint_16)
                            == (*png_ptr).trans_color.gray
                        {
                            let mut tmp: c_uint =
                                ((*sp as c_int) & (0x7f7f >> (7 - shift))) as c_uint;
                            tmp |= (((*png_ptr).background.gray as c_int) << shift) as c_uint;
                            *sp = (tmp & 0xff) as png_byte;
                        }

                        if shift == 0 {
                            shift = 7;
                            sp = sp.add(1);
                        } else {
                            shift -= 1;
                        }

                        i += 1;
                    }
                }

                2 => {
                    if gamma_table != core::ptr::null() {
                        sp = row;
                        shift = 6;
                        i = 0;
                        while i < row_width {
                            if ((((*sp as c_int) >> shift) & 0x03) as png_uint_16)
                                == (*png_ptr).trans_color.gray
                            {
                                let mut tmp: c_uint =
                                    ((*sp as c_int) & (0x3f3f >> (6 - shift))) as c_uint;
                                tmp |= ((*png_ptr).background.gray as c_uint) << shift;
                                *sp = (tmp & 0xff) as png_byte;
                            } else {
                                let p: c_uint = (((*sp as c_int) >> shift) & 0x03) as c_uint;
                                let g: c_uint = (((*gamma_table
                                    .add((p | (p << 2) | (p << 4) | (p << 6)) as usize)
                                    as c_int)
                                    >> 6)
                                    & 0x03) as c_uint;
                                let mut tmp: c_uint =
                                    ((*sp as c_int) & (0x3f3f >> (6 - shift))) as c_uint;
                                tmp |= g << shift;
                                *sp = (tmp & 0xff) as png_byte;
                            }

                            if shift == 0 {
                                shift = 6;
                                sp = sp.add(1);
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
                            if ((((*sp as c_int) >> shift) & 0x03) as png_uint_16)
                                == (*png_ptr).trans_color.gray
                            {
                                let mut tmp: c_uint =
                                    ((*sp as c_int) & (0x3f3f >> (6 - shift))) as c_uint;
                                tmp |= ((*png_ptr).background.gray as c_uint) << shift;
                                *sp = (tmp & 0xff) as png_byte;
                            }

                            if shift == 0 {
                                shift = 6;
                                sp = sp.add(1);
                            } else {
                                shift -= 2;
                            }

                            i += 1;
                        }
                    }
                }

                4 => {
                    if gamma_table != core::ptr::null() {
                        sp = row;
                        shift = 4;
                        i = 0;
                        while i < row_width {
                            if ((((*sp as c_int) >> shift) & 0x0f) as png_uint_16)
                                == (*png_ptr).trans_color.gray
                            {
                                let mut tmp: c_uint =
                                    ((*sp as c_int) & (0x0f0f >> (4 - shift))) as c_uint;
                                tmp |= (((*png_ptr).background.gray as c_int) << shift) as c_uint;
                                *sp = (tmp & 0xff) as png_byte;
                            } else {
                                let p: c_uint = (((*sp as c_int) >> shift) & 0x0f) as c_uint;
                                let g: c_uint = (((*gamma_table.add((p | (p << 4)) as usize)
                                    as c_int)
                                    >> 4)
                                    & 0x0f) as c_uint;
                                let mut tmp: c_uint =
                                    ((*sp as c_int) & (0x0f0f >> (4 - shift))) as c_uint;
                                tmp |= g << shift;
                                *sp = (tmp & 0xff) as png_byte;
                            }

                            if shift == 0 {
                                shift = 4;
                                sp = sp.add(1);
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
                            if ((((*sp as c_int) >> shift) & 0x0f) as png_uint_16)
                                == (*png_ptr).trans_color.gray
                            {
                                let mut tmp: c_uint =
                                    ((*sp as c_int) & (0x0f0f >> (4 - shift))) as c_uint;
                                tmp |= (((*png_ptr).background.gray as c_int) << shift) as c_uint;
                                *sp = (tmp & 0xff) as png_byte;
                            }

                            if shift == 0 {
                                shift = 4;
                                sp = sp.add(1);
                            } else {
                                shift -= 4;
                            }

                            i += 1;
                        }
                    }
                }

                8 => {
                    if gamma_table != core::ptr::null() {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            if (*sp as png_uint_16) == (*png_ptr).trans_color.gray {
                                *sp = (*png_ptr).background.gray as png_byte;
                            } else {
                                *sp = *gamma_table.add(*sp as usize);
                            }

                            i += 1;
                            sp = sp.add(1);
                        }
                    } else {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            if (*sp as png_uint_16) == (*png_ptr).trans_color.gray {
                                *sp = (*png_ptr).background.gray as png_byte;
                            }

                            i += 1;
                            sp = sp.add(1);
                        }
                    }
                }

                16 => {
                    if gamma_16 != core::ptr::null_mut() {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let mut v: png_uint_16;

                            v = (((*sp as c_int) << 8) + (*sp.add(1) as c_int)) as png_uint_16;

                            if v == (*png_ptr).trans_color.gray {
                                /* Background is already in screen gamma */
                                *sp = ((((*png_ptr).background.gray as c_int) >> 8) & 0xff)
                                    as png_byte;
                                *sp.add(1) =
                                    (((*png_ptr).background.gray as c_int) & 0xff) as png_byte;
                            } else {
                                v = *(*gamma_16
                                    .add((((*sp.add(1) as c_int) >> gamma_shift) as usize)))
                                .add(*sp as usize);
                                *sp = (((v as c_int) >> 8) & 0xff) as png_byte;
                                *sp.add(1) = ((v as c_int) & 0xff) as png_byte;
                            }

                            i += 1;
                            sp = sp.add(2);
                        }
                    } else {
                        sp = row;
                        i = 0;
                        while i < row_width {
                            let v: png_uint_16;

                            v = (((*sp as c_int) << 8) + (*sp.add(1) as c_int)) as png_uint_16;

                            if v == (*png_ptr).trans_color.gray {
                                *sp = ((((*png_ptr).background.gray as c_int) >> 8) & 0xff)
                                    as png_byte;
                                *sp.add(1) =
                                    (((*png_ptr).background.gray as c_int) & 0xff) as png_byte;
                            }

                            i += 1;
                            sp = sp.add(2);
                        }
                    }
                }

                _ => {}
            }
        }

        PNG_COLOR_TYPE_RGB => {
            if (*row_info).bit_depth as c_int == 8 {
                if gamma_table != core::ptr::null() {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        if (*sp as png_uint_16) == (*png_ptr).trans_color.red
                            && (*sp.add(1) as png_uint_16) == (*png_ptr).trans_color.green
                            && (*sp.add(2) as png_uint_16) == (*png_ptr).trans_color.blue
                        {
                            *sp = (*png_ptr).background.red as png_byte;
                            *sp.add(1) = (*png_ptr).background.green as png_byte;
                            *sp.add(2) = (*png_ptr).background.blue as png_byte;
                        } else {
                            *sp = *gamma_table.add(*sp as usize);
                            *sp.add(1) = *gamma_table.add(*sp.add(1) as usize);
                            *sp.add(2) = *gamma_table.add(*sp.add(2) as usize);
                        }

                        i += 1;
                        sp = sp.add(3);
                    }
                } else {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        if (*sp as png_uint_16) == (*png_ptr).trans_color.red
                            && (*sp.add(1) as png_uint_16) == (*png_ptr).trans_color.green
                            && (*sp.add(2) as png_uint_16) == (*png_ptr).trans_color.blue
                        {
                            *sp = (*png_ptr).background.red as png_byte;
                            *sp.add(1) = (*png_ptr).background.green as png_byte;
                            *sp.add(2) = (*png_ptr).background.blue as png_byte;
                        }

                        i += 1;
                        sp = sp.add(3);
                    }
                }
            } else
            /* if (row_info->bit_depth == 16) */
            {
                if gamma_16 != core::ptr::null_mut() {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let r: png_uint_16 =
                            (((*sp as c_int) << 8) + (*sp.add(1) as c_int)) as png_uint_16;

                        let g: png_uint_16 =
                            (((*sp.add(2) as c_int) << 8) + (*sp.add(3) as c_int)) as png_uint_16;

                        let b: png_uint_16 =
                            (((*sp.add(4) as c_int) << 8) + (*sp.add(5) as c_int)) as png_uint_16;

                        if r == (*png_ptr).trans_color.red
                            && g == (*png_ptr).trans_color.green
                            && b == (*png_ptr).trans_color.blue
                        {
                            /* Background is already in screen gamma */
                            *sp =
                                ((((*png_ptr).background.red as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(1) = (((*png_ptr).background.red as c_int) & 0xff) as png_byte;
                            *sp.add(2) =
                                ((((*png_ptr).background.green as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(3) =
                                (((*png_ptr).background.green as c_int) & 0xff) as png_byte;
                            *sp.add(4) =
                                ((((*png_ptr).background.blue as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(5) = (((*png_ptr).background.blue as c_int) & 0xff) as png_byte;
                        } else {
                            let mut v: png_uint_16 = *(*gamma_16
                                .add(((*sp.add(1) as c_int) >> gamma_shift) as usize))
                            .add(*sp as usize);
                            *sp = (((v as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(1) = ((v as c_int) & 0xff) as png_byte;

                            v = *(*gamma_16.add(((*sp.add(3) as c_int) >> gamma_shift) as usize))
                                .add(*sp.add(2) as usize);
                            *sp.add(2) = (((v as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(3) = ((v as c_int) & 0xff) as png_byte;

                            v = *(*gamma_16.add(((*sp.add(5) as c_int) >> gamma_shift) as usize))
                                .add(*sp.add(4) as usize);
                            *sp.add(4) = (((v as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(5) = ((v as c_int) & 0xff) as png_byte;
                        }

                        i += 1;
                        sp = sp.add(6);
                    }
                } else {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let r: png_uint_16 =
                            (((*sp as c_int) << 8) + (*sp.add(1) as c_int)) as png_uint_16;

                        let g: png_uint_16 =
                            (((*sp.add(2) as c_int) << 8) + (*sp.add(3) as c_int)) as png_uint_16;

                        let b: png_uint_16 =
                            (((*sp.add(4) as c_int) << 8) + (*sp.add(5) as c_int)) as png_uint_16;

                        if r == (*png_ptr).trans_color.red
                            && g == (*png_ptr).trans_color.green
                            && b == (*png_ptr).trans_color.blue
                        {
                            *sp =
                                ((((*png_ptr).background.red as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(1) = (((*png_ptr).background.red as c_int) & 0xff) as png_byte;
                            *sp.add(2) =
                                ((((*png_ptr).background.green as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(3) =
                                (((*png_ptr).background.green as c_int) & 0xff) as png_byte;
                            *sp.add(4) =
                                ((((*png_ptr).background.blue as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(5) = (((*png_ptr).background.blue as c_int) & 0xff) as png_byte;
                        }

                        i += 1;
                        sp = sp.add(6);
                    }
                }
            }
        }

        PNG_COLOR_TYPE_GRAY_ALPHA => {
            if (*row_info).bit_depth as c_int == 8 {
                if gamma_to_1 != core::ptr::null()
                    && gamma_from_1 != core::ptr::null()
                    && gamma_table != core::ptr::null()
                {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_uint_16 = *sp.add(1) as png_uint_16;

                        if a == 0xff {
                            *sp = *gamma_table.add(*sp as usize);
                        } else if a == 0 {
                            /* Background is already in screen gamma */
                            *sp = (*png_ptr).background.gray as png_byte;
                        } else {
                            let v: png_byte;
                            let mut w: png_byte;

                            v = *gamma_to_1.add(*sp as usize);
                            w = png_composite(
                                v as png_uint_16,
                                a as png_uint_16,
                                (*png_ptr).background_1.gray as png_uint_16,
                            );
                            if optimize == 0 {
                                w = *gamma_from_1.add(w as usize);
                            }
                            *sp = w;
                        }

                        i += 1;
                        sp = sp.add(2);
                    }
                } else {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_byte = *sp.add(1);

                        if a == 0 {
                            *sp = (*png_ptr).background.gray as png_byte;
                        } else if a < 0xff {
                            *sp = png_composite(
                                *sp as png_uint_16,
                                a as png_uint_16,
                                (*png_ptr).background.gray as png_uint_16,
                            );
                        }

                        i += 1;
                        sp = sp.add(2);
                    }
                }
            } else
            /* if (png_ptr->bit_depth == 16) */
            {
                if gamma_16 != core::ptr::null_mut()
                    && gamma_16_from_1 != core::ptr::null_mut()
                    && gamma_16_to_1 != core::ptr::null_mut()
                {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_uint_16 =
                            (((*sp.add(2) as c_int) << 8) + (*sp.add(3) as c_int)) as png_uint_16;

                        if a == 0xffff as png_uint_16 {
                            let v: png_uint_16;

                            v = *(*gamma_16.add(((*sp.add(1) as c_int) >> gamma_shift) as usize))
                                .add(*sp as usize);
                            *sp = (((v as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(1) = ((v as c_int) & 0xff) as png_byte;
                        } else if a == 0 {
                            /* Background is already in screen gamma */
                            *sp =
                                ((((*png_ptr).background.gray as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(1) = (((*png_ptr).background.gray as c_int) & 0xff) as png_byte;
                        } else {
                            let g: png_uint_16;
                            let v: png_uint_16;
                            let w: png_uint_16;

                            g = *(*gamma_16_to_1
                                .add(((*sp.add(1) as c_int) >> gamma_shift) as usize))
                            .add(*sp as usize);
                            v = png_composite_16(
                                g as png_uint_32,
                                a as png_uint_32,
                                (*png_ptr).background_1.gray as png_uint_32,
                            );
                            if optimize != 0 {
                                w = v;
                            } else {
                                w = *(*gamma_16_from_1
                                    .add(((((v as c_int) & 0xff) >> gamma_shift) as usize)))
                                .add(((v as c_int) >> 8) as usize);
                            }
                            *sp = (((w as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(1) = ((w as c_int) & 0xff) as png_byte;
                        }

                        i += 1;
                        sp = sp.add(4);
                    }
                } else {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_uint_16 =
                            (((*sp.add(2) as c_int) << 8) + (*sp.add(3) as c_int)) as png_uint_16;

                        if a == 0 {
                            *sp =
                                ((((*png_ptr).background.gray as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(1) = (((*png_ptr).background.gray as c_int) & 0xff) as png_byte;
                        } else if a < 0xffff {
                            let g: png_uint_16;
                            let v: png_uint_16;

                            g = (((*sp as c_int) << 8) + (*sp.add(1) as c_int)) as png_uint_16;
                            v = png_composite_16(
                                g as png_uint_32,
                                a as png_uint_32,
                                (*png_ptr).background.gray as png_uint_32,
                            );
                            *sp = (((v as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(1) = ((v as c_int) & 0xff) as png_byte;
                        }

                        i += 1;
                        sp = sp.add(4);
                    }
                }
            }
        }

        PNG_COLOR_TYPE_RGB_ALPHA => {
            if (*row_info).bit_depth as c_int == 8 {
                if gamma_to_1 != core::ptr::null()
                    && gamma_from_1 != core::ptr::null()
                    && gamma_table != core::ptr::null()
                {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_byte = *sp.add(3);

                        if a == 0xff {
                            *sp = *gamma_table.add(*sp as usize);
                            *sp.add(1) = *gamma_table.add(*sp.add(1) as usize);
                            *sp.add(2) = *gamma_table.add(*sp.add(2) as usize);
                        } else if a == 0 {
                            /* Background is already in screen gamma */
                            *sp = (*png_ptr).background.red as png_byte;
                            *sp.add(1) = (*png_ptr).background.green as png_byte;
                            *sp.add(2) = (*png_ptr).background.blue as png_byte;
                        } else {
                            let mut v: png_byte;
                            let mut w: png_byte;

                            v = *gamma_to_1.add(*sp as usize);
                            w = png_composite(
                                v as png_uint_16,
                                a as png_uint_16,
                                (*png_ptr).background_1.red as png_uint_16,
                            );
                            if optimize == 0 {
                                w = *gamma_from_1.add(w as usize);
                            }
                            *sp = w;

                            v = *gamma_to_1.add(*sp.add(1) as usize);
                            w = png_composite(
                                v as png_uint_16,
                                a as png_uint_16,
                                (*png_ptr).background_1.green as png_uint_16,
                            );
                            if optimize == 0 {
                                w = *gamma_from_1.add(w as usize);
                            }
                            *sp.add(1) = w;

                            v = *gamma_to_1.add(*sp.add(2) as usize);
                            w = png_composite(
                                v as png_uint_16,
                                a as png_uint_16,
                                (*png_ptr).background_1.blue as png_uint_16,
                            );
                            if optimize == 0 {
                                w = *gamma_from_1.add(w as usize);
                            }
                            *sp.add(2) = w;
                        }

                        i += 1;
                        sp = sp.add(4);
                    }
                } else {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_byte = *sp.add(3);

                        if a == 0 {
                            *sp = (*png_ptr).background.red as png_byte;
                            *sp.add(1) = (*png_ptr).background.green as png_byte;
                            *sp.add(2) = (*png_ptr).background.blue as png_byte;
                        } else if a < 0xff {
                            *sp = png_composite(
                                *sp as png_uint_16,
                                a as png_uint_16,
                                (*png_ptr).background.red as png_uint_16,
                            );

                            *sp.add(1) = png_composite(
                                *sp.add(1) as png_uint_16,
                                a as png_uint_16,
                                (*png_ptr).background.green as png_uint_16,
                            );

                            *sp.add(2) = png_composite(
                                *sp.add(2) as png_uint_16,
                                a as png_uint_16,
                                (*png_ptr).background.blue as png_uint_16,
                            );
                        }

                        i += 1;
                        sp = sp.add(4);
                    }
                }
            } else
            /* if (row_info->bit_depth == 16) */
            {
                if gamma_16 != core::ptr::null_mut()
                    && gamma_16_from_1 != core::ptr::null_mut()
                    && gamma_16_to_1 != core::ptr::null_mut()
                {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_uint_16 = ((((*sp.add(6) as png_uint_16) as c_int) << 8)
                            + ((*sp.add(7) as png_uint_16) as c_int))
                            as png_uint_16;

                        if a == 0xffff as png_uint_16 {
                            let mut v: png_uint_16;

                            v = *(*gamma_16.add(((*sp.add(1) as c_int) >> gamma_shift) as usize))
                                .add(*sp as usize);
                            *sp = (((v as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(1) = ((v as c_int) & 0xff) as png_byte;

                            v = *(*gamma_16.add(((*sp.add(3) as c_int) >> gamma_shift) as usize))
                                .add(*sp.add(2) as usize);
                            *sp.add(2) = (((v as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(3) = ((v as c_int) & 0xff) as png_byte;

                            v = *(*gamma_16.add(((*sp.add(5) as c_int) >> gamma_shift) as usize))
                                .add(*sp.add(4) as usize);
                            *sp.add(4) = (((v as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(5) = ((v as c_int) & 0xff) as png_byte;
                        } else if a == 0 {
                            /* Background is already in screen gamma */
                            *sp =
                                ((((*png_ptr).background.red as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(1) = (((*png_ptr).background.red as c_int) & 0xff) as png_byte;
                            *sp.add(2) =
                                ((((*png_ptr).background.green as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(3) =
                                (((*png_ptr).background.green as c_int) & 0xff) as png_byte;
                            *sp.add(4) =
                                ((((*png_ptr).background.blue as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(5) = (((*png_ptr).background.blue as c_int) & 0xff) as png_byte;
                        } else {
                            let mut v: png_uint_16;
                            let mut w: png_uint_16;

                            v = *(*gamma_16_to_1
                                .add(((*sp.add(1) as c_int) >> gamma_shift) as usize))
                            .add(*sp as usize);
                            w = png_composite_16(
                                v as png_uint_32,
                                a as png_uint_32,
                                (*png_ptr).background_1.red as png_uint_32,
                            );
                            if optimize == 0 {
                                w = *(*gamma_16_from_1
                                    .add(((((w as c_int) & 0xff) >> gamma_shift) as usize)))
                                .add(((w as c_int) >> 8) as usize);
                            }
                            *sp = (((w as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(1) = ((w as c_int) & 0xff) as png_byte;

                            v = *(*gamma_16_to_1
                                .add(((*sp.add(3) as c_int) >> gamma_shift) as usize))
                            .add(*sp.add(2) as usize);
                            w = png_composite_16(
                                v as png_uint_32,
                                a as png_uint_32,
                                (*png_ptr).background_1.green as png_uint_32,
                            );
                            if optimize == 0 {
                                w = *(*gamma_16_from_1
                                    .add(((((w as c_int) & 0xff) >> gamma_shift) as usize)))
                                .add(((w as c_int) >> 8) as usize);
                            }

                            *sp.add(2) = (((w as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(3) = ((w as c_int) & 0xff) as png_byte;

                            v = *(*gamma_16_to_1
                                .add(((*sp.add(5) as c_int) >> gamma_shift) as usize))
                            .add(*sp.add(4) as usize);
                            w = png_composite_16(
                                v as png_uint_32,
                                a as png_uint_32,
                                (*png_ptr).background_1.blue as png_uint_32,
                            );
                            if optimize == 0 {
                                w = *(*gamma_16_from_1
                                    .add(((((w as c_int) & 0xff) >> gamma_shift) as usize)))
                                .add(((w as c_int) >> 8) as usize);
                            }

                            *sp.add(4) = (((w as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(5) = ((w as c_int) & 0xff) as png_byte;
                        }

                        i += 1;
                        sp = sp.add(8);
                    }
                } else {
                    sp = row;
                    i = 0;
                    while i < row_width {
                        let a: png_uint_16 = ((((*sp.add(6) as png_uint_16) as c_int) << 8)
                            + ((*sp.add(7) as png_uint_16) as c_int))
                            as png_uint_16;

                        if a == 0 {
                            *sp =
                                ((((*png_ptr).background.red as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(1) = (((*png_ptr).background.red as c_int) & 0xff) as png_byte;
                            *sp.add(2) =
                                ((((*png_ptr).background.green as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(3) =
                                (((*png_ptr).background.green as c_int) & 0xff) as png_byte;
                            *sp.add(4) =
                                ((((*png_ptr).background.blue as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(5) = (((*png_ptr).background.blue as c_int) & 0xff) as png_byte;
                        } else if a < 0xffff {
                            let mut v: png_uint_16;

                            let r: png_uint_16 =
                                (((*sp as c_int) << 8) + (*sp.add(1) as c_int)) as png_uint_16;
                            let g: png_uint_16 = (((*sp.add(2) as c_int) << 8)
                                + (*sp.add(3) as c_int))
                                as png_uint_16;
                            let b: png_uint_16 = (((*sp.add(4) as c_int) << 8)
                                + (*sp.add(5) as c_int))
                                as png_uint_16;

                            v = png_composite_16(
                                r as png_uint_32,
                                a as png_uint_32,
                                (*png_ptr).background.red as png_uint_32,
                            );
                            *sp = (((v as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(1) = ((v as c_int) & 0xff) as png_byte;

                            v = png_composite_16(
                                g as png_uint_32,
                                a as png_uint_32,
                                (*png_ptr).background.green as png_uint_32,
                            );
                            *sp.add(2) = (((v as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(3) = ((v as c_int) & 0xff) as png_byte;

                            v = png_composite_16(
                                b as png_uint_32,
                                a as png_uint_32,
                                (*png_ptr).background.blue as png_uint_32,
                            );
                            *sp.add(4) = (((v as c_int) >> 8) & 0xff) as png_byte;
                            *sp.add(5) = ((v as c_int) & 0xff) as png_byte;
                        }

                        i += 1;
                        sp = sp.add(8);
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
pub unsafe fn png_do_gamma(row_info: png_row_infop, row: png_bytep, png_ptr: png_structrp) {
    let gamma_table: png_const_bytep = (*png_ptr).gamma_table;
    let gamma_16_table: png_uint_16pp = (*png_ptr).gamma_16_table;
    let gamma_shift: c_int = (*png_ptr).gamma_shift;

    let mut sp: png_bytep;
    let mut i: png_uint_32;
    let row_width: png_uint_32 = (*row_info).width;

    if ((*row_info).bit_depth as c_int <= 8 && gamma_table != core::ptr::null())
        || ((*row_info).bit_depth as c_int == 16 && gamma_16_table != core::ptr::null_mut())
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

                        *sp = ((((*gamma_table
                            .add((a | (a >> 2) | (a >> 4) | (a >> 6)) as usize)
                            as c_int))
                            & 0xc0)
                            | (((*gamma_table
                                .add(((b << 2) | b | (b >> 2) | (b >> 4)) as usize)
                                as c_int)
                                >> 2)
                                & 0x30)
                            | (((*gamma_table
                                .add(((c << 4) | (c << 2) | c | (c >> 2)) as usize)
                                as c_int)
                                >> 4)
                                & 0x0c)
                            | ((*gamma_table
                                .add(((d << 6) | (d << 4) | (d << 2) | d) as usize)
                                as c_int)
                                >> 6)) as png_byte;
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

                        *sp = (((*gamma_table.add((msb | (msb >> 4)) as usize) as c_int) & 0xf0)
                            | ((*gamma_table.add(((lsb << 4) | lsb) as usize) as c_int) >> 4))
                            as png_byte;
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
pub unsafe fn png_do_encode_alpha(row_info: png_row_infop, row: png_bytep, png_ptr: png_structrp) {
    let mut row: png_bytep = row;
    let mut row_width: png_uint_32 = (*row_info).width;

    if ((*row_info).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0 {
        if (*row_info).bit_depth as c_int == 8 {
            let table: png_bytep = (*png_ptr).gamma_from_1;

            if table != core::ptr::null_mut() {
                let step: c_int = if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
                    4
                } else {
                    2
                };

                /* The alpha channel is the last component: */
                row = row.add((step - 1) as usize);

                while row_width > 0 {
                    *row = *table.add(*row as usize);

                    row_width -= 1;
                    row = row.add(step as usize);
                }

                return;
            }
        } else if (*row_info).bit_depth as c_int == 16 {
            let table: png_uint_16pp = (*png_ptr).gamma_16_from_1;
            let gamma_shift: c_int = (*png_ptr).gamma_shift;

            if table != core::ptr::null_mut() {
                let step: c_int = if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
                    8
                } else {
                    4
                };

                /* The alpha channel is the last component: */
                row = row.add((step - 2) as usize);

                while row_width > 0 {
                    let v: png_uint_16;

                    v = *(*table.add(((*row.add(1) as c_int) >> gamma_shift) as usize))
                        .add(*row as usize);
                    *row = (((v as c_int) >> 8) & 0xff) as png_byte;
                    *row.add(1) = ((v as c_int) & 0xff) as png_byte;

                    row_width -= 1;
                    row = row.add(step as usize);
                }

                return;
            }
        }
    }

    /* Only get to here if called with a weird row_info; no harm has been done,
     * so just issue a warning.
     */
    png_warning(png_ptr, c"png_do_encode_alpha: unexpected call".as_ptr());
}
