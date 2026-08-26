/* pngwrite.c lines 1407..1851 */

/* png_write_png */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_png(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    transforms: c_int,
    params: png_voidp,
) {
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    if ((*info_ptr).valid & PNG_INFO_IDAT) == 0 {
        png_app_error(
            png_ptr,
            b"no rows for png_write_image to write\0".as_ptr() as png_const_charp,
        );
        return;
    }

    /* Write the file header information. */
    png_write_info(png_ptr, info_ptr);

    /* ------ these transformations don't touch the info structure ------- */

    /* Invert monochrome pixels */
    if (transforms & PNG_TRANSFORM_INVERT_MONO) != 0 {
        png_set_invert_mono(png_ptr);
    }

    /* Shift the pixels up to a legal bit depth and fill in
     * as appropriate to correctly scale the image.
     */
    if (transforms & PNG_TRANSFORM_SHIFT) != 0 {
        if ((*info_ptr).valid & PNG_INFO_sBIT) != 0 {
            png_set_shift(png_ptr, core::ptr::addr_of!((*info_ptr).sig_bit));
        }
    }

    /* Pack pixels into bytes */
    if (transforms & PNG_TRANSFORM_PACKING) != 0 {
        png_set_packing(png_ptr);
    }

    /* Swap location of alpha bytes from ARGB to RGBA */
    if (transforms & PNG_TRANSFORM_SWAP_ALPHA) != 0 {
        png_set_swap_alpha(png_ptr);
    }

    /* Remove a filler (X) from XRGB/RGBX/AG/GA into to convert it into
     * RGB, note that the code expects the input color type to be G or RGB; no
     * alpha channel.
     */
    if (transforms & (PNG_TRANSFORM_STRIP_FILLER_AFTER | PNG_TRANSFORM_STRIP_FILLER_BEFORE)) != 0 {
        if (transforms & PNG_TRANSFORM_STRIP_FILLER_AFTER) != 0 {
            if (transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE) != 0 {
                png_app_error(
                    png_ptr,
                    b"PNG_TRANSFORM_STRIP_FILLER: BEFORE+AFTER not supported\0".as_ptr()
                        as png_const_charp,
                );
            }

            /* Continue if ignored - this is the pre-1.6.10 behavior */
            png_set_filler(png_ptr, 0, PNG_FILLER_AFTER);
        } else if (transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE) != 0 {
            png_set_filler(png_ptr, 0, PNG_FILLER_BEFORE);
        }
    }

    /* Flip BGR pixels to RGB */
    if (transforms & PNG_TRANSFORM_BGR) != 0 {
        png_set_bgr(png_ptr);
    }

    /* Swap bytes of 16-bit files to most significant byte first */
    if (transforms & PNG_TRANSFORM_SWAP_ENDIAN) != 0 {
        png_set_swap(png_ptr);
    }

    /* Swap bits of 1-bit, 2-bit, 4-bit packed pixel formats */
    if (transforms & PNG_TRANSFORM_PACKSWAP) != 0 {
        png_set_packswap(png_ptr);
    }

    /* Invert the alpha channel from opacity to transparency */
    if (transforms & PNG_TRANSFORM_INVERT_ALPHA) != 0 {
        png_set_invert_alpha(png_ptr);
    }

    /* ----------------------- end of transformations ------------------- */

    /* Write the bits */
    png_write_image(png_ptr, (*info_ptr).row_pointers);

    /* It is REQUIRED to call this to finish writing the rest of the file */
    png_write_end(png_ptr, info_ptr);
}

/* Initialize the write structure - general purpose utility. */
/* png_image_write_init */
unsafe fn png_image_write_init(image: png_imagep) -> c_int {
    let mut png_ptr: png_structp = png_create_write_struct(
        PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
        image as png_voidp,
        /* png_safe_error is declared `-> !` in Rust; the C prototype it must
         * match here returns void.
         */
        Some(png_safe_error),
        Some(png_safe_warning),
    );

    if png_ptr != core::ptr::null_mut() {
        let mut info_ptr: png_infop = png_create_info_struct(png_ptr);

        if info_ptr != core::ptr::null_mut() {
            let control: png_controlp = png_malloc_warn(
                png_ptr,
                core::mem::size_of::<png_control>() as png_alloc_size_t,
            ) as png_controlp;

            if control != core::ptr::null_mut() {
                memset(
                    control as *mut c_void,
                    0,
                    core::mem::size_of::<png_control>(),
                );

                (*control).png_ptr = png_ptr;
                (*control).info_ptr = info_ptr;
                (*control).set_for_write(1);

                (*image).opaque = control;
                return 1;
            }

            /* Error clean up */
            png_destroy_info_struct(png_ptr, &mut info_ptr);
        }

        png_destroy_write_struct(&mut png_ptr, core::ptr::null_mut());
    }

    png_image_error(
        image,
        b"png_image_write_: out of memory\0".as_ptr() as png_const_charp,
    )
}

