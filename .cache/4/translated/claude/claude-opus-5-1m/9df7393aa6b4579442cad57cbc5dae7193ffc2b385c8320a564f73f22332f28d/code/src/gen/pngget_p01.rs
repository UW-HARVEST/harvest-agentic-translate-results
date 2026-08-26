/* pngget.c lines 1..501 */

/* png_get_valid */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_valid(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    flag: png_uint_32,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut() && info_ptr != core::ptr::null_mut() {
        /* png_handle_PLTE() may have canceled a valid tRNS chunk but left the
         * 'valid' flag for the detection of duplicate chunks. Do not report a
         * valid tRNS chunk in this case.
         */
        if flag == PNG_INFO_tRNS && (*png_ptr).num_trans == 0 {
            return 0;
        }

        return (*info_ptr).valid & flag;
    }

    0
}

/* png_get_rowbytes */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_rowbytes(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> usize {
    if png_ptr != core::ptr::null_mut() && info_ptr != core::ptr::null_mut() {
        return (*info_ptr).rowbytes;
    }

    0
}

/* png_get_rows */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_rows(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_bytepp {
    if png_ptr != core::ptr::null_mut() && info_ptr != core::ptr::null_mut() {
        return (*info_ptr).row_pointers;
    }

    core::ptr::null_mut()
}

/* Easy access to info, added in libpng-0.99 */
/* png_get_image_width */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_image_width(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut() && info_ptr != core::ptr::null_mut() {
        return (*info_ptr).width;
    }

    0
}

/* png_get_image_height */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_image_height(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut() && info_ptr != core::ptr::null_mut() {
        return (*info_ptr).height;
    }

    0
}

/* png_get_bit_depth */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_bit_depth(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    if png_ptr != core::ptr::null_mut() && info_ptr != core::ptr::null_mut() {
        return (*info_ptr).bit_depth;
    }

    0
}

/* png_get_color_type */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_color_type(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    if png_ptr != core::ptr::null_mut() && info_ptr != core::ptr::null_mut() {
        return (*info_ptr).color_type;
    }

    0
}

/* png_get_filter_type */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_filter_type(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    if png_ptr != core::ptr::null_mut() && info_ptr != core::ptr::null_mut() {
        return (*info_ptr).filter_type;
    }

    0
}

/* png_get_interlace_type */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_interlace_type(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    if png_ptr != core::ptr::null_mut() && info_ptr != core::ptr::null_mut() {
        return (*info_ptr).interlace_type;
    }

    0
}

/* png_get_compression_type */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_compression_type(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    if png_ptr != core::ptr::null_mut() && info_ptr != core::ptr::null_mut() {
        return (*info_ptr).compression_type;
    }

    0
}

/* png_get_x_pixels_per_meter */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_pixels_per_meter(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_pHYs) != 0
    {
        if (*info_ptr).phys_unit_type as c_int == PNG_RESOLUTION_METER {
            return (*info_ptr).x_pixels_per_unit;
        }
    }

    0
}

/* png_get_y_pixels_per_meter */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_pixels_per_meter(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_pHYs) != 0
    {
        if (*info_ptr).phys_unit_type as c_int == PNG_RESOLUTION_METER {
            return (*info_ptr).y_pixels_per_unit;
        }
    }

    0
}

/* png_get_pixels_per_meter */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pixels_per_meter(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_pHYs) != 0
    {
        if (*info_ptr).phys_unit_type as c_int == PNG_RESOLUTION_METER
            && (*info_ptr).x_pixels_per_unit == (*info_ptr).y_pixels_per_unit
        {
            return (*info_ptr).x_pixels_per_unit;
        }
    }

    0
}

/* png_get_pixel_aspect_ratio */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pixel_aspect_ratio(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> f32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_pHYs) != 0
    {
        if (*info_ptr).x_pixels_per_unit != 0 {
            return (*info_ptr).y_pixels_per_unit as f32 / (*info_ptr).x_pixels_per_unit as f32;
        }
    }

    0.0f32
}

/* png_get_pixel_aspect_ratio_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pixel_aspect_ratio_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_fixed_point {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_pHYs) != 0
        && (*info_ptr).x_pixels_per_unit > 0
        && (*info_ptr).y_pixels_per_unit > 0
        && (*info_ptr).x_pixels_per_unit <= PNG_UINT_31_MAX
        && (*info_ptr).y_pixels_per_unit <= PNG_UINT_31_MAX
    {
        let mut res: png_fixed_point = 0;

        /* The following casts work because a PNG 4 byte integer only has a valid
         * range of 0..2^31-1; otherwise the cast might overflow.
         */
        if png_muldiv(
            &mut res,
            (*info_ptr).y_pixels_per_unit as png_int_32,
            PNG_FP_1,
            (*info_ptr).x_pixels_per_unit as png_int_32,
        ) != 0
        {
            return res;
        }
    }

    0
}

/* png_get_x_offset_microns */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_offset_microns(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_int_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_oFFs) != 0
    {
        if (*info_ptr).offset_unit_type as c_int == PNG_OFFSET_MICROMETER {
            return (*info_ptr).x_offset;
        }
    }

    0
}

