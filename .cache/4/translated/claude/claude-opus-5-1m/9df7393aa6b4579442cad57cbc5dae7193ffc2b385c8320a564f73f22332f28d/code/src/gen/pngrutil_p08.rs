/* The following defines allow generation of compile time constant bit masks for
 * each pixel depth and each possibility of swapped or not swapped bytes.  Pass
 * 'p' is in the range 0..6; 'x', a pixel index, is in the range 0..7; and the
 * result is 1 if the pixel is to be copied in the pass, 0 if not.  'S' is for
 * the sparkle method, 'B' for the block method.
 *
 * (These are the C function-local `static const` tables of png_combine_row,
 * hoisted to module scope.  PNG_USE_COMPILE_TIME_MASKS is 1.)
 *
 * Hence the pre-compiled masks indexed by PACKSWAP (or not), depth and then
 * pass:
 */
static row_mask: [[[png_uint_32; 6]; 3]; 2] = [
    /* Little-endian byte masks for PACKSWAP */
    [
        [0x01010101, 0x10101010, 0x11111111, 0x44444444, 0x55555555, 0xaaaaaaaa],
        [0x00030003, 0x03000300, 0x03030303, 0x30303030, 0x33333333, 0xcccccccc],
        [0x0000000f, 0x000f0000, 0x000f000f, 0x0f000f00, 0x0f0f0f0f, 0xf0f0f0f0],
    ],
    /* Normal (big-endian byte) masks - PNG format */
    [
        [0x80808080, 0x08080808, 0x88888888, 0x22222222, 0xaaaaaaaa, 0x55555555],
        [0x00c000c0, 0xc000c000, 0xc0c0c0c0, 0x0c0c0c0c, 0xcccccccc, 0x33333333],
        [0x000000f0, 0x00f00000, 0x00f000f0, 0xf000f000, 0xf0f0f0f0, 0x0f0f0f0f],
    ],
];

/* display_mask has only three entries for the odd passes, so index by
 * pass>>1.
 */
