//! Translation of c_src/src/pngget.c lines 1..1369
use crate::prelude::*;

/* atof(s) == strtod(s, NULL); libpng uses atof() but it is not in sys.rs. */
#[inline]
unsafe fn atof(s: *const c_char) -> c_double {
    strtod(s, core::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_valid(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    flag: png_uint_32,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_rowbytes(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> usize {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).rowbytes;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_rows(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_bytepp {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).row_pointers;
    }

    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_image_width(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).width;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_image_height(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).height;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_bit_depth(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).bit_depth;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_color_type(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).color_type;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_filter_type(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).filter_type;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_interlace_type(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).interlace_type;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_compression_type(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).compression_type;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_pixels_per_meter(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_pHYs) != 0 {
        if (*info_ptr).phys_unit_type as c_int == PNG_RESOLUTION_METER {
            return (*info_ptr).x_pixels_per_unit;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_pixels_per_meter(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_pHYs) != 0 {
        if (*info_ptr).phys_unit_type as c_int == PNG_RESOLUTION_METER {
            return (*info_ptr).y_pixels_per_unit;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pixels_per_meter(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_pHYs) != 0 {
        if (*info_ptr).phys_unit_type as c_int == PNG_RESOLUTION_METER
            && (*info_ptr).x_pixels_per_unit == (*info_ptr).y_pixels_per_unit
        {
            return (*info_ptr).x_pixels_per_unit;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pixel_aspect_ratio(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> f32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_pHYs) != 0 {
        if (*info_ptr).x_pixels_per_unit != 0 {
            return (*info_ptr).y_pixels_per_unit as f32 / (*info_ptr).x_pixels_per_unit as f32;
        }
    }

    0.0f32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pixel_aspect_ratio_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_fixed_point {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_offset_microns(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_int_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_oFFs) != 0 {
        if (*info_ptr).offset_unit_type as c_int == PNG_OFFSET_MICROMETER {
            return (*info_ptr).x_offset;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_offset_microns(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_int_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_oFFs) != 0 {
        if (*info_ptr).offset_unit_type as c_int == PNG_OFFSET_MICROMETER {
            return (*info_ptr).y_offset;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_offset_pixels(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_int_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_oFFs) != 0 {
        if (*info_ptr).offset_unit_type as c_int == PNG_OFFSET_PIXEL {
            return (*info_ptr).x_offset;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_offset_pixels(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_int_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_oFFs) != 0 {
        if (*info_ptr).offset_unit_type as c_int == PNG_OFFSET_PIXEL {
            return (*info_ptr).y_offset;
        }
    }

    0
}

/* static in C */
pub unsafe extern "C" fn ppi_from_ppm(ppm: png_uint_32) -> png_uint_32 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pixels_per_inch(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    ppi_from_ppm(png_get_pixels_per_meter(png_ptr, info_ptr))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_pixels_per_inch(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    ppi_from_ppm(png_get_x_pixels_per_meter(png_ptr, info_ptr))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_pixels_per_inch(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    ppi_from_ppm(png_get_y_pixels_per_meter(png_ptr, info_ptr))
}

/* static in C */
pub unsafe extern "C" fn png_fixed_inches_from_microns(
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

    png_warning(png_ptr, cstr(b"fixed point overflow ignored\0"));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_offset_inches_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_fixed_point {
    png_fixed_inches_from_microns(png_ptr, png_get_x_offset_microns(png_ptr, info_ptr))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_offset_inches_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_fixed_point {
    png_fixed_inches_from_microns(png_ptr, png_get_y_offset_microns(png_ptr, info_ptr))
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pHYs_dpi(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    res_x: *mut png_uint_32,
    res_y: *mut png_uint_32,
    unit_type: *mut c_int,
) -> png_uint_32 {
    let mut retval: png_uint_32 = 0;

    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_pHYs) != 0 {
        if !res_x.is_null() {
            *res_x = (*info_ptr).x_pixels_per_unit;
            retval |= PNG_INFO_pHYs;
        }

        if !res_y.is_null() {
            *res_y = (*info_ptr).y_pixels_per_unit;
            retval |= PNG_INFO_pHYs;
        }

        if !unit_type.is_null() {
            *unit_type = (*info_ptr).phys_unit_type as c_int;
            retval |= PNG_INFO_pHYs;

            if *unit_type == 1 {
                if !res_x.is_null() {
                    *res_x = (*res_x as f64 * 0.0254 + 0.50) as png_uint_32;
                }
                if !res_y.is_null() {
                    *res_y = (*res_y as f64 * 0.0254 + 0.50) as png_uint_32;
                }
            }
        }
    }

    retval
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_channels(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).channels;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_signature(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_const_bytep {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).signature.as_ptr();
    }

    core::ptr::null()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_bKGD(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    background: *mut png_color_16p,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_bKGD) != 0
        && !background.is_null()
    {
        *background = &mut (*info_ptr).background;
        return PNG_INFO_bKGD;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cHRM(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    whitex: *mut f64,
    whitey: *mut f64,
    redx: *mut f64,
    redy: *mut f64,
    greenx: *mut f64,
    greeny: *mut f64,
    bluex: *mut f64,
    bluey: *mut f64,
) -> png_uint_32 {
    /* PNGv3: this just returns the values store from the cHRM, if any. */
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_cHRM) != 0 {
        if !whitex.is_null() {
            *whitex = png_float(png_ptr, (*info_ptr).cHRM.whitex, cstr(b"cHRM wx\0"));
        }
        if !whitey.is_null() {
            *whitey = png_float(png_ptr, (*info_ptr).cHRM.whitey, cstr(b"cHRM wy\0"));
        }
        if !redx.is_null() {
            *redx = png_float(png_ptr, (*info_ptr).cHRM.redx, cstr(b"cHRM rx\0"));
        }
        if !redy.is_null() {
            *redy = png_float(png_ptr, (*info_ptr).cHRM.redy, cstr(b"cHRM ry\0"));
        }
        if !greenx.is_null() {
            *greenx = png_float(png_ptr, (*info_ptr).cHRM.greenx, cstr(b"cHRM gx\0"));
        }
        if !greeny.is_null() {
            *greeny = png_float(png_ptr, (*info_ptr).cHRM.greeny, cstr(b"cHRM gy\0"));
        }
        if !bluex.is_null() {
            *bluex = png_float(png_ptr, (*info_ptr).cHRM.bluex, cstr(b"cHRM bx\0"));
        }
        if !bluey.is_null() {
            *bluey = png_float(png_ptr, (*info_ptr).cHRM.bluey, cstr(b"cHRM by\0"));
        }
        return PNG_INFO_cHRM;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cHRM_XYZ(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    red_X: *mut f64,
    red_Y: *mut f64,
    red_Z: *mut f64,
    green_X: *mut f64,
    green_Y: *mut f64,
    green_Z: *mut f64,
    blue_X: *mut f64,
    blue_Y: *mut f64,
    blue_Z: *mut f64,
) -> png_uint_32 {
    let mut XYZ: png_XYZ = png_XYZ::default();

    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_cHRM) != 0
        && png_XYZ_from_xy(&mut XYZ, &(*info_ptr).cHRM) == 0
    {
        if !red_X.is_null() {
            *red_X = png_float(png_ptr, XYZ.red_X, cstr(b"cHRM red X\0"));
        }
        if !red_Y.is_null() {
            *red_Y = png_float(png_ptr, XYZ.red_Y, cstr(b"cHRM red Y\0"));
        }
        if !red_Z.is_null() {
            *red_Z = png_float(png_ptr, XYZ.red_Z, cstr(b"cHRM red Z\0"));
        }
        if !green_X.is_null() {
            *green_X = png_float(png_ptr, XYZ.green_X, cstr(b"cHRM green X\0"));
        }
        if !green_Y.is_null() {
            *green_Y = png_float(png_ptr, XYZ.green_Y, cstr(b"cHRM green Y\0"));
        }
        if !green_Z.is_null() {
            *green_Z = png_float(png_ptr, XYZ.green_Z, cstr(b"cHRM green Z\0"));
        }
        if !blue_X.is_null() {
            *blue_X = png_float(png_ptr, XYZ.blue_X, cstr(b"cHRM blue X\0"));
        }
        if !blue_Y.is_null() {
            *blue_Y = png_float(png_ptr, XYZ.blue_Y, cstr(b"cHRM blue Y\0"));
        }
        if !blue_Z.is_null() {
            *blue_Z = png_float(png_ptr, XYZ.blue_Z, cstr(b"cHRM blue Z\0"));
        }
        return PNG_INFO_cHRM;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cHRM_XYZ_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    int_red_X: *mut png_fixed_point,
    int_red_Y: *mut png_fixed_point,
    int_red_Z: *mut png_fixed_point,
    int_green_X: *mut png_fixed_point,
    int_green_Y: *mut png_fixed_point,
    int_green_Z: *mut png_fixed_point,
    int_blue_X: *mut png_fixed_point,
    int_blue_Y: *mut png_fixed_point,
    int_blue_Z: *mut png_fixed_point,
) -> png_uint_32 {
    let mut XYZ: png_XYZ = png_XYZ::default();

    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_cHRM) != 0u32
        && png_XYZ_from_xy(&mut XYZ, &(*info_ptr).cHRM) == 0
    {
        if !int_red_X.is_null() {
            *int_red_X = XYZ.red_X;
        }
        if !int_red_Y.is_null() {
            *int_red_Y = XYZ.red_Y;
        }
        if !int_red_Z.is_null() {
            *int_red_Z = XYZ.red_Z;
        }
        if !int_green_X.is_null() {
            *int_green_X = XYZ.green_X;
        }
        if !int_green_Y.is_null() {
            *int_green_Y = XYZ.green_Y;
        }
        if !int_green_Z.is_null() {
            *int_green_Z = XYZ.green_Z;
        }
        if !int_blue_X.is_null() {
            *int_blue_X = XYZ.blue_X;
        }
        if !int_blue_Y.is_null() {
            *int_blue_Y = XYZ.blue_Y;
        }
        if !int_blue_Z.is_null() {
            *int_blue_Z = XYZ.blue_Z;
        }
        return PNG_INFO_cHRM;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cHRM_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    whitex: *mut png_fixed_point,
    whitey: *mut png_fixed_point,
    redx: *mut png_fixed_point,
    redy: *mut png_fixed_point,
    greenx: *mut png_fixed_point,
    greeny: *mut png_fixed_point,
    bluex: *mut png_fixed_point,
    bluey: *mut png_fixed_point,
) -> png_uint_32 {
    /* PNGv3: this just returns the values store from the cHRM, if any. */
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_cHRM) != 0 {
        if !whitex.is_null() {
            *whitex = (*info_ptr).cHRM.whitex;
        }
        if !whitey.is_null() {
            *whitey = (*info_ptr).cHRM.whitey;
        }
        if !redx.is_null() {
            *redx = (*info_ptr).cHRM.redx;
        }
        if !redy.is_null() {
            *redy = (*info_ptr).cHRM.redy;
        }
        if !greenx.is_null() {
            *greenx = (*info_ptr).cHRM.greenx;
        }
        if !greeny.is_null() {
            *greeny = (*info_ptr).cHRM.greeny;
        }
        if !bluex.is_null() {
            *bluex = (*info_ptr).cHRM.bluex;
        }
        if !bluey.is_null() {
            *bluey = (*info_ptr).cHRM.bluey;
        }
        return PNG_INFO_cHRM;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_gAMA_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    file_gamma: *mut png_fixed_point,
) -> png_uint_32 {
    /* PNGv3 compatibility: only report gAMA if it is really present. */
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_gAMA) != 0 {
        if !file_gamma.is_null() {
            *file_gamma = (*info_ptr).gamma;
        }
        return PNG_INFO_gAMA;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_gAMA(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    file_gamma: *mut f64,
) -> png_uint_32 {
    /* PNGv3 compatibility: only report gAMA if it is really present. */
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_gAMA) != 0 {
        if !file_gamma.is_null() {
            *file_gamma = png_float(png_ptr, (*info_ptr).gamma, cstr(b"gAMA\0"));
        }

        return PNG_INFO_gAMA;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sRGB(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    file_srgb_intent: *mut c_int,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_sRGB) != 0 {
        if !file_srgb_intent.is_null() {
            *file_srgb_intent = (*info_ptr).rendering_intent;
        }
        return PNG_INFO_sRGB;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_iCCP(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    name: png_charpp,
    compression_type: *mut c_int,
    profile: png_bytepp,
    proflen: *mut png_uint_32,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_iCCP) != 0
        && !name.is_null()
        && !profile.is_null()
        && !proflen.is_null()
    {
        *name = (*info_ptr).iccp_name;
        *profile = (*info_ptr).iccp_profile;
        *proflen = png_get_uint_32((*info_ptr).iccp_profile);
        /* This is somewhat irrelevant since the profile data returned has
         * actually been uncompressed.
         */
        if !compression_type.is_null() {
            *compression_type = PNG_COMPRESSION_TYPE_BASE;
        }
        return PNG_INFO_iCCP;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sPLT(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    spalettes: *mut png_sPLT_tp,
) -> c_int {
    if !png_ptr.is_null() && !info_ptr.is_null() && !spalettes.is_null() {
        *spalettes = (*info_ptr).splt_palettes;
        return (*info_ptr).splt_palettes_num;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cICP(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    colour_primaries: png_bytep,
    transfer_function: png_bytep,
    matrix_coefficients: png_bytep,
    video_full_range_flag: png_bytep,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_cICP) != 0
        && !colour_primaries.is_null()
        && !transfer_function.is_null()
        && !matrix_coefficients.is_null()
        && !video_full_range_flag.is_null()
    {
        *colour_primaries = (*info_ptr).cicp_colour_primaries;
        *transfer_function = (*info_ptr).cicp_transfer_function;
        *matrix_coefficients = (*info_ptr).cicp_matrix_coefficients;
        *video_full_range_flag = (*info_ptr).cicp_video_full_range_flag;
        return PNG_INFO_cICP;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cLLI_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    maxCLL: png_uint_32p,
    maxFALL: png_uint_32p,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_cLLI) != 0 {
        if !maxCLL.is_null() {
            *maxCLL = (*info_ptr).maxCLL;
        }
        if !maxFALL.is_null() {
            *maxFALL = (*info_ptr).maxFALL;
        }
        return PNG_INFO_cLLI;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cLLI(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    maxCLL: *mut f64,
    maxFALL: *mut f64,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_cLLI) != 0 {
        if !maxCLL.is_null() {
            *maxCLL = (*info_ptr).maxCLL as f64 * 0.0001;
        }
        if !maxFALL.is_null() {
            *maxFALL = (*info_ptr).maxFALL as f64 * 0.0001;
        }
        return PNG_INFO_cLLI;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_mDCV_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    white_x: *mut png_fixed_point,
    white_y: *mut png_fixed_point,
    red_x: *mut png_fixed_point,
    red_y: *mut png_fixed_point,
    green_x: *mut png_fixed_point,
    green_y: *mut png_fixed_point,
    blue_x: *mut png_fixed_point,
    blue_y: *mut png_fixed_point,
    mastering_maxDL: png_uint_32p,
    mastering_minDL: png_uint_32p,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_mDCV) != 0 {
        if !white_x.is_null() {
            *white_x = (*info_ptr).mastering_white_x as c_int as png_fixed_point * 2;
        }
        if !white_y.is_null() {
            *white_y = (*info_ptr).mastering_white_y as c_int as png_fixed_point * 2;
        }
        if !red_x.is_null() {
            *red_x = (*info_ptr).mastering_red_x as c_int as png_fixed_point * 2;
        }
        if !red_y.is_null() {
            *red_y = (*info_ptr).mastering_red_y as c_int as png_fixed_point * 2;
        }
        if !green_x.is_null() {
            *green_x = (*info_ptr).mastering_green_x as c_int as png_fixed_point * 2;
        }
        if !green_y.is_null() {
            *green_y = (*info_ptr).mastering_green_y as c_int as png_fixed_point * 2;
        }
        if !blue_x.is_null() {
            *blue_x = (*info_ptr).mastering_blue_x as c_int as png_fixed_point * 2;
        }
        if !blue_y.is_null() {
            *blue_y = (*info_ptr).mastering_blue_y as c_int as png_fixed_point * 2;
        }
        if !mastering_maxDL.is_null() {
            *mastering_maxDL = (*info_ptr).mastering_maxDL;
        }
        if !mastering_minDL.is_null() {
            *mastering_minDL = (*info_ptr).mastering_minDL;
        }
        return PNG_INFO_mDCV;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_mDCV(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    white_x: *mut f64,
    white_y: *mut f64,
    red_x: *mut f64,
    red_y: *mut f64,
    green_x: *mut f64,
    green_y: *mut f64,
    blue_x: *mut f64,
    blue_y: *mut f64,
    mastering_maxDL: *mut f64,
    mastering_minDL: *mut f64,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_mDCV) != 0 {
        if !white_x.is_null() {
            *white_x = (*info_ptr).mastering_white_x as f64 * 0.00002;
        }
        if !white_y.is_null() {
            *white_y = (*info_ptr).mastering_white_y as f64 * 0.00002;
        }
        if !red_x.is_null() {
            *red_x = (*info_ptr).mastering_red_x as f64 * 0.00002;
        }
        if !red_y.is_null() {
            *red_y = (*info_ptr).mastering_red_y as f64 * 0.00002;
        }
        if !green_x.is_null() {
            *green_x = (*info_ptr).mastering_green_x as f64 * 0.00002;
        }
        if !green_y.is_null() {
            *green_y = (*info_ptr).mastering_green_y as f64 * 0.00002;
        }
        if !blue_x.is_null() {
            *blue_x = (*info_ptr).mastering_blue_x as f64 * 0.00002;
        }
        if !blue_y.is_null() {
            *blue_y = (*info_ptr).mastering_blue_y as f64 * 0.00002;
        }
        if !mastering_maxDL.is_null() {
            *mastering_maxDL = (*info_ptr).mastering_maxDL as f64 * 0.0001;
        }
        if !mastering_minDL.is_null() {
            *mastering_minDL = (*info_ptr).mastering_minDL as f64 * 0.0001;
        }
        return PNG_INFO_mDCV;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_eXIf(
    png_ptr: png_const_structrp,
    _info_ptr: png_inforp,
    _exif: *mut png_bytep,
) -> png_uint_32 {
    png_warning(
        png_ptr,
        cstr(b"png_get_eXIf does not work; use png_get_eXIf_1\0"),
    );
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_eXIf_1(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    num_exif: *mut png_uint_32,
    exif: *mut png_bytep,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_eXIf) != 0
        && !exif.is_null()
    {
        *num_exif = (*info_ptr).num_exif;
        *exif = (*info_ptr).exif;
        return PNG_INFO_eXIf;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_hIST(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    hist: *mut png_uint_16p,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_hIST) != 0
        && !hist.is_null()
    {
        *hist = (*info_ptr).hist;
        return PNG_INFO_hIST;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_IHDR(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    width: *mut png_uint_32,
    height: *mut png_uint_32,
    bit_depth: *mut c_int,
    color_type: *mut c_int,
    interlace_type: *mut c_int,
    compression_type: *mut c_int,
    filter_type: *mut c_int,
) -> png_uint_32 {
    if png_ptr.is_null() || info_ptr.is_null() {
        return 0;
    }

    if !width.is_null() {
        *width = (*info_ptr).width;
    }

    if !height.is_null() {
        *height = (*info_ptr).height;
    }

    if !bit_depth.is_null() {
        *bit_depth = (*info_ptr).bit_depth as c_int;
    }

    if !color_type.is_null() {
        *color_type = (*info_ptr).color_type as c_int;
    }

    if !compression_type.is_null() {
        *compression_type = (*info_ptr).compression_type as c_int;
    }

    if !filter_type.is_null() {
        *filter_type = (*info_ptr).filter_type as c_int;
    }

    if !interlace_type.is_null() {
        *interlace_type = (*info_ptr).interlace_type as c_int;
    }

    /* This is redundant if we can be sure that the info_ptr values were all
     * assigned in png_set_IHDR().  We do the check anyhow in case an
     * application has ignored our advice not to mess with the members
     * of info_ptr directly.
     */
    png_check_IHDR(
        png_ptr,
        (*info_ptr).width,
        (*info_ptr).height,
        (*info_ptr).bit_depth as c_int,
        (*info_ptr).color_type as c_int,
        (*info_ptr).interlace_type as c_int,
        (*info_ptr).compression_type as c_int,
        (*info_ptr).filter_type as c_int,
    );

    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_oFFs(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    offset_x: *mut png_int_32,
    offset_y: *mut png_int_32,
    unit_type: *mut c_int,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_oFFs) != 0
        && !offset_x.is_null()
        && !offset_y.is_null()
        && !unit_type.is_null()
    {
        *offset_x = (*info_ptr).x_offset;
        *offset_y = (*info_ptr).y_offset;
        *unit_type = (*info_ptr).offset_unit_type as c_int;
        return PNG_INFO_oFFs;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pCAL(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    purpose: *mut png_charp,
    X0: *mut png_int_32,
    X1: *mut png_int_32,
    type_: *mut c_int,
    nparams: *mut c_int,
    units: *mut png_charp,
    params: *mut png_charpp,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_pCAL) != 0
        && !purpose.is_null()
        && !X0.is_null()
        && !X1.is_null()
        && !type_.is_null()
        && !nparams.is_null()
        && !units.is_null()
        && !params.is_null()
    {
        *purpose = (*info_ptr).pcal_purpose;
        *X0 = (*info_ptr).pcal_X0;
        *X1 = (*info_ptr).pcal_X1;
        *type_ = (*info_ptr).pcal_type as c_int;
        *nparams = (*info_ptr).pcal_nparams as c_int;
        *units = (*info_ptr).pcal_units;
        *params = (*info_ptr).pcal_params;
        return PNG_INFO_pCAL;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sCAL_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    unit: *mut c_int,
    width: *mut png_fixed_point,
    height: *mut png_fixed_point,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_sCAL) != 0 {
        *unit = (*info_ptr).scal_unit as c_int;
        /*TODO: make this work without FP support; the API is currently eliminated
         * if neither floating point APIs nor internal floating point arithmetic
         * are enabled.
         */
        *width = png_fixed(
            png_ptr,
            atof((*info_ptr).scal_s_width),
            cstr(b"sCAL width\0"),
        );
        *height = png_fixed(
            png_ptr,
            atof((*info_ptr).scal_s_height),
            cstr(b"sCAL height\0"),
        );
        return PNG_INFO_sCAL;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sCAL(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    unit: *mut c_int,
    width: *mut f64,
    height: *mut f64,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_sCAL) != 0 {
        *unit = (*info_ptr).scal_unit as c_int;
        *width = atof((*info_ptr).scal_s_width);
        *height = atof((*info_ptr).scal_s_height);
        return PNG_INFO_sCAL;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sCAL_s(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    unit: *mut c_int,
    width: *mut png_charp,
    height: *mut png_charp,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_sCAL) != 0 {
        *unit = (*info_ptr).scal_unit as c_int;
        *width = (*info_ptr).scal_s_width;
        *height = (*info_ptr).scal_s_height;
        return PNG_INFO_sCAL;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pHYs(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    res_x: *mut png_uint_32,
    res_y: *mut png_uint_32,
    unit_type: *mut c_int,
) -> png_uint_32 {
    let mut retval: png_uint_32 = 0;

    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_pHYs) != 0 {
        if !res_x.is_null() {
            *res_x = (*info_ptr).x_pixels_per_unit;
            retval |= PNG_INFO_pHYs;
        }

        if !res_y.is_null() {
            *res_y = (*info_ptr).y_pixels_per_unit;
            retval |= PNG_INFO_pHYs;
        }

        if !unit_type.is_null() {
            *unit_type = (*info_ptr).phys_unit_type as c_int;
            retval |= PNG_INFO_pHYs;
        }
    }

    retval
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_PLTE(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    palette: *mut png_colorp,
    num_palette: *mut c_int,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_PLTE) != 0
        && !palette.is_null()
    {
        *palette = (*info_ptr).palette;
        *num_palette = (*info_ptr).num_palette as c_int;
        return PNG_INFO_PLTE;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sBIT(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    sig_bit: *mut png_color_8p,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_sBIT) != 0
        && !sig_bit.is_null()
    {
        *sig_bit = &mut (*info_ptr).sig_bit;
        return PNG_INFO_sBIT;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_text(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    text_ptr: *mut png_textp,
    num_text: *mut c_int,
) -> c_int {
    if !png_ptr.is_null() && !info_ptr.is_null() && (*info_ptr).num_text > 0 {
        if !text_ptr.is_null() {
            *text_ptr = (*info_ptr).text;
        }

        if !num_text.is_null() {
            *num_text = (*info_ptr).num_text;
        }

        return (*info_ptr).num_text;
    }

    if !num_text.is_null() {
        *num_text = 0;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_tIME(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mod_time: *mut png_timep,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_tIME) != 0
        && !mod_time.is_null()
    {
        *mod_time = &mut (*info_ptr).mod_time;
        return PNG_INFO_tIME;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_tRNS(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    trans_alpha: *mut png_bytep,
    num_trans: *mut c_int,
    trans_color: *mut png_color_16p,
) -> png_uint_32 {
    let mut retval: png_uint_32 = 0;

    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_tRNS) != 0 {
        if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            if !trans_alpha.is_null() {
                *trans_alpha = (*info_ptr).trans_alpha;
                retval |= PNG_INFO_tRNS;
            }

            if !trans_color.is_null() {
                *trans_color = &mut (*info_ptr).trans_color;
            }
        } else
        /* if (info_ptr->color_type != PNG_COLOR_TYPE_PALETTE) */
        {
            if !trans_color.is_null() {
                *trans_color = &mut (*info_ptr).trans_color;
                retval |= PNG_INFO_tRNS;
            }

            if !trans_alpha.is_null() {
                *trans_alpha = core::ptr::null_mut();
            }
        }

        if !num_trans.is_null() {
            *num_trans = (*info_ptr).num_trans as c_int;
            retval |= PNG_INFO_tRNS;
        }
    }

    retval
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_unknown_chunks(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    unknowns: *mut png_unknown_chunkp,
) -> c_int {
    if !png_ptr.is_null() && !info_ptr.is_null() && !unknowns.is_null() {
        *unknowns = (*info_ptr).unknown_chunks;
        return (*info_ptr).unknown_chunks_num;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_rgb_to_gray_status(png_ptr: png_const_structrp) -> png_byte {
    (if !png_ptr.is_null() {
        (*png_ptr).rgb_to_gray_status
    } else {
        0
    }) as png_byte
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_user_chunk_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if !png_ptr.is_null() {
        (*png_ptr).user_chunk_ptr
    } else {
        core::ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_compression_buffer_size(png_ptr: png_const_structrp) -> usize {
    if png_ptr.is_null() {
        return 0;
    }

    if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 {
        (*png_ptr).IDAT_read_size as usize
    } else {
        (*png_ptr).zbuffer_size as usize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_user_width_max(png_ptr: png_const_structrp) -> png_uint_32 {
    if !png_ptr.is_null() {
        (*png_ptr).user_width_max
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_user_height_max(png_ptr: png_const_structrp) -> png_uint_32 {
    if !png_ptr.is_null() {
        (*png_ptr).user_height_max
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_chunk_cache_max(png_ptr: png_const_structrp) -> png_uint_32 {
    if !png_ptr.is_null() {
        (*png_ptr).user_chunk_cache_max
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_chunk_malloc_max(png_ptr: png_const_structrp) -> png_alloc_size_t {
    if !png_ptr.is_null() {
        (*png_ptr).user_chunk_malloc_max
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_io_state(png_ptr: png_const_structrp) -> png_uint_32 {
    (*png_ptr).io_state
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_io_chunk_type(png_ptr: png_const_structrp) -> png_uint_32 {
    (*png_ptr).chunk_name
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_palette_max(
    png_ptr: png_const_structp,
    info_ptr: png_const_infop,
) -> c_int {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*png_ptr).num_palette_max;
    }

    -1
}