/* png_get_y_offset_microns */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_offset_microns(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_int_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_oFFs) != 0
    {
        if (*info_ptr).offset_unit_type as c_int == PNG_OFFSET_MICROMETER {
            return (*info_ptr).y_offset;
        }
    }

    0
}

/* png_get_x_offset_pixels */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_offset_pixels(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_int_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_oFFs) != 0
    {
        if (*info_ptr).offset_unit_type as c_int == PNG_OFFSET_PIXEL {
            return (*info_ptr).x_offset;
        }
    }

    0
}

/* png_get_y_offset_pixels */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_offset_pixels(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_int_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_oFFs) != 0
    {
        if (*info_ptr).offset_unit_type as c_int == PNG_OFFSET_PIXEL {
            return (*info_ptr).y_offset;
        }
    }

    0
}

/* ppi_from_ppm (static) */
unsafe fn ppi_from_ppm(ppm: png_uint_32) -> png_uint_32 {
    /* The argument is a PNG unsigned integer, so it is not permitted
     * to be bigger than 2^31.
     */
    let mut result: png_fixed_point = 0;
    if ppm <= PNG_UINT_31_MAX && png_muldiv(&mut result, ppm as png_int_32, 127, 5000) != 0 {
        return result as png_uint_32;
    }

    /* Overflow. */
    0
}

/* png_get_pixels_per_inch */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pixels_per_inch(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    ppi_from_ppm(png_get_pixels_per_meter(png_ptr, info_ptr))
}

/* png_get_x_pixels_per_inch */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_pixels_per_inch(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    ppi_from_ppm(png_get_x_pixels_per_meter(png_ptr, info_ptr))
}

/* png_get_y_pixels_per_inch */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_pixels_per_inch(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    ppi_from_ppm(png_get_y_pixels_per_meter(png_ptr, info_ptr))
}

/* png_fixed_inches_from_microns (static) */
unsafe fn png_fixed_inches_from_microns(
    png_ptr: png_const_structrp,
    microns: png_int_32,
) -> png_fixed_point {
    /* Convert from meters * 1,000,000 to inches * 100,000, meters to
     * inches is simply *(100/2.54), so we want *(10/2.54) == 500/127.
     * Notice that this can overflow - a warning is output and 0 is
     * returned.
     */
    let mut result: png_fixed_point = 0;

    if png_muldiv(&mut result, microns, 500, 127) != 0 {
        return result;
    }

    png_warning(
        png_ptr,
        b"fixed point overflow ignored\0".as_ptr() as png_const_charp,
    );
    0
}

/* png_get_x_offset_inches_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_offset_inches_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_fixed_point {
    png_fixed_inches_from_microns(png_ptr, png_get_x_offset_microns(png_ptr, info_ptr))
}

/* png_get_y_offset_inches_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_offset_inches_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_fixed_point {
    png_fixed_inches_from_microns(png_ptr, png_get_y_offset_microns(png_ptr, info_ptr))
}

/* png_get_x_offset_inches */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_offset_inches(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> f32 {
    /* To avoid the overflow do the conversion directly in floating
     * point.
     */
    (png_get_x_offset_microns(png_ptr, info_ptr) as f64 * 0.00003937) as f32
}

/* png_get_y_offset_inches */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_offset_inches(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> f32 {
    /* To avoid the overflow do the conversion directly in floating
     * point.
     */
    (png_get_y_offset_microns(png_ptr, info_ptr) as f64 * 0.00003937) as f32
}

/* png_get_pHYs_dpi */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pHYs_dpi(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    res_x: *mut png_uint_32,
    res_y: *mut png_uint_32,
    unit_type: *mut c_int,
) -> png_uint_32 {
    let mut retval: png_uint_32 = 0;

    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_pHYs) != 0
    {
        if res_x != core::ptr::null_mut() {
            *res_x = (*info_ptr).x_pixels_per_unit;
            retval |= PNG_INFO_pHYs;
        }

        if res_y != core::ptr::null_mut() {
            *res_y = (*info_ptr).y_pixels_per_unit;
            retval |= PNG_INFO_pHYs;
        }

        if unit_type != core::ptr::null_mut() {
            *unit_type = (*info_ptr).phys_unit_type as c_int;
            retval |= PNG_INFO_pHYs;

            if *unit_type == 1 {
                if res_x != core::ptr::null_mut() {
                    *res_x = (*res_x as f64 * 0.0254 + 0.50) as png_uint_32;
                }
                if res_y != core::ptr::null_mut() {
                    *res_y = (*res_y as f64 * 0.0254 + 0.50) as png_uint_32;
                }
            }
        }
    }

    retval
}

/* png_get_channels really belongs in here, too, but it's been around longer */

/* png_get_channels */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_channels(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    if png_ptr != core::ptr::null_mut() && info_ptr != core::ptr::null_mut() {
        return (*info_ptr).channels;
    }

    0
}

/* png_get_signature */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_signature(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_const_bytep {
    if png_ptr != core::ptr::null_mut() && info_ptr != core::ptr::null_mut() {
        return core::ptr::addr_of!((*info_ptr).signature) as png_const_bytep;
    }

    core::ptr::null()
}
