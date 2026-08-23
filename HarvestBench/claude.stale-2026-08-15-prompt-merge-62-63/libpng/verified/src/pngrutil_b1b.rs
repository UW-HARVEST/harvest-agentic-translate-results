// pngrutil.c part B1b (lines 3226-3953)

// ---- PNG_PASS_* macros from png.h ----
#[inline]
fn png_pass_start_col(pass: c_int) -> c_int {
    ((1 & pass) << (3 - ((pass + 1) >> 1))) & 7
}
#[inline]
fn png_pass_col_offset(pass: c_int) -> c_int {
    1 << ((7 - pass) >> 1)
}

// ---- Compile-time interlace pixel-combining masks (PNG_USE_COMPILE_TIME_MASKS = 1) ----
//
// PNG_LSR(x,s) = (x)>>((s)&0x1f)  ; PNG_LSL(x,s) = (x)<<((s)&0x1f)
#[inline]
const fn png_lsr(x: png_uint_32, s: png_uint_32) -> png_uint_32 {
    x >> (s & 0x1f)
}
#[inline]
const fn png_lsl(x: png_uint_32, s: png_uint_32) -> png_uint_32 {
    x << (s & 0x1f)
}

// S_COPY(p,x): sparkle copy predicate.
#[inline]
const fn s_copy(p: png_uint_32, x: png_uint_32) -> png_uint_32 {
    (if p < 4 {
        png_lsr(0x80088822, (3 - p) * 8 + (7 - x))
    } else {
        png_lsr(0xaa55ff00, (7 - p) * 8 + (7 - x))
    }) & 1
}

// B_COPY(p,x): block copy predicate.
#[inline]
const fn b_copy(p: png_uint_32, x: png_uint_32) -> png_uint_32 {
    (if p < 4 {
        png_lsr(0xff0fff33, (3 - p) * 8 + (7 - x))
    } else {
        png_lsr(0xff55ff00, (7 - p) * 8 + (7 - x))
    }) & 1
}

// PIXEL_MASK(p,x,d,s) = PNG_LSL(((PNG_LSL(1U,d))-1),(((x)*(d))^((s)?8-(d):0)))
#[inline]
const fn pixel_mask(x: png_uint_32, d: png_uint_32, s: png_uint_32) -> png_uint_32 {
    png_lsl(
        png_lsl(1, d).wrapping_sub(1),
        (x * d) ^ (if s != 0 { 8 - d } else { 0 }),
    )
}

#[inline]
const fn s_maskx(p: png_uint_32, x: png_uint_32, d: png_uint_32, s: png_uint_32) -> png_uint_32 {
    if s_copy(p, x) != 0 {
        pixel_mask(x, d, s)
    } else {
        0
    }
}

#[inline]
const fn b_maskx(p: png_uint_32, x: png_uint_32, d: png_uint_32, s: png_uint_32) -> png_uint_32 {
    if b_copy(p, x) != 0 {
        pixel_mask(x, d, s)
    } else {
        0
    }
}

// MASK_EXPAND(m,d) = (m)*((d)==1?0x01010101:((d)==2?0x00010001:1))
#[inline]
const fn mask_expand(m: png_uint_32, d: png_uint_32) -> png_uint_32 {
    m.wrapping_mul(if d == 1 {
        0x01010101
    } else if d == 2 {
        0x00010001
    } else {
        1
    })
}

#[inline]
const fn s_mask(p: png_uint_32, d: png_uint_32, s: png_uint_32) -> png_uint_32 {
    mask_expand(
        s_maskx(p, 0, d, s)
            .wrapping_add(s_maskx(p, 1, d, s))
            .wrapping_add(s_maskx(p, 2, d, s))
            .wrapping_add(s_maskx(p, 3, d, s))
            .wrapping_add(s_maskx(p, 4, d, s))
            .wrapping_add(s_maskx(p, 5, d, s))
            .wrapping_add(s_maskx(p, 6, d, s))
            .wrapping_add(s_maskx(p, 7, d, s)),
        d,
    )
}

#[inline]
const fn b_mask(p: png_uint_32, d: png_uint_32, s: png_uint_32) -> png_uint_32 {
    mask_expand(
        b_maskx(p, 0, d, s)
            .wrapping_add(b_maskx(p, 1, d, s))
            .wrapping_add(b_maskx(p, 2, d, s))
            .wrapping_add(b_maskx(p, 3, d, s))
            .wrapping_add(b_maskx(p, 4, d, s))
            .wrapping_add(b_maskx(p, 5, d, s))
            .wrapping_add(b_maskx(p, 6, d, s))
            .wrapping_add(b_maskx(p, 7, d, s)),
        d,
    )
}

