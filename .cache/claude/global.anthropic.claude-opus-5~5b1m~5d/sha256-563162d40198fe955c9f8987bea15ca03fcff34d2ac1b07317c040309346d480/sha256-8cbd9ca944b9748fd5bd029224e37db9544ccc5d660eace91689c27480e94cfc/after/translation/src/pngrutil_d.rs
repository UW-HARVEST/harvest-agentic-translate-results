//! pngrutil.c lines 3227-3953: png_combine_row and png_do_read_interlace.
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

/* Combines the row recently read in with the existing pixels in the row.  This
 * routine takes care of alpha and transparency if requested.  This routine also
 * handles the two methods of progressive display of interlaced images,
 * depending on the 'display' value; if 'display' is true then the whole row
 * (dp) is filled from the start by replicating the available pixels.  If
 * 'display' is false only those pixels present in the pass are filled in.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_combine_row(
    png_ptr: png_const_structrp,
    dp: png_bytep,
    display: c_int,
) {
    let mut dp: png_bytep = dp;
    let mut pixel_depth: c_uint = (*png_ptr).transformed_pixel_depth as c_uint;
    let mut sp: png_const_bytep = (*png_ptr).row_buf.add(1);
    let mut row_width: png_alloc_size_t = (*png_ptr).width as png_alloc_size_t;
    let pass: c_uint = (*png_ptr).pass as c_uint;
    let mut end_ptr: png_bytep = core::ptr::null_mut();
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
        && (*png_ptr).info_rowbytes != PNG_ROWBYTES(pixel_depth as u32, row_width as png_uint_32)
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
    end_mask = (((pixel_depth as usize).wrapping_mul(row_width)) & 7) as c_uint;
    if end_mask != 0 {
        /* end_ptr == NULL is a flag to say do nothing */
        end_ptr = dp
            .add(PNG_ROWBYTES(pixel_depth as u32, row_width as png_uint_32))
            .sub(1);
        end_byte = *end_ptr;
        if ((*png_ptr).transformations & PNG_PACKSWAP) != 0 {
            /* little-endian byte */
            end_mask = (0xffi32 << end_mask) as c_uint;
        } else
        /* big-endian byte */
        {
            end_mask = 0xffu32 >> end_mask;
        }
        /* end_mask is now the bits to *keep* from the destination row */
    }

    /* For non-interlaced images this reduces to a memcpy(). A memcpy()
     * will also happen if interlacing isn't supported or if the application
     * does not call png_set_interlace_handling().  In the latter cases the
     * caller just gets a sequence of the unexpanded rows from each interlace
     * pass.
     */
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
            /* For pixel depths up to 4 bpp the 8-pixel mask can be expanded to fit
             * into 32 bits, then a single loop over the bytes using the four byte
             * values in the 32-bit mask can be used.  For the 'display' option the
             * expanded mask may also not require any masking within a byte.  To
             * make this work the PACKSWAP option must be taken into account - it
             * simply requires the pixels to be reversed in each byte.
             *
             * The 'regular' case requires a mask for each of the first 6 passes,
             * the 'display' case does a copy for the even passes in the range
             * 0..6.  This has already been handled in the test above.
             *
             * The masks are arranged as four bytes with the first byte to use in
             * the lowest bits (little-endian) regardless of the order (PACKSWAP or
             * not) of the pixels in each byte.
             *
             * NOTE: the whole of this logic depends on the caller of this function
             * only calling it on rows appropriate to the pass.  This function only
             * understands the 'x' logic; the 'y' logic is handled by the caller.
             *
             * The following defines allow generation of compile time constant bit
             * masks for each pixel depth and each possibility of swapped or not
             * swapped bytes.  Pass 'p' is in the range 0..6; 'x', a pixel index,
             * is in the range 0..7; and the result is 1 if the pixel is to be
             * copied in the pass, 0 if not.  'S' is for the sparkle method, 'B'
             * for the block method.
             *
             * With some compilers a compile time expression of the general form:
             *
             *    (shift >= 32) ? (a >> (shift-32)) : (b >> shift)
             *
             * Produces warnings with values of 'shift' in the range 33 to 63
             * because the right hand side of the ?: expression is evaluated by
             * the compiler even though it isn't used.  Microsoft Visual C (various
             * versions) and the Intel C compiler are known to do this.  To avoid
             * this the following macros are used in 1.5.6.  This is a temporary
             * solution to avoid destabilizing the code during the release process.
             *
             * PNG_LSR(x,s) ((x)>>((s) & 0x1f))
             * PNG_LSL(x,s) ((x)<<((s) & 0x1f))
             *
             * S_COPY(p,x) (((p)<4 ? PNG_LSR(0x80088822,(3-(p))*8+(7-(x))) :
             *    PNG_LSR(0xaa55ff00,(7-(p))*8+(7-(x)))) & 1)
             * B_COPY(p,x) (((p)<4 ? PNG_LSR(0xff0fff33,(3-(p))*8+(7-(x))) :
             *    PNG_LSR(0xff55ff00,(7-(p))*8+(7-(x)))) & 1)
             *
             * Return a mask for pass 'p' pixel 'x' at depth 'd'.  The mask is
             * little endian - the first pixel is at bit 0 - however the extra
             * parameter 's' can be set to cause the mask position to be swapped
             * within each byte, to match the PNG format.  This is done by XOR of
             * the shift with 7, 6 or 4 for bit depths 1, 2 and 4.
             *
             * PIXEL_MASK(p,x,d,s)
             *     (PNG_LSL(((PNG_LSL(1U,(d)))-1),(((x)*(d))^((s)?8-(d):0))))
             *
             * Hence generate the appropriate 'block' or 'sparkle' pixel copy mask.
             *
             * S_MASKx(p,x,d,s) (S_COPY(p,x)?PIXEL_MASK(p,x,d,s):0)
             * B_MASKx(p,x,d,s) (B_COPY(p,x)?PIXEL_MASK(p,x,d,s):0)
             *
             * Combine 8 of these to get the full mask.  For the 1-bpp and 2-bpp
             * cases the result needs replicating, for the 4-bpp case the above
             * generates a full 32 bits.
             *
             * MASK_EXPAND(m,d) ((m)*((d)==1?0x01010101:((d)==2?0x00010001:1)))
             *
             * S_MASK(p,d,s) MASK_EXPAND(S_MASKx(p,0,d,s) + ... + S_MASKx(p,7,d,s), d)
             * B_MASK(p,d,s) MASK_EXPAND(B_MASKx(p,0,d,s) + ... + B_MASKx(p,7,d,s), d)
             *
             * Utility macros to construct all the masks for a depth/swap
             * combination.  The 's' parameter says whether the format is PNG
             * (big endian bytes) or not.  Only the three odd-numbered passes are
             * required for the display/block algorithm.
             *
             * S_MASKS(d,s) { S_MASK(0,d,s), S_MASK(1,d,s), S_MASK(2,d,s),
             *     S_MASK(3,d,s), S_MASK(4,d,s), S_MASK(5,d,s) }
             *
             * B_MASKS(d,s) { B_MASK(1,d,s), B_MASK(3,d,s), B_MASK(5,d,s) }
             *
             * DEPTH_INDEX(d) ((d)==1?0:((d)==2?1:2))
             */

            /* Hence the pre-compiled masks indexed by PACKSWAP (or not), depth and
             * then pass:
             */
            static row_mask: [[[png_uint_32; 6]; 3]; 2] = [
                /* Little-endian byte masks for PACKSWAP */
                [
                    /* S_MASKS(1,0) */
                    [
                        0x01010101, 0x10101010, 0x11111111, 0x44444444, 0x55555555, 0xaaaaaaaa,
                    ],
                    /* S_MASKS(2,0) */
                    [
                        0x00030003, 0x03000300, 0x03030303, 0x30303030, 0x33333333, 0xcccccccc,
                    ],
                    /* S_MASKS(4,0) */
                    [
                        0x0000000f, 0x000f0000, 0x000f000f, 0x0f000f00, 0x0f0f0f0f, 0xf0f0f0f0,
                    ],
                ],
                /* Normal (big-endian byte) masks - PNG format */
                [
                    /* S_MASKS(1,1) */
                    [
                        0x80808080, 0x08080808, 0x88888888, 0x22222222, 0xaaaaaaaa, 0x55555555,
                    ],
                    /* S_MASKS(2,1) */
                    [
                        0x00c000c0, 0xc000c000, 0xc0c0c0c0, 0x0c0c0c0c, 0xcccccccc, 0x33333333,
                    ],
                    /* S_MASKS(4,1) */
                    [
                        0x000000f0, 0x00f00000, 0x00f000f0, 0xf000f000, 0xf0f0f0f0, 0x0f0f0f0f,
                    ],
                ],
            ];

            /* display_mask has only three entries for the odd passes, so index by
             * pass>>1.
             */
            static display_mask: [[[png_uint_32; 3]; 3]; 2] = [
                /* Little-endian byte masks for PACKSWAP */
                [
                    /* B_MASKS(1,0) */
                    [0xf0f0f0f0, 0xcccccccc, 0xaaaaaaaa],
                    /* B_MASKS(2,0) */
                    [0xff00ff00, 0xf0f0f0f0, 0xcccccccc],
                    /* B_MASKS(4,0) */
                    [0xffff0000, 0xff00ff00, 0xf0f0f0f0],
                ],
                /* Normal (big-endian byte) masks - PNG format */
                [
                    /* B_MASKS(1,1) */
                    [0x0f0f0f0f, 0x33333333, 0x55555555],
                    /* B_MASKS(2,1) */
                    [0xff00ff00, 0x0f0f0f0f, 0x33333333],
                    /* B_MASKS(4,1) */
                    [0xffff0000, 0xff00ff00, 0x0f0f0f0f],
                ],
            ];

            /* MASK(pass,depth,display,png)
             *    ((display)?display_mask[png][DEPTH_INDEX(depth)][pass>>1]:
             *       row_mask[png][DEPTH_INDEX(depth)][pass])
             */
            let MASK = |pass: c_uint, depth: c_uint, display: c_int, png: usize| -> png_uint_32 {
                let depth_index: usize = if depth == 1 {
                    0
                } else if depth == 2 {
                    1
                } else {
                    2
                };

                if display != 0 {
                    display_mask[png][depth_index][(pass >> 1) as usize]
                } else {
                    row_mask[png][depth_index][pass as usize]
                }
            };

            /* Use the appropriate mask to copy the required bits.  In some cases
             * the byte mask will be 0 or 0xff; optimize these cases.  row_width is
             * the number of pixels, but the code copies bytes, so it is necessary
             * to special case the end.
             */
            let pixels_per_byte: png_uint_32 = 8 / pixel_depth as png_uint_32;
            let mut mask: png_uint_32;

            if ((*png_ptr).transformations & PNG_PACKSWAP) != 0 {
                mask = MASK(pass, pixel_depth, display, 0);
            } else {
                mask = MASK(pass, pixel_depth, display, 1);
            }

            loop {
                let mut m: png_uint_32;

                /* It doesn't matter in the following if png_uint_32 has more than
                 * 32 bits because the high bits always match those in m<<24; it is,
                 * however, essential to use OR here, not +, because of this.
                 */
                m = mask;
                mask = (m >> 8) | (m << 24); /* rotate right to good compilers */
                m &= 0xff;

                if m != 0
                /* something to copy */
                {
                    if m != 0xff {
                        *dp = ((((*dp as png_uint_32) & !m) | ((*sp as png_uint_32) & m)) & 0xff)
                            as png_byte;
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

                row_width = row_width.wrapping_sub(pixels_per_byte as png_alloc_size_t);
                dp = dp.add(1);
                sp = sp.add(1);
            }
        } else
        /* pixel_depth >= 8 */
        {
            let mut bytes_to_copy: c_uint;
            let mut bytes_to_jump: c_uint;

            /* Validate the depth - it must be a multiple of 8 */
            if (pixel_depth & 7) != 0 {
                png_error(png_ptr, c"invalid user transform pixel depth".as_ptr());
            }

            pixel_depth >>= 3; /* now in bytes */
            row_width = row_width.wrapping_mul(pixel_depth as png_alloc_size_t);

            /* Regardless of pass number the Adam 7 interlace always results in a
             * fixed number of pixels to copy then to skip.  There may be a
             * different number of pixels to skip at the start though.
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
                 * replicated to adjacent pixels.  This is why the even (0,2,4,6)
                 * passes are skipped above - the entire expanded row is copied.
                 */
                bytes_to_copy = (1u32 << ((6u32.wrapping_sub(pass)) >> 1)).wrapping_mul(pixel_depth);

                /* But don't allow this number to exceed the actual row width. */
                if bytes_to_copy as png_alloc_size_t > row_width {
                    bytes_to_copy = row_width as c_uint; /*SAFE*/
                }
            } else
            /* normal row; Adam7 only ever gives us one pixel to copy. */
            {
                bytes_to_copy = pixel_depth;
            }

            /* In Adam7 there is a constant offset between where the pixels go. */
            bytes_to_jump =
                (png_pass_col_offset(pass as c_int) as c_uint).wrapping_mul(pixel_depth);

            /* And simply copy these bytes.  Some optimization is possible here,
             * depending on the value of 'bytes_to_copy'.  Special case the low
             * byte counts, which we know to be frequent.
             *
             * Notice that these cases all 'return' rather than 'break' - this
             * avoids an unnecessary test on whether to restore the last byte
             * below.
             */
            match bytes_to_copy {
                1 => loop {
                    *dp = *sp;

                    if row_width <= bytes_to_jump as png_alloc_size_t {
                        return;
                    }

                    dp = dp.add(bytes_to_jump as usize);
                    sp = sp.add(bytes_to_jump as usize);
                    row_width = row_width.wrapping_sub(bytes_to_jump as png_alloc_size_t);
                },

                2 => {
                    /* There is a possibility of a partial copy at the end here; this
                     * slows the code down somewhat.
                     */
                    loop {
                        *dp.add(0) = *sp.add(0);
                        *dp.add(1) = *sp.add(1);

                        if row_width <= bytes_to_jump as png_alloc_size_t {
                            return;
                        }

                        sp = sp.add(bytes_to_jump as usize);
                        dp = dp.add(bytes_to_jump as usize);
                        row_width = row_width.wrapping_sub(bytes_to_jump as png_alloc_size_t);

                        if !(row_width > 1) {
                            break;
                        }
                    }

                    /* And there can only be one byte left at this point: */
                    *dp = *sp;
                    return;
                }

                3 => {
                    /* This can only be the RGB case, so each copy is exactly one
                     * pixel and it is not necessary to check for a partial copy.
                     */
                    loop {
                        *dp.add(0) = *sp.add(0);
                        *dp.add(1) = *sp.add(1);
                        *dp.add(2) = *sp.add(2);

                        if row_width <= bytes_to_jump as png_alloc_size_t {
                            return;
                        }

                        sp = sp.add(bytes_to_jump as usize);
                        dp = dp.add(bytes_to_jump as usize);
                        row_width = row_width.wrapping_sub(bytes_to_jump as png_alloc_size_t);
                    }
                }

                _ => {
                    /* Check for double byte alignment and, if possible, use a
                     * 16-bit copy.  Don't attempt this for narrow images - ones that
                     * are less than an interlace panel wide.  Don't attempt it for
                     * wide bytes_to_copy either - use the memcpy there.
                     */
                    if bytes_to_copy < 16 /*else use memcpy*/
                        && (((dp as usize as png_uint_16)
                            & ((core::mem::size_of::<png_uint_16>() as png_uint_16) - 1))
                            == 0)
                        && (((sp as usize as png_uint_16)
                            & ((core::mem::size_of::<png_uint_16>() as png_uint_16) - 1))
                            == 0)
                        && (bytes_to_copy as usize) % core::mem::size_of::<png_uint_16>() == 0
                        && (bytes_to_jump as usize) % core::mem::size_of::<png_uint_16>() == 0
                    {
                        /* Everything is aligned for png_uint_16 copies, but try for
                         * png_uint_32 first.
                         */
                        if (((dp as usize as png_uint_32)
                            & ((core::mem::size_of::<png_uint_32>() as png_uint_32) - 1))
                            == 0)
                            && (((sp as usize as png_uint_32)
                                & ((core::mem::size_of::<png_uint_32>() as png_uint_32) - 1))
                                == 0)
                            && (bytes_to_copy as usize) % core::mem::size_of::<png_uint_32>() == 0
                            && (bytes_to_jump as usize) % core::mem::size_of::<png_uint_32>() == 0
                        {
                            let mut dp32: png_uint_32p = dp as png_uint_32p;
                            let mut sp32: png_const_uint_32p = sp as png_const_uint_32p;
                            let skip: usize = ((bytes_to_jump.wrapping_sub(bytes_to_copy)) as usize)
                                / core::mem::size_of::<png_uint_32>();

                            loop {
                                let mut c: usize = bytes_to_copy as usize;
                                loop {
                                    *dp32 = *sp32;
                                    dp32 = dp32.add(1);
                                    sp32 = sp32.add(1);
                                    c = c.wrapping_sub(core::mem::size_of::<png_uint_32>());

                                    if !(c > 0) {
                                        break;
                                    }
                                }

                                if row_width <= bytes_to_jump as png_alloc_size_t {
                                    return;
                                }

                                dp32 = dp32.add(skip);
                                sp32 = sp32.add(skip);
                                row_width =
                                    row_width.wrapping_sub(bytes_to_jump as png_alloc_size_t);

                                if !(bytes_to_copy as png_alloc_size_t <= row_width) {
                                    break;
                                }
                            }

                            /* Get to here when the row_width truncates the final copy.
                             * There will be 1-3 bytes left to copy, so don't try the
                             * 16-bit loop below.
                             */
                            dp = dp32 as png_bytep;
                            sp = sp32 as png_const_bytep;
                            loop {
                                *dp = *sp;
                                dp = dp.add(1);
                                sp = sp.add(1);
                                row_width = row_width.wrapping_sub(1);
                                if !(row_width > 0) {
                                    break;
                                }
                            }
                            return;
                        }
                        /* Else do it in 16-bit quantities, but only if the size is
                         * not too large.
                         */
                        else {
                            let mut dp16: png_uint_16p = dp as png_uint_16p;
                            let mut sp16: png_const_uint_16p = sp as png_const_uint_16p;
                            let skip: usize = ((bytes_to_jump.wrapping_sub(bytes_to_copy)) as usize)
                                / core::mem::size_of::<png_uint_16>();

                            loop {
                                let mut c: usize = bytes_to_copy as usize;
                                loop {
                                    *dp16 = *sp16;
                                    dp16 = dp16.add(1);
                                    sp16 = sp16.add(1);
                                    c = c.wrapping_sub(core::mem::size_of::<png_uint_16>());

                                    if !(c > 0) {
                                        break;
                                    }
                                }

                                if row_width <= bytes_to_jump as png_alloc_size_t {
                                    return;
                                }

                                dp16 = dp16.add(skip);
                                sp16 = sp16.add(skip);
                                row_width =
                                    row_width.wrapping_sub(bytes_to_jump as png_alloc_size_t);

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
                                row_width = row_width.wrapping_sub(1);
                                if !(row_width > 0) {
                                    break;
                                }
                            }
                            return;
                        }
                    }

                    /* The true default - use a memcpy: */
                    loop {
                        memcpy(dp, sp, bytes_to_copy as usize);

                        if row_width <= bytes_to_jump as png_alloc_size_t {
                            return;
                        }

                        sp = sp.add(bytes_to_jump as usize);
                        dp = dp.add(bytes_to_jump as usize);
                        row_width = row_width.wrapping_sub(bytes_to_jump as png_alloc_size_t);
                        if bytes_to_copy as png_alloc_size_t > row_width {
                            bytes_to_copy = row_width as c_uint; /*SAFE*/
                        }
                    }
                }
            }

            /* NOT REACHED*/
        } /* pixel_depth >= 8 */

        /* Here if pixel_depth < 8 to check 'end_ptr' below. */
    } else {
        /* If here then the switch above wasn't used so just memcpy the whole row
         * from the temporary row buffer (notice that this overwrites the end of the
         * destination row if it is a partial byte.)
         */
        memcpy(
            dp,
            sp,
            PNG_ROWBYTES(pixel_depth as u32, row_width as png_uint_32),
        );
    }

    /* Restore the overwritten bits from the last byte if necessary. */
    if !end_ptr.is_null() {
        *end_ptr = ((((end_byte as c_uint) & end_mask) | ((*end_ptr as c_uint) & !end_mask)) & 0xff)
            as png_byte;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_do_read_interlace(
    row_info: png_row_infop,
    row: png_bytep,
    pass: c_int,
    transformations: png_uint_32, /* Because these may affect the byte layout */
) {
    if !row.is_null() && !row_info.is_null() {
        let final_width: png_uint_32;

        final_width = (*row_info)
            .width
            .wrapping_mul(png_pass_inc[pass as usize] as png_uint_32);

        match (*row_info).pixel_depth as c_int {
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
                    sshift = ((*row_info).width.wrapping_add(7) & 0x07) as c_uint;
                    dshift = (final_width.wrapping_add(7) & 0x07) as c_uint;
                    s_start = 7;
                    s_end = 0;
                    s_inc = -1;
                } else {
                    sshift = (7u32).wrapping_sub((*row_info).width.wrapping_add(7) & 0x07) as c_uint;
                    dshift = (7u32).wrapping_sub(final_width.wrapping_add(7) & 0x07) as c_uint;
                    s_start = 0;
                    s_end = 7;
                    s_inc = 1;
                }

                i = 0;
                while i < (*row_info).width {
                    v = (((*sp as c_int) >> sshift) & 0x01) as png_byte;
                    j = 0;
                    while j < jstop {
                        let mut tmp: c_uint =
                            ((*dp as c_int) & (0x7f7fi32 >> (7u32).wrapping_sub(dshift))) as c_uint;
                        tmp |= (((v as c_int) << dshift) as c_uint);
                        *dp = (tmp & 0xff) as png_byte;

                        if dshift == s_end {
                            dshift = s_start;
                            dp = dp.wrapping_sub(1);
                        } else {
                            dshift = ((dshift as c_int) + s_inc) as c_uint;
                        }

                        j += 1;
                    }

                    if sshift == s_end {
                        sshift = s_start;
                        sp = sp.wrapping_sub(1);
                    } else {
                        sshift = ((sshift as c_int) + s_inc) as c_uint;
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
                    sshift = (((*row_info).width.wrapping_add(3) & 0x03) << 1) as c_uint;
                    dshift = ((final_width.wrapping_add(3) & 0x03) << 1) as c_uint;
                    s_start = 6;
                    s_end = 0;
                    s_inc = -2;
                } else {
                    sshift = (((3u32).wrapping_sub((*row_info).width.wrapping_add(3) & 0x03)) << 1)
                        as c_uint;
                    dshift =
                        (((3u32).wrapping_sub(final_width.wrapping_add(3) & 0x03)) << 1) as c_uint;
                    s_start = 0;
                    s_end = 6;
                    s_inc = 2;
                }

                i = 0;
                while i < (*row_info).width {
                    let v: png_byte;
                    let mut j: c_int;

                    v = (((*sp as c_int) >> sshift) & 0x03) as png_byte;
                    j = 0;
                    while j < jstop {
                        let mut tmp: c_uint =
                            ((*dp as c_int) & (0x3f3fi32 >> (6u32).wrapping_sub(dshift))) as c_uint;
                        tmp |= (((v as c_int) << dshift) as c_uint);
                        *dp = (tmp & 0xff) as png_byte;

                        if dshift == s_end {
                            dshift = s_start;
                            dp = dp.wrapping_sub(1);
                        } else {
                            dshift = ((dshift as c_int) + s_inc) as c_uint;
                        }

                        j += 1;
                    }

                    if sshift == s_end {
                        sshift = s_start;
                        sp = sp.wrapping_sub(1);
                    } else {
                        sshift = ((sshift as c_int) + s_inc) as c_uint;
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
                    sshift = (((*row_info).width.wrapping_add(1) & 0x01) << 2) as c_uint;
                    dshift = ((final_width.wrapping_add(1) & 0x01) << 2) as c_uint;
                    s_start = 4;
                    s_end = 0;
                    s_inc = -4;
                } else {
                    sshift = (((1u32).wrapping_sub((*row_info).width.wrapping_add(1) & 0x01)) << 2)
                        as c_uint;
                    dshift =
                        (((1u32).wrapping_sub(final_width.wrapping_add(1) & 0x01)) << 2) as c_uint;
                    s_start = 0;
                    s_end = 4;
                    s_inc = 4;
                }

                i = 0;
                while i < (*row_info).width {
                    let v: png_byte = (((*sp as c_int) >> sshift) & 0x0f) as png_byte;
                    let mut j: c_int;

                    j = 0;
                    while j < jstop {
                        let mut tmp: c_uint =
                            ((*dp as c_int) & (0xf0fi32 >> (4u32).wrapping_sub(dshift))) as c_uint;
                        tmp |= (((v as c_int) << dshift) as c_uint);
                        *dp = (tmp & 0xff) as png_byte;

                        if dshift == s_end {
                            dshift = s_start;
                            dp = dp.wrapping_sub(1);
                        } else {
                            dshift = ((dshift as c_int) + s_inc) as c_uint;
                        }

                        j += 1;
                    }

                    if sshift == s_end {
                        sshift = s_start;
                        sp = sp.wrapping_sub(1);
                    } else {
                        sshift = ((sshift as c_int) + s_inc) as c_uint;
                    }

                    i = i.wrapping_add(1);
                }
            }

            _ => {
                let pixel_bytes: usize = ((*row_info).pixel_depth as c_int >> 3) as usize;

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

                    memcpy(v.as_mut_ptr(), sp, pixel_bytes);

                    j = 0;
                    while j < jstop {
                        memcpy(dp, v.as_ptr(), pixel_bytes);
                        dp = dp.wrapping_sub(pixel_bytes);

                        j += 1;
                    }

                    sp = sp.wrapping_sub(pixel_bytes);

                    i = i.wrapping_add(1);
                }
            }
        }

        (*row_info).width = final_width;
        (*row_info).rowbytes = PNG_ROWBYTES((*row_info).pixel_depth as u32, final_width);
    }
}