/* Write png_uint_16 input to a 16-bit PNG; the png_ptr has already been set to
 * do any necessary byte swapping.  The component order is defined by the
 * png_image format value.
 */
/* png_write_image_16bit */
unsafe extern "C" fn png_write_image_16bit(argument: png_voidp) -> c_int {
    let display: *mut png_image_write_control = argument as *mut png_image_write_control;
    let image: png_imagep = (*display).image;
    let png_ptr: png_structrp = (*(*image).opaque).png_ptr;

    let mut input_row: png_const_uint_16p = (*display).first_row as png_const_uint_16p;
    let mut output_row: png_uint_16p = (*display).local_row as png_uint_16p;
    let row_end: png_uint_16p;
    let channels: c_uint = if ((*image).format & PNG_FORMAT_FLAG_COLOR) != 0 {
        3
    } else {
        1
    };
    let mut aindex: c_int = 0;
    let mut y: png_uint_32 = (*image).height;

    if ((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0 {
        if ((*image).format & PNG_FORMAT_FLAG_AFIRST) != 0 {
            aindex = -1;
            input_row = input_row.add(1); /* To point to the first component */
            output_row = output_row.add(1);
        } else {
            aindex = channels as c_int;
        }
    } else {
        png_error(
            png_ptr,
            b"png_write_image: internal call error\0".as_ptr() as png_const_charp,
        );
    }

    /* Work out the output row end and count over this, note that the increment
     * above to 'row' means that row_end can actually be beyond the end of the
     * row; this is correct.
     */
    row_end = output_row.add((*image).width.wrapping_mul(channels.wrapping_add(1)) as usize);

    while y > 0 {
        let mut in_ptr: png_const_uint_16p = input_row;
        let mut out_ptr: png_uint_16p = output_row;

        while out_ptr < row_end {
            let alpha: png_uint_16 = *in_ptr.offset(aindex as isize);
            let mut reciprocal: png_uint_32 = 0;
            let mut c: c_int;

            *out_ptr.offset(aindex as isize) = alpha;

            /* Calculate a reciprocal.  The correct calculation is simply
             * component/alpha*65535 << 15. (I.e. 15 bits of precision); this
             * allows correct rounding by adding .5 before the shift.  'reciprocal'
             * is only initialized when required.
             */
            if alpha > 0 && alpha < 65535 {
                reciprocal = (((0xffff as png_uint_32) << 15) + ((alpha as png_uint_32) >> 1))
                    / (alpha as png_uint_32);
            }

            c = channels as c_int;
            loop
            /* always at least one channel */
            {
                let mut component: png_uint_16 = *in_ptr;
                in_ptr = in_ptr.add(1);

                /* The following gives 65535 for an alpha of 0, which is fine,
                 * otherwise if 0/0 is represented as some other value there is more
                 * likely to be a discontinuity which will probably damage
                 * compression when moving from a fully transparent area to a
                 * nearly transparent one.  (The assumption here is that opaque
                 * areas tend not to be 0 intensity.)
                 */
                if component >= alpha {
                    component = 65535;
                }
                /* component<alpha, so component/alpha is less than one and
                 * component*reciprocal is less than 2^31.
                 */
                else if component > 0 && alpha < 65535 {
                    let mut calc: png_uint_32 = (component as png_uint_32).wrapping_mul(reciprocal);
                    calc = calc.wrapping_add(16384); /* round to nearest */
                    component = (calc >> 15) as png_uint_16;
                }

                *out_ptr = component;
                out_ptr = out_ptr.add(1);

                c -= 1;
                if !(c > 0) {
                    break;
                }
            }

            /* Skip to next component (skip the intervening alpha channel) */
            in_ptr = in_ptr.add(1);
            out_ptr = out_ptr.add(1);
        }

        png_write_row(png_ptr, (*display).local_row as png_const_bytep);
        input_row = input_row.offset((*display).row_step / 2);

        y -= 1;
    }

    1
}

/* Given 16-bit input (1 to 4 channels) write 8-bit output.  If an alpha channel
 * is present it must be removed from the components, the components are then
 * written in sRGB encoding.  No components are added or removed.
 *
 * Calculate an alpha reciprocal to reverse pre-multiplication.  As above the
 * calculation can be done to 15 bits of accuracy; however, the output needs to
 * be scaled in the range 0..255*65535, so include that scaling here.
 */
/* UNP_RECIPROCAL(alpha) */
#[inline]
fn UNP_RECIPROCAL(alpha: png_uint_32) -> png_uint_32 {
    ((((0xffff as png_uint_32) * 0xff) << 7) + (alpha >> 1)) / alpha
}

/* png_unpremultiply */
unsafe fn png_unpremultiply(
    mut component: png_uint_32,
    alpha: png_uint_32,
    reciprocal: png_uint_32, /*from the above macro*/
) -> png_byte {
    /* The following gives 1.0 for an alpha of 0, which is fine, otherwise if 0/0
     * is represented as some other value there is more likely to be a
     * discontinuity which will probably damage compression when moving from a
     * fully transparent area to a nearly transparent one.  (The assumption here
     * is that opaque areas tend not to be 0 intensity.)
     *
     * There is a rounding problem here; if alpha is less than 128 it will end up
     * as 0 when scaled to 8 bits.  To avoid introducing spurious colors into the
     * output change for this too.
     */
    if component >= alpha || alpha < 128 {
        return 255;
    }
    /* component<alpha, so component/alpha is less than one and
     * component*reciprocal is less than 2^31.
     */
    else if component > 0 {
        /* The test is that alpha/257 (rounded) is less than 255, the first value
         * that becomes 255 is 65407.
         * NOTE: this must agree with the PNG_DIV257 macro (which must, therefore,
         * be exact!)  [Could also test reciprocal != 0]
         */
        if alpha < 65407 {
            component = component.wrapping_mul(reciprocal);
            component = component.wrapping_add(64); /* round to nearest */
            component >>= 7;
        } else {
            component = component.wrapping_mul(255);
        }

        /* Convert the component to sRGB. */
        return PNG_sRGB_FROM_LINEAR(component) as png_byte;
    } else {
        return 0;
    }
}

/* png_write_image_8bit */
unsafe extern "C" fn png_write_image_8bit(argument: png_voidp) -> c_int {
    let display: *mut png_image_write_control = argument as *mut png_image_write_control;
    let image: png_imagep = (*display).image;
    let png_ptr: png_structrp = (*(*image).opaque).png_ptr;

    let mut input_row: png_const_uint_16p = (*display).first_row as png_const_uint_16p;
    let mut output_row: png_bytep = (*display).local_row as png_bytep;
    let mut y: png_uint_32 = (*image).height;
    let channels: c_uint = if ((*image).format & PNG_FORMAT_FLAG_COLOR) != 0 {
        3
    } else {
        1
    };

    if ((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0 {
        let row_end: png_bytep;
        let aindex: c_int;

        if ((*image).format & PNG_FORMAT_FLAG_AFIRST) != 0 {
            aindex = -1;
            input_row = input_row.add(1); /* To point to the first component */
            output_row = output_row.add(1);
        } else {
            aindex = channels as c_int;
        }

        /* Use row_end in place of a loop counter: */
        row_end = output_row.add((*image).width.wrapping_mul(channels.wrapping_add(1)) as usize);

        while y > 0 {
            let mut in_ptr: png_const_uint_16p = input_row;
            let mut out_ptr: png_bytep = output_row;

            while out_ptr < row_end {
                let alpha: png_uint_16 = *in_ptr.offset(aindex as isize);
                let alphabyte: png_byte = PNG_DIV257(alpha as png_uint_32) as png_byte;
                let mut reciprocal: png_uint_32 = 0;
                let mut c: c_int;

                /* Scale and write the alpha channel. */
                *out_ptr.offset(aindex as isize) = alphabyte;

                if alphabyte > 0 && alphabyte < 255 {
                    reciprocal = UNP_RECIPROCAL(alpha as png_uint_32);
                }

                c = channels as c_int;
                loop
                /* always at least one channel */
                {
                    let v: png_uint_16 = *in_ptr;
                    in_ptr = in_ptr.add(1);
                    *out_ptr =
                        png_unpremultiply(v as png_uint_32, alpha as png_uint_32, reciprocal);
                    out_ptr = out_ptr.add(1);

                    c -= 1;
                    if !(c > 0) {
                        break;
                    }
                }

                /* Skip to next component (skip the intervening alpha channel) */
                in_ptr = in_ptr.add(1);
                out_ptr = out_ptr.add(1);
            } /* while out_ptr < row_end */

            png_write_row(png_ptr, (*display).local_row as png_const_bytep);
            input_row = input_row.offset((*display).row_step / 2);

            y -= 1;
        } /* while y */
    } else {
        /* No alpha channel, so the row_end really is the end of the row and it
         * is sufficient to loop over the components one by one.
         */
        let row_end: png_bytep = output_row.add((*image).width.wrapping_mul(channels) as usize);

        while y > 0 {
            let mut in_ptr: png_const_uint_16p = input_row;
            let mut out_ptr: png_bytep = output_row;

            while out_ptr < row_end {
                let mut component: png_uint_32 = *in_ptr as png_uint_32;
                in_ptr = in_ptr.add(1);

                component = component.wrapping_mul(255);
                *out_ptr = PNG_sRGB_FROM_LINEAR(component) as png_byte;
                out_ptr = out_ptr.add(1);
            }

            png_write_row(png_ptr, output_row as png_const_bytep);
            input_row = input_row.offset((*display).row_step / 2);

            y -= 1;
        }
    }

    1
}