// S_MASKS(d,s) = { S_MASK(0..5,d,s) }
#[inline]
const fn s_masks(d: png_uint_32, s: png_uint_32) -> [png_uint_32; 6] {
    [
        s_mask(0, d, s),
        s_mask(1, d, s),
        s_mask(2, d, s),
        s_mask(3, d, s),
        s_mask(4, d, s),
        s_mask(5, d, s),
    ]
}

// B_MASKS(d,s) = { B_MASK(1,d,s), B_MASK(3,d,s), B_MASK(5,d,s) }
#[inline]
const fn b_masks(d: png_uint_32, s: png_uint_32) -> [png_uint_32; 3] {
    [b_mask(1, d, s), b_mask(3, d, s), b_mask(5, d, s)]
}

// row_mask[2/*PACKSWAP*/][3/*depth*/][6]
static ROW_MASK: [[[png_uint_32; 6]; 3]; 2] = [
    // Little-endian byte masks for PACKSWAP
    [s_masks(1, 0), s_masks(2, 0), s_masks(4, 0)],
    // Normal (big-endian byte) masks - PNG format
    [s_masks(1, 1), s_masks(2, 1), s_masks(4, 1)],
];

// display_mask[2][3][3]
static DISPLAY_MASK: [[[png_uint_32; 3]; 3]; 2] = [
    // Little-endian byte masks for PACKSWAP
    [b_masks(1, 0), b_masks(2, 0), b_masks(4, 0)],
    // Normal (big-endian byte) masks - PNG format
    [b_masks(1, 1), b_masks(2, 1), b_masks(4, 1)],
];

// DEPTH_INDEX(d) = (d)==1?0:((d)==2?1:2)
#[inline]
fn depth_index(d: c_uint) -> usize {
    if d == 1 {
        0
    } else if d == 2 {
        1
    } else {
        2
    }
}