static display_mask: [[[png_uint_32; 3]; 3]; 2] = [
    /* Little-endian byte masks for PACKSWAP */
    [
        [0xf0f0f0f0, 0xcccccccc, 0xaaaaaaaa],
        [0xff00ff00, 0xf0f0f0f0, 0xcccccccc],
        [0xffff0000, 0xff00ff00, 0xf0f0f0f0],
    ],
    /* Normal (big-endian byte) masks - PNG format */
    [
        [0x0f0f0f0f, 0x33333333, 0x55555555],
        [0xff00ff00, 0x0f0f0f0f, 0x33333333],
        [0xffff0000, 0xff00ff00, 0x0f0f0f0f],
    ],
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_combine_row(
    png_ptr: png_const_structrp,
    mut dp: png_bytep,
    display: c_int,
) {
    let mut pixel_depth: c_uint = (*png_ptr).transformed_pixel_depth as c_uint;
    let mut sp: png_const_bytep = (*png_ptr).row_buf.add(1) as png_const_bytep;
    let mut row_width: png_alloc_size_t = (*png_ptr).width as png_alloc_size_t;
    let pass: c_uint = (*png_ptr).pass as c_uint;
    let mut end_ptr: png_bytep = core::ptr::null_mut();
    let mut end_byte: png_byte = 0;
    let mut end_mask: c_uint;

    /* Added in 1.5.6: it should not be possible to enter this routine until at
     * least one row has been read from the PNG data and transformed.
     */
    if pixel_depth == 0 {
        png_error(png_ptr, b"internal row logic error\0".as_ptr() as png_const_charp);
    }

    /* Added in 1.5.4: the pixel depth should match the information returned by
     * any call to png_read_update_info at this point.  Do not continue if we got
     * this wrong.
     */
    if (*png_ptr).info_rowbytes != 0
        && (*png_ptr).info_rowbytes != PNG_ROWBYTES(pixel_depth as usize, row_width)
    {
        png_error(
            png_ptr,
            b"internal row size calculation error\0".as_ptr() as png_const_charp,
        );
    }

    /* Don't expect this to ever happen: */
    if row_width == 0 {
        png_error(png_ptr, b"internal row width error\0".as_ptr() as png_const_charp);
    }

    /* Preserve the last byte in cases where only part of it will be overwritten,
     * the multiply below may overflow, we don't care because ANSI-C guarantees
     * we get the low bits.
     */
    end_mask = (((pixel_depth as usize).wrapping_mul(row_width)) & 7) as c_uint;
    if end_mask != 0 {
        /* end_ptr == NULL is a flag to say do nothing */
        end_ptr = dp.add(PNG_ROWBYTES(pixel_depth as usize, row_width)).sub(1);
        end_byte = *end_ptr;
        if ((*png_ptr).transformations & PNG_PACKSWAP) != 0 {
            /* little-endian byte */
            end_mask = ((0xff as c_int) << end_mask) as c_uint;
        } else
        /* big-endian byte */
        {
            end_mask = ((0xff as c_int) >> end_mask) as c_uint;
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
        if row_width <= PNG_PASS_START_COL(pass as c_int) as png_alloc_size_t {
            return;
        }

        if pixel_depth < 8 {
            /* For pixel depths up to 4 bpp the 8-pixel mask can be expanded to
             * fit into 32 bits, then a single loop over the bytes using the four
             * byte values in the 32-bit mask can be used.  For the 'display'
             * option the expanded mask may also not require any masking within a
             * byte.  To make this work the PACKSWAP option must be taken into
             * account - it simply requires the pixels to be reversed in each
             * byte.
             *
             * The 'regular' case requires a mask for each of the first 6 passes,
             * the 'display' case does a copy for the even passes in the range
             * 0..6.  This has already been handled in the test above.
             *
             * NOTE: the whole of this logic depends on the caller of this
             * function only calling it on rows appropriate to the pass.  This
             * function only understands the 'x' logic; the 'y' logic is handled
             * by the caller.
             */

            /* Use the appropriate mask to copy the required bits.  In some cases
             * the byte mask will be 0 or 0xff; optimize these cases.  row_width
             * is the number of pixels, but the code copies bytes, so it is
             * necessary to special case the end.
             */
            let pixels_per_byte: png_uint_32 = 8 / pixel_depth as png_uint_32;
            let mut mask: png_uint_32;

            /* DEPTH_INDEX(d) ((d)==1?0:((d)==2?1:2)) */
            let depth_index: usize = if pixel_depth == 1 {
                0
            } else if pixel_depth == 2 {
                1
            } else {
                2
            };

            if ((*png_ptr).transformations & PNG_PACKSWAP) != 0 {
                /* MASK(pass, pixel_depth, display, 0) */
                mask = if display != 0 {
                    display_mask[0][depth_index][(pass >> 1) as usize]
                } else {
                    row_mask[0][depth_index][pass as usize]
                };
            } else {
                /* MASK(pass, pixel_depth, display, 1) */
                mask = if display != 0 {
                    display_mask[1][depth_index][(pass >> 1) as usize]
                } else {
                    row_mask[1][depth_index][pass as usize]
                };
            }

            loop {
                let mut m: png_uint_32;

                /* It doesn't matter in the following if png_uint_32 has more than
                 * 32 bits because the high bits always match those in m<<24; it
                 * is, however, essential to use OR here, not +, because of this.
                 */
                m = mask;
                mask = (m >> 8) | (m << 24); /* rotate right to good compilers */
                m &= 0xff;

                if m != 0
                /* something to copy */
                {
                    if m != 0xff {
                        *dp = (((*dp as png_uint_32) & !m) | ((*sp as png_uint_32) & m)) as png_byte;
                    } else {
                        *dp = *sp;
                    }
                }

                /* NOTE: this may overwrite the last byte with garbage if the
                 * image is not an exact number of bytes wide; libpng has always
                 * done this.
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
            let bytes_to_jump: c_uint;

            /* Validate the depth - it must be a multiple of 8 */
            if (pixel_depth & 7) != 0 {
                png_error(
                    png_ptr,
                    b"invalid user transform pixel depth\0".as_ptr() as png_const_charp,
                );
            }

            pixel_depth >>= 3; /* now in bytes */
            row_width = row_width.wrapping_mul(pixel_depth as png_alloc_size_t);

            /* Regardless of pass number the Adam 7 interlace always results in a
             * fixed number of pixels to copy then to skip.  There may be a
             * different number of pixels to skip at the start though.
             */
            {
                let offset: c_uint =
                    (PNG_PASS_START_COL(pass as c_int) as c_uint).wrapping_mul(pixel_depth);

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
                bytes_to_copy =
                    (((1 as c_int) << ((6 as c_uint).wrapping_sub(pass) >> 1)) as c_uint)
                        .wrapping_mul(pixel_depth);

                /* But don't allow this number to exceed the actual row width. */
                if bytes_to_copy as png_alloc_size_t > row_width {
                    bytes_to_copy = row_width as c_uint /*SAFE*/;
                }
            } else
            /* normal row; Adam7 only ever gives us one pixel to copy. */
            {
                bytes_to_copy = pixel_depth;
            }

            /* In Adam7 there is a constant offset between where the pixels go. */
            bytes_to_jump =
                (PNG_PASS_COL_OFFSET(pass as c_int) as c_uint).wrapping_mul(pixel_depth);

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
                    /* There is a possibility of a partial copy at the end here;
                     * this slows the code down somewhat.
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
                    /* PNG_ALIGN_TYPE != PNG_ALIGN_NONE */
                    /* Check for double byte alignment and, if possible, use a
                     * 16-bit copy.  Don't attempt this for narrow images - ones
                     * that are less than an interlace panel wide.  Don't attempt
                     * it for wide bytes_to_copy either - use the memcpy there.
                     */
                    if bytes_to_copy < 16 /*else use memcpy*/
                        && (((dp as usize as png_uint_16)
                            & ((core::mem::size_of::<png_uint_16>() - 1) as png_uint_16))
                            == 0)
                        && (((sp as usize as png_uint_16)
                            & ((core::mem::size_of::<png_uint_16>() - 1) as png_uint_16))
                            == 0)
                        && (bytes_to_copy as usize) % core::mem::size_of::<png_uint_16>() == 0
                        && (bytes_to_jump as usize) % core::mem::size_of::<png_uint_16>() == 0
                    {
                        /* Everything is aligned for png_uint_16 copies, but try
                         * for png_uint_32 first.
                         */
                        if (((dp as usize as png_uint_32)
                            & ((core::mem::size_of::<png_uint_32>() - 1) as png_uint_32))
                            == 0)
                            && (((sp as usize as png_uint_32)
                                & ((core::mem::size_of::<png_uint_32>() - 1) as png_uint_32))
                                == 0)
                            && (bytes_to_copy as usize) % core::mem::size_of::<png_uint_32>() == 0
                            && (bytes_to_jump as usize) % core::mem::size_of::<png_uint_32>() == 0
                        {
                            let mut dp32: png_uint_32p = dp as png_uint_32p;
                            let mut sp32: png_const_uint_32p = sp as png_const_uint_32p;
                            let skip: usize = (bytes_to_jump.wrapping_sub(bytes_to_copy) as usize)
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

                                if !((bytes_to_copy as png_alloc_size_t) <= row_width) {
                                    break;
                                }
                            }

                            /* Get to here when the row_width truncates the final
                             * copy.  There will be 1-3 bytes left to copy, so
                             * don't try the 16-bit loop below.
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
                        /* Else do it in 16-bit quantities, but only if the size
                         * is not too large.
                         */
                        else {
                            let mut dp16: png_uint_16p = dp as png_uint_16p;
                            let mut sp16: png_const_uint_16p = sp as png_const_uint_16p;
                            let skip: usize = (bytes_to_jump.wrapping_sub(bytes_to_copy) as usize)
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

                                if !((bytes_to_copy as png_alloc_size_t) <= row_width) {
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
                        memcpy(dp as *mut c_void, sp as *const c_void, bytes_to_copy as usize);

                        if row_width <= bytes_to_jump as png_alloc_size_t {
                            return;
                        }

                        sp = sp.add(bytes_to_jump as usize);
                        dp = dp.add(bytes_to_jump as usize);
                        row_width = row_width.wrapping_sub(bytes_to_jump as png_alloc_size_t);
                        if bytes_to_copy as png_alloc_size_t > row_width {
                            bytes_to_copy = row_width as c_uint /*SAFE*/;
                        }
                    }
                }
            }

            /* NOT REACHED*/
        } /* pixel_depth >= 8 */

        /* Here if pixel_depth < 8 to check 'end_ptr' below. */
    } else {
        /* If here then the switch above wasn't used so just memcpy the whole row
         * from the temporary row buffer (notice that this overwrites the end of
         * the destination row if it is a partial byte.)
         */
        memcpy(
            dp as *mut c_void,
            sp as *const c_void,
            PNG_ROWBYTES(pixel_depth as usize, row_width),
        );
    }

    /* Restore the overwritten bits from the last byte if necessary. */
    if end_ptr != core::ptr::null_mut() {
        *end_ptr = (((end_byte as c_uint) & end_mask) | ((*end_ptr as c_uint) & !end_mask)) as png_byte;
    }
}