// MASK(pass,depth,display,png)
#[inline]
fn mask_val(pass: c_uint, depth: c_uint, display: c_int, png: usize) -> png_uint_32 {
    if display != 0 {
        DISPLAY_MASK[png][depth_index(depth)][(pass >> 1) as usize]
    } else {
        ROW_MASK[png][depth_index(depth)][pass as usize]
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_combine_row(png_ptr: png_const_structrp, dp: png_bytep, display: c_int) {
    let mut pixel_depth: c_uint = (*png_ptr).transformed_pixel_depth as c_uint;
    let mut sp: png_const_bytep = (*png_ptr).row_buf.add(1);
    let mut row_width: png_alloc_size_t = (*png_ptr).width as png_alloc_size_t;
    let pass: c_uint = (*png_ptr).pass as c_uint;
    let mut dp = dp;
    let mut end_ptr: png_bytep = ptr::null_mut();
    let mut end_byte: png_byte = 0;
    let mut end_mask: c_uint;

    /* Added in 1.5.6: it should not be possible to enter this routine until at
     * least one row has been read from the PNG data and transformed.
     */
    if pixel_depth == 0 {
        png_error(png_ptr, c"internal row logic error".as_ptr());
    }

    /* Added in 1.5.4: the pixel depth should match the information returned by
     * any call to png_read_update_info at this point.  Do not continue if we got
     * this wrong.
     */
    if (*png_ptr).info_rowbytes != 0
        && (*png_ptr).info_rowbytes != png_rowbytes(pixel_depth, row_width as png_uint_32)
    {
        png_error(png_ptr, c"internal row size calculation error".as_ptr());
    }

    /* Don't expect this to ever happen: */
    if row_width == 0 {
        png_error(png_ptr, c"internal row width error".as_ptr());
    }

    /* Preserve the last byte in cases where only part of it will be overwritten,
     * the multiply below may overflow, we don't care because ANSI-C guarantees
     * we get the low bits.
     */
    end_mask = ((pixel_depth as png_alloc_size_t).wrapping_mul(row_width) & 7) as c_uint;
    if end_mask != 0 {
        /* end_ptr == NULL is a flag to say do nothing */
        end_ptr = dp.add(png_rowbytes(pixel_depth, row_width as png_uint_32) - 1);
        end_byte = *end_ptr;
        if ((*png_ptr).transformations & PNG_PACKSWAP) != 0 {
            /* little-endian byte */
            end_mask = (0xffu32 << end_mask) as c_uint;
        } else {
            /* big-endian byte */
            end_mask = 0xff >> end_mask;
        }
        /* end_mask is now the bits to *keep* from the destination row */
    }

    /* For non-interlaced images this reduces to a memcpy(). */
    if (*png_ptr).interlaced != 0
        && ((*png_ptr).transformations & PNG_INTERLACE) != 0
        && pass < 6
        && (display == 0 ||
        /* The following copies everything for 'display' on passes 0, 2 and 4. */
        (display == 1 && (pass & 1) != 0))
    {
        /* Narrow images may have no bits in a pass; the caller should handle
         * this, but this test is cheap:
         */
        if row_width <= png_pass_start_col(pass as c_int) as png_alloc_size_t {
            return;
        }

        if pixel_depth < 8 {
            /* Use the appropriate mask to copy the required bits. */
            let pixels_per_byte: png_uint_32 = 8 / pixel_depth;
            let mut mask: png_uint_32;

            if ((*png_ptr).transformations & PNG_PACKSWAP) != 0 {
                mask = mask_val(pass, pixel_depth, display, 0);
            } else {
                mask = mask_val(pass, pixel_depth, display, 1);
            }

            loop {
                /* It doesn't matter in the following if png_uint_32 has more than
                 * 32 bits because the high bits always match those in m<<24; it is,
                 * however, essential to use OR here, not +, because of this.
                 */
                let m0: png_uint_32 = mask;
                mask = (m0 >> 8) | (m0 << 24); /* rotate right to good compilers */
                let m: png_uint_32 = m0 & 0xff;

                if m != 0 {
                    /* something to copy */
                    if m != 0xff {
                        *dp = (((*dp as png_uint_32) & !m) | ((*sp as png_uint_32) & m)) as png_byte;
                    } else {
                        *dp = *sp;
                    }
                }

                /* NOTE: this may overwrite the last byte with garbage if the image
                 * is not an exact number of bytes wide; libpng has always done
                 * this.
                 */
                if row_width <= pixels_per_byte as png_alloc_size_t {
                    break; /* May need to restore part of the last byte */
                }

                row_width -= pixels_per_byte as png_alloc_size_t;
                dp = dp.add(1);
                sp = sp.add(1);
            }
        } else {
            /* pixel_depth >= 8 */
            let mut bytes_to_copy: c_uint;
            let bytes_to_jump: c_uint;

            /* Validate the depth - it must be a multiple of 8 */
            if pixel_depth & 7 != 0 {
                png_error(png_ptr, c"invalid user transform pixel depth".as_ptr());
            }

            pixel_depth >>= 3; /* now in bytes */
            row_width = row_width.wrapping_mul(pixel_depth as png_alloc_size_t);

            /* Regardless of pass number the Adam 7 interlace always results in a
             * fixed number of pixels to copy then to skip.
             */
            {
                let offset: c_uint =
                    (png_pass_start_col(pass as c_int) as c_uint).wrapping_mul(pixel_depth);

                row_width = row_width.wrapping_sub(offset as png_alloc_size_t);
                dp = dp.add(offset as usize);
                sp = sp.add(offset as usize);
            }

            /* Work out the bytes to copy. */
            if display != 0 {
                /* When doing the 'block' algorithm the pixel in the pass gets
                 * replicated to adjacent pixels.
                 */
                bytes_to_copy = (1u32 << ((6 - pass) >> 1)).wrapping_mul(pixel_depth);

                /* But don't allow this number to exceed the actual row width. */
                if bytes_to_copy as png_alloc_size_t > row_width {
                    bytes_to_copy = row_width as c_uint;
                }
            } else {
                /* normal row; Adam7 only ever gives us one pixel to copy. */
                bytes_to_copy = pixel_depth;
            }

            /* In Adam7 there is a constant offset between where the pixels go. */
            bytes_to_jump =
                (png_pass_col_offset(pass as c_int) as c_uint).wrapping_mul(pixel_depth);

            let btj: png_alloc_size_t = bytes_to_jump as png_alloc_size_t;

            match bytes_to_copy {
                1 => loop {
                    *dp = *sp;

                    if row_width <= btj {
                        return;
                    }

                    dp = dp.add(btj);
                    sp = sp.add(btj);
                    row_width -= btj;
                },

                2 => {
                    /* There is a possibility of a partial copy at the end here. */
                    loop {
                        *dp.add(0) = *sp.add(0);
                        *dp.add(1) = *sp.add(1);

                        if row_width <= btj {
                            return;
                        }

                        sp = sp.add(btj);
                        dp = dp.add(btj);
                        row_width -= btj;

                        if !(row_width > 1) {
                            break;
                        }
                    }

                    /* And there can only be one byte left at this point: */
                    *dp = *sp;
                    return;
                }

                3 => {
                    /* This can only be the RGB case. */
                    loop {
                        *dp.add(0) = *sp.add(0);
                        *dp.add(1) = *sp.add(1);
                        *dp.add(2) = *sp.add(2);

                        if row_width <= btj {
                            return;
                        }

                        sp = sp.add(btj);
                        dp = dp.add(btj);
                        row_width -= btj;
                    }
                }

                _ => {
                    /* PNG_ALIGN_TYPE == PNG_ALIGN_SIZE (default): try aligned copies. */
                    if bytes_to_copy < 16
                        && ((dp as usize) & 1) == 0
                        && ((sp as usize) & 1) == 0
                        && bytes_to_copy % 2 == 0
                        && bytes_to_jump % 2 == 0
                    {
                        /* Everything is aligned for png_uint_16 copies, but try for
                         * png_uint_32 first.
                         */
                        if ((dp as usize) & 3) == 0
                            && ((sp as usize) & 3) == 0
                            && bytes_to_copy % 4 == 0
                            && bytes_to_jump % 4 == 0
                        {
                            let mut dp32: png_uint_32p = dp as png_uint_32p;
                            let mut sp32: png_const_uint_32p = sp as png_const_uint_32p;
                            let skip: size_t =
                                (bytes_to_jump.wrapping_sub(bytes_to_copy) as size_t) / 4;

                            loop {
                                let mut c: size_t = bytes_to_copy as size_t;
                                loop {
                                    *dp32 = *sp32;
                                    dp32 = dp32.add(1);
                                    sp32 = sp32.add(1);
                                    c -= 4;
                                    if !(c > 0) {
                                        break;
                                    }
                                }

                                if row_width <= btj {
                                    return;
                                }

                                dp32 = dp32.add(skip);
                                sp32 = sp32.add(skip);
                                row_width -= btj;

                                if !(bytes_to_copy as png_alloc_size_t <= row_width) {
                                    break;
                                }
                            }

                            /* row_width truncates the final copy; 1-3 bytes left. */
                            dp = dp32 as png_bytep;
                            sp = sp32 as png_const_bytep;
                            loop {
                                *dp = *sp;
                                dp = dp.add(1);
                                sp = sp.add(1);
                                row_width -= 1;
                                if !(row_width > 0) {
                                    break;
                                }
                            }
                            return;
                        } else {
                            /* Else do it in 16-bit quantities. */
                            let mut dp16: png_uint_16p = dp as png_uint_16p;
                            let mut sp16: png_const_uint_16p = sp as png_const_uint_16p;
                            let skip: size_t =
                                (bytes_to_jump.wrapping_sub(bytes_to_copy) as size_t) / 2;

                            loop {
                                let mut c: size_t = bytes_to_copy as size_t;
                                loop {
                                    *dp16 = *sp16;
                                    dp16 = dp16.add(1);
                                    sp16 = sp16.add(1);
                                    c -= 2;
                                    if !(c > 0) {
                                        break;
                                    }
                                }

                                if row_width <= btj {
                                    return;
                                }

                                dp16 = dp16.add(skip);
                                sp16 = sp16.add(skip);
                                row_width -= btj;

                                if !(bytes_to_copy as png_alloc_size_t <= row_width) {
                                    break;
                                }
                            }

                            /* End of row - 1 byte left, bytes_to_copy > row_width: */
                            dp = dp16 as png_bytep;
                            sp = sp16 as png_const_bytep;
                            loop {
                                *dp = *sp;
                                dp = dp.add(1);
                                sp = sp.add(1);
                                row_width -= 1;
                                if !(row_width > 0) {
                                    break;
                                }
                            }
                            return;
                        }
                    }

                    /* The true default - use a memcpy: */
                    loop {
                        memcpy(
                            dp as *mut c_void,
                            sp as *const c_void,
                            bytes_to_copy as size_t,
                        );

                        if row_width <= btj {
                            return;
                        }

                        sp = sp.add(btj);
                        dp = dp.add(btj);
                        row_width -= btj;
                        if bytes_to_copy as png_alloc_size_t > row_width {
                            bytes_to_copy = row_width as c_uint;
                        }
                    }
                }
            }

            /* NOT REACHED */
        } /* pixel_depth >= 8 */

        /* Here if pixel_depth < 8 to check 'end_ptr' below. */
    } else {
        /* If here then the switch above wasn't used so just memcpy the whole row
         * from the temporary row buffer.
         */
        memcpy(
            dp as *mut c_void,
            sp as *const c_void,
            png_rowbytes(pixel_depth, row_width as png_uint_32),
        );
    }

    /* Restore the overwritten bits from the last byte if necessary. */
    if !end_ptr.is_null() {
        *end_ptr =
            (((end_byte as png_uint_32) & end_mask) | ((*end_ptr as png_uint_32) & !end_mask))
                as png_byte;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_read_interlace(
    row_info: png_row_infop,
    row: png_bytep,
    pass: c_int,
    transformations: png_uint_32, /* Because these may affect the byte layout */
) {
    if !row.is_null() && !row_info.is_null() {
        let final_width: png_uint_32 =
            (*row_info).width.wrapping_mul(PNG_PASS_INC[pass as usize] as png_uint_32);

        match (*row_info).pixel_depth {
            1 => {
                let mut sp: png_bytep = row.add(((*row_info).width.wrapping_sub(1) >> 3) as size_t);
                let mut dp: png_bytep = row.add((final_width.wrapping_sub(1) >> 3) as size_t);
                let mut sshift: c_uint;
                let mut dshift: c_uint;
                let s_start: c_uint;
                let s_end: c_uint;
                let s_inc: c_int;
                let jstop: c_int = PNG_PASS_INC[pass as usize] as c_int;
                let mut v: png_byte;

                if (transformations & PNG_PACKSWAP) != 0 {
                    sshift = (*row_info).width.wrapping_add(7) & 0x07;
                    dshift = final_width.wrapping_add(7) & 0x07;
                    s_start = 7;
                    s_end = 0;
                    s_inc = -1;
                } else {
                    sshift = 7 - ((*row_info).width.wrapping_add(7) & 0x07);
                    dshift = 7 - (final_width.wrapping_add(7) & 0x07);
                    s_start = 0;
                    s_end = 7;
                    s_inc = 1;
                }

                let mut i: png_uint_32 = 0;
                while i < (*row_info).width {
                    v = (((*sp as c_int) >> sshift) & 0x01) as png_byte;
                    let mut j: c_int = 0;
                    while j < jstop {
                        let mut tmp: c_uint = (*dp as c_uint) & (0x7f7f >> (7 - dshift));
                        tmp |= (v as c_uint) << dshift;
                        *dp = (tmp & 0xff) as png_byte;

                        if dshift == s_end {
                            dshift = s_start;
                            dp = dp.sub(1);
                        } else {
                            dshift = ((dshift as c_int) + s_inc) as c_uint;
                        }
                        j += 1;
                    }

                    if sshift == s_end {
                        sshift = s_start;
                        sp = sp.sub(1);
                    } else {
                        sshift = ((sshift as c_int) + s_inc) as c_uint;
                    }
                    i += 1;
                }
            }

            2 => {
                let mut sp: png_bytep = row.add(((*row_info).width.wrapping_sub(1) >> 2) as size_t);
                let mut dp: png_bytep = row.add((final_width.wrapping_sub(1) >> 2) as size_t);
                let mut sshift: c_uint;
                let mut dshift: c_uint;
                let s_start: c_uint;
                let s_end: c_uint;
                let s_inc: c_int;
                let jstop: c_int = PNG_PASS_INC[pass as usize] as c_int;

                if (transformations & PNG_PACKSWAP) != 0 {
                    sshift = ((*row_info).width.wrapping_add(3) & 0x03) << 1;
                    dshift = (final_width.wrapping_add(3) & 0x03) << 1;
                    s_start = 6;
                    s_end = 0;
                    s_inc = -2;
                } else {
                    sshift = (3 - ((*row_info).width.wrapping_add(3) & 0x03)) << 1;
                    dshift = (3 - (final_width.wrapping_add(3) & 0x03)) << 1;
                    s_start = 0;
                    s_end = 6;
                    s_inc = 2;
                }

                let mut i: png_uint_32 = 0;
                while i < (*row_info).width {
                    let v: png_byte = (((*sp as c_int) >> sshift) & 0x03) as png_byte;
                    let mut j: c_int = 0;
                    while j < jstop {
                        let mut tmp: c_uint = (*dp as c_uint) & (0x3f3f >> (6 - dshift));
                        tmp |= (v as c_uint) << dshift;
                        *dp = (tmp & 0xff) as png_byte;

                        if dshift == s_end {
                            dshift = s_start;
                            dp = dp.sub(1);
                        } else {
                            dshift = ((dshift as c_int) + s_inc) as c_uint;
                        }
                        j += 1;
                    }

                    if sshift == s_end {
                        sshift = s_start;
                        sp = sp.sub(1);
                    } else {
                        sshift = ((sshift as c_int) + s_inc) as c_uint;
                    }
                    i += 1;
                }
            }

            4 => {
                let mut sp: png_bytep = row.add(((*row_info).width.wrapping_sub(1) >> 1) as size_t);
                let mut dp: png_bytep = row.add((final_width.wrapping_sub(1) >> 1) as size_t);
                let mut sshift: c_uint;
                let mut dshift: c_uint;
                let s_start: c_uint;
                let s_end: c_uint;
                let s_inc: c_int;
                let jstop: c_int = PNG_PASS_INC[pass as usize] as c_int;

                if (transformations & PNG_PACKSWAP) != 0 {
                    sshift = ((*row_info).width.wrapping_add(1) & 0x01) << 2;
                    dshift = (final_width.wrapping_add(1) & 0x01) << 2;
                    s_start = 4;
                    s_end = 0;
                    s_inc = -4;
                } else {
                    sshift = (1 - ((*row_info).width.wrapping_add(1) & 0x01)) << 2;
                    dshift = (1 - (final_width.wrapping_add(1) & 0x01)) << 2;
                    s_start = 0;
                    s_end = 4;
                    s_inc = 4;
                }

                let mut i: png_uint_32 = 0;
                while i < (*row_info).width {
                    let v: png_byte = (((*sp as c_int) >> sshift) & 0x0f) as png_byte;
                    let mut j: c_int = 0;
                    while j < jstop {
                        let mut tmp: c_uint = (*dp as c_uint) & (0xf0f >> (4 - dshift));
                        tmp |= (v as c_uint) << dshift;
                        *dp = (tmp & 0xff) as png_byte;

                        if dshift == s_end {
                            dshift = s_start;
                            dp = dp.sub(1);
                        } else {
                            dshift = ((dshift as c_int) + s_inc) as c_uint;
                        }
                        j += 1;
                    }

                    if sshift == s_end {
                        sshift = s_start;
                        sp = sp.sub(1);
                    } else {
                        sshift = ((sshift as c_int) + s_inc) as c_uint;
                    }
                    i += 1;
                }
            }

            _ => {
                let pixel_bytes: size_t = ((*row_info).pixel_depth >> 3) as size_t;

                let mut sp: png_bytep =
                    row.add(((*row_info).width.wrapping_sub(1) as size_t).wrapping_mul(pixel_bytes));

                let mut dp: png_bytep =
                    row.add((final_width.wrapping_sub(1) as size_t).wrapping_mul(pixel_bytes));

                let jstop: c_int = PNG_PASS_INC[pass as usize] as c_int;

                let mut i: png_uint_32 = 0;
                while i < (*row_info).width {
                    let mut v: [png_byte; 8] = [0; 8]; /* SAFE; pixel_depth does not exceed 64 */

                    memcpy(v.as_mut_ptr() as *mut c_void, sp as *const c_void, pixel_bytes);

                    let mut j: c_int = 0;
                    while j < jstop {
                        memcpy(dp as *mut c_void, v.as_ptr() as *const c_void, pixel_bytes);
                        dp = dp.sub(pixel_bytes);
                        j += 1;
                    }

                    sp = sp.sub(pixel_bytes);
                    i += 1;
                }
            }
        }

        (*row_info).width = final_width;
        (*row_info).rowbytes = png_rowbytes((*row_info).pixel_depth as png_uint_32, final_width);
    }
}
