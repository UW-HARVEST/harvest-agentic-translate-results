//! Translation of pngget.c

use crate::*;

unsafe extern "C" {
    fn atof(nptr: *const c_char) -> c_double;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_valid(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    flag: png_uint_32,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null() && info_ptr != core::ptr::null() {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_rowbytes(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> usize {
    unsafe {
        if png_ptr != core::ptr::null() && info_ptr != core::ptr::null() {
            return (*info_ptr).rowbytes;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_rows(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_bytepp {
    unsafe {
        if png_ptr != core::ptr::null() && info_ptr != core::ptr::null() {
            return (*info_ptr).row_pointers;
        }

        core::ptr::null_mut()
    }
}

/* Easy access to info, added in libpng-0.99 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_image_width(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null() && info_ptr != core::ptr::null() {
            return (*info_ptr).width;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_image_height(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null() && info_ptr != core::ptr::null() {
            return (*info_ptr).height;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_bit_depth(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    unsafe {
        if png_ptr != core::ptr::null() && info_ptr != core::ptr::null() {
            return (*info_ptr).bit_depth;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_color_type(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    unsafe {
        if png_ptr != core::ptr::null() && info_ptr != core::ptr::null() {
            return (*info_ptr).color_type;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_filter_type(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    unsafe {
        if png_ptr != core::ptr::null() && info_ptr != core::ptr::null() {
            return (*info_ptr).filter_type;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_interlace_type(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    unsafe {
        if png_ptr != core::ptr::null() && info_ptr != core::ptr::null() {
            return (*info_ptr).interlace_type;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_compression_type(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    unsafe {
        if png_ptr != core::ptr::null() && info_ptr != core::ptr::null() {
            return (*info_ptr).compression_type;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_x_pixels_per_meter(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_pHYs) != 0
        {
            if (*info_ptr).phys_unit_type as c_int == PNG_RESOLUTION_METER {
                return (*info_ptr).x_pixels_per_unit;
            }
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_y_pixels_per_meter(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_pHYs) != 0
        {
            if (*info_ptr).phys_unit_type as c_int == PNG_RESOLUTION_METER {
                return (*info_ptr).y_pixels_per_unit;
            }
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_pixels_per_meter(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_pixel_aspect_ratio(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> f32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_pHYs) != 0
        {
            if (*info_ptr).x_pixels_per_unit != 0 {
                return (*info_ptr).y_pixels_per_unit as f32
                    / (*info_ptr).x_pixels_per_unit as f32;
            }
        }

        0.0f32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_pixel_aspect_ratio_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_fixed_point {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_x_offset_microns(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_int_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_oFFs) != 0
        {
            if (*info_ptr).offset_unit_type as c_int == PNG_OFFSET_MICROMETER {
                return (*info_ptr).x_offset;
            }
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_y_offset_microns(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_int_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_oFFs) != 0
        {
            if (*info_ptr).offset_unit_type as c_int == PNG_OFFSET_MICROMETER {
                return (*info_ptr).y_offset;
            }
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_x_offset_pixels(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_int_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_oFFs) != 0
        {
            if (*info_ptr).offset_unit_type as c_int == PNG_OFFSET_PIXEL {
                return (*info_ptr).x_offset;
            }
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_y_offset_pixels(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_int_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_oFFs) != 0
        {
            if (*info_ptr).offset_unit_type as c_int == PNG_OFFSET_PIXEL {
                return (*info_ptr).y_offset;
            }
        }

        0
    }
}

unsafe fn ppi_from_ppm(ppm: png_uint_32) -> png_uint_32 {
    unsafe {
        /* The argument is a PNG unsigned integer, so it is not permitted
         * to be bigger than 2^31.
         */
        let mut result: png_fixed_point = 0;
        if ppm <= PNG_UINT_31_MAX
            && png_muldiv(&mut result, ppm as png_int_32, 127, 5000) != 0
        {
            return result as png_uint_32;
        }

        /* Overflow. */
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_pixels_per_inch(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    unsafe { ppi_from_ppm(png_get_pixels_per_meter(png_ptr, info_ptr)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_x_pixels_per_inch(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    unsafe { ppi_from_ppm(png_get_x_pixels_per_meter(png_ptr, info_ptr)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_y_pixels_per_inch(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_uint_32 {
    unsafe { ppi_from_ppm(png_get_y_pixels_per_meter(png_ptr, info_ptr)) }
}

unsafe fn png_fixed_inches_from_microns(
    png_ptr: png_const_structrp,
    microns: png_int_32,
) -> png_fixed_point {
    unsafe {
        /* Convert from meters * 1,000,000 to inches * 100,000, meters to
         * inches is simply *(100/2.54), so we want *(10/2.54) == 500/127.
         * Notice that this can overflow - a warning is output and 0 is
         * returned.
         */
        let mut result: png_fixed_point = 0;

        if png_muldiv(&mut result, microns, 500, 127) != 0 {
            return result;
        }

        png_warning(png_ptr, c"fixed point overflow ignored".as_ptr());
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_x_offset_inches_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_fixed_point {
    unsafe {
        png_fixed_inches_from_microns(png_ptr, png_get_x_offset_microns(png_ptr, info_ptr))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_y_offset_inches_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_fixed_point {
    unsafe {
        png_fixed_inches_from_microns(png_ptr, png_get_y_offset_microns(png_ptr, info_ptr))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_x_offset_inches(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> f32 {
    unsafe {
        /* To avoid the overflow do the conversion directly in floating
         * point.
         */
        (png_get_x_offset_microns(png_ptr, info_ptr) as c_double * 0.00003937) as f32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_y_offset_inches(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> f32 {
    unsafe {
        /* To avoid the overflow do the conversion directly in floating
         * point.
         */
        (png_get_y_offset_microns(png_ptr, info_ptr) as c_double * 0.00003937) as f32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_pHYs_dpi(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    res_x: *mut png_uint_32,
    res_y: *mut png_uint_32,
    unit_type: *mut c_int,
) -> png_uint_32 {
    unsafe {
        let mut retval: png_uint_32 = 0;

        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
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
                        *res_x = (*res_x as c_double * 0.0254 + 0.50) as png_uint_32;
                    }
                    if res_y != core::ptr::null_mut() {
                        *res_y = (*res_y as c_double * 0.0254 + 0.50) as png_uint_32;
                    }
                }
            }
        }

        retval
    }
}

/* png_get_channels really belongs in here, too, but it's been around longer */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_channels(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_byte {
    unsafe {
        if png_ptr != core::ptr::null() && info_ptr != core::ptr::null() {
            return (*info_ptr).channels;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_signature(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
) -> png_const_bytep {
    unsafe {
        if png_ptr != core::ptr::null() && info_ptr != core::ptr::null() {
            return (*info_ptr).signature.as_ptr();
        }

        core::ptr::null()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_bKGD(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    background: *mut png_color_16p,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null_mut()
            && ((*info_ptr).valid & PNG_INFO_bKGD) != 0
            && background != core::ptr::null_mut()
        {
            *background = &raw mut (*info_ptr).background;
            return PNG_INFO_bKGD;
        }

        0
    }
}

/* The XYZ APIs were added in 1.5.5 to take advantage of the code added at the
 * same time to correct the rgb grayscale coefficient defaults obtained from the
 * cHRM chunk in 1.5.4
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_cHRM(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    whitex: *mut c_double,
    whitey: *mut c_double,
    redx: *mut c_double,
    redy: *mut c_double,
    greenx: *mut c_double,
    greeny: *mut c_double,
    bluex: *mut c_double,
    bluey: *mut c_double,
) -> png_uint_32 {
    unsafe {
        /* PNGv3: this just returns the values store from the cHRM, if any. */
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_cHRM) != 0
        {
            if whitex != core::ptr::null_mut() {
                *whitex = png_float(png_ptr, (*info_ptr).cHRM.whitex, c"cHRM wx".as_ptr());
            }
            if whitey != core::ptr::null_mut() {
                *whitey = png_float(png_ptr, (*info_ptr).cHRM.whitey, c"cHRM wy".as_ptr());
            }
            if redx != core::ptr::null_mut() {
                *redx = png_float(png_ptr, (*info_ptr).cHRM.redx, c"cHRM rx".as_ptr());
            }
            if redy != core::ptr::null_mut() {
                *redy = png_float(png_ptr, (*info_ptr).cHRM.redy, c"cHRM ry".as_ptr());
            }
            if greenx != core::ptr::null_mut() {
                *greenx = png_float(png_ptr, (*info_ptr).cHRM.greenx, c"cHRM gx".as_ptr());
            }
            if greeny != core::ptr::null_mut() {
                *greeny = png_float(png_ptr, (*info_ptr).cHRM.greeny, c"cHRM gy".as_ptr());
            }
            if bluex != core::ptr::null_mut() {
                *bluex = png_float(png_ptr, (*info_ptr).cHRM.bluex, c"cHRM bx".as_ptr());
            }
            if bluey != core::ptr::null_mut() {
                *bluey = png_float(png_ptr, (*info_ptr).cHRM.bluey, c"cHRM by".as_ptr());
            }
            return PNG_INFO_cHRM;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_cHRM_XYZ(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    red_X: *mut c_double,
    red_Y: *mut c_double,
    red_Z: *mut c_double,
    green_X: *mut c_double,
    green_Y: *mut c_double,
    green_Z: *mut c_double,
    blue_X: *mut c_double,
    blue_Y: *mut c_double,
    blue_Z: *mut c_double,
) -> png_uint_32 {
    unsafe {
        let mut XYZ: png_XYZ = png_XYZ::default();

        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_cHRM) != 0
            && png_XYZ_from_xy(&mut XYZ, &(*info_ptr).cHRM) == 0
        {
            if red_X != core::ptr::null_mut() {
                *red_X = png_float(png_ptr, XYZ.red_X, c"cHRM red X".as_ptr());
            }
            if red_Y != core::ptr::null_mut() {
                *red_Y = png_float(png_ptr, XYZ.red_Y, c"cHRM red Y".as_ptr());
            }
            if red_Z != core::ptr::null_mut() {
                *red_Z = png_float(png_ptr, XYZ.red_Z, c"cHRM red Z".as_ptr());
            }
            if green_X != core::ptr::null_mut() {
                *green_X = png_float(png_ptr, XYZ.green_X, c"cHRM green X".as_ptr());
            }
            if green_Y != core::ptr::null_mut() {
                *green_Y = png_float(png_ptr, XYZ.green_Y, c"cHRM green Y".as_ptr());
            }
            if green_Z != core::ptr::null_mut() {
                *green_Z = png_float(png_ptr, XYZ.green_Z, c"cHRM green Z".as_ptr());
            }
            if blue_X != core::ptr::null_mut() {
                *blue_X = png_float(png_ptr, XYZ.blue_X, c"cHRM blue X".as_ptr());
            }
            if blue_Y != core::ptr::null_mut() {
                *blue_Y = png_float(png_ptr, XYZ.blue_Y, c"cHRM blue Y".as_ptr());
            }
            if blue_Z != core::ptr::null_mut() {
                *blue_Z = png_float(png_ptr, XYZ.blue_Z, c"cHRM blue Z".as_ptr());
            }
            return PNG_INFO_cHRM;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_cHRM_XYZ_fixed(
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
    unsafe {
        let mut XYZ: png_XYZ = png_XYZ::default();

        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_cHRM) != 0u32
            && png_XYZ_from_xy(&mut XYZ, &(*info_ptr).cHRM) == 0
        {
            if int_red_X != core::ptr::null_mut() {
                *int_red_X = XYZ.red_X;
            }
            if int_red_Y != core::ptr::null_mut() {
                *int_red_Y = XYZ.red_Y;
            }
            if int_red_Z != core::ptr::null_mut() {
                *int_red_Z = XYZ.red_Z;
            }
            if int_green_X != core::ptr::null_mut() {
                *int_green_X = XYZ.green_X;
            }
            if int_green_Y != core::ptr::null_mut() {
                *int_green_Y = XYZ.green_Y;
            }
            if int_green_Z != core::ptr::null_mut() {
                *int_green_Z = XYZ.green_Z;
            }
            if int_blue_X != core::ptr::null_mut() {
                *int_blue_X = XYZ.blue_X;
            }
            if int_blue_Y != core::ptr::null_mut() {
                *int_blue_Y = XYZ.blue_Y;
            }
            if int_blue_Z != core::ptr::null_mut() {
                *int_blue_Z = XYZ.blue_Z;
            }
            return PNG_INFO_cHRM;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_cHRM_fixed(
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
    unsafe {
        /* PNGv3: this just returns the values store from the cHRM, if any. */
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_cHRM) != 0
        {
            if whitex != core::ptr::null_mut() {
                *whitex = (*info_ptr).cHRM.whitex;
            }
            if whitey != core::ptr::null_mut() {
                *whitey = (*info_ptr).cHRM.whitey;
            }
            if redx != core::ptr::null_mut() {
                *redx = (*info_ptr).cHRM.redx;
            }
            if redy != core::ptr::null_mut() {
                *redy = (*info_ptr).cHRM.redy;
            }
            if greenx != core::ptr::null_mut() {
                *greenx = (*info_ptr).cHRM.greenx;
            }
            if greeny != core::ptr::null_mut() {
                *greeny = (*info_ptr).cHRM.greeny;
            }
            if bluex != core::ptr::null_mut() {
                *bluex = (*info_ptr).cHRM.bluex;
            }
            if bluey != core::ptr::null_mut() {
                *bluey = (*info_ptr).cHRM.bluey;
            }
            return PNG_INFO_cHRM;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_gAMA_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    file_gamma: *mut png_fixed_point,
) -> png_uint_32 {
    unsafe {
        /* PNGv3 compatibility: only report gAMA if it is really present. */
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_gAMA) != 0
        {
            if file_gamma != core::ptr::null_mut() {
                *file_gamma = (*info_ptr).gamma;
            }
            return PNG_INFO_gAMA;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_gAMA(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    file_gamma: *mut c_double,
) -> png_uint_32 {
    unsafe {
        /* PNGv3 compatibility: only report gAMA if it is really present. */
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_gAMA) != 0
        {
            if file_gamma != core::ptr::null_mut() {
                *file_gamma = png_float(png_ptr, (*info_ptr).gamma, c"gAMA".as_ptr());
            }

            return PNG_INFO_gAMA;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_sRGB(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    file_srgb_intent: *mut c_int,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_sRGB) != 0
        {
            if file_srgb_intent != core::ptr::null_mut() {
                *file_srgb_intent = (*info_ptr).rendering_intent;
            }
            return PNG_INFO_sRGB;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_iCCP(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    name: png_charpp,
    compression_type: *mut c_int,
    profile: png_bytepp,
    proflen: *mut png_uint_32,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null_mut()
            && ((*info_ptr).valid & PNG_INFO_iCCP) != 0
            && name != core::ptr::null_mut()
            && profile != core::ptr::null_mut()
            && proflen != core::ptr::null_mut()
        {
            *name = (*info_ptr).iccp_name;
            *profile = (*info_ptr).iccp_profile;
            *proflen = png_get_uint_32((*info_ptr).iccp_profile);
            /* This is somewhat irrelevant since the profile data returned has
             * actually been uncompressed.
             */
            if compression_type != core::ptr::null_mut() {
                *compression_type = PNG_COMPRESSION_TYPE_BASE;
            }
            return PNG_INFO_iCCP;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_sPLT(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    spalettes: png_sPLT_tpp,
) -> c_int {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null_mut()
            && spalettes != core::ptr::null_mut()
        {
            *spalettes = (*info_ptr).splt_palettes;
            return (*info_ptr).splt_palettes_num;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_cICP(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    colour_primaries: png_bytep,
    transfer_function: png_bytep,
    matrix_coefficients: png_bytep,
    video_full_range_flag: png_bytep,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_cICP) != 0
            && colour_primaries != core::ptr::null_mut()
            && transfer_function != core::ptr::null_mut()
            && matrix_coefficients != core::ptr::null_mut()
            && video_full_range_flag != core::ptr::null_mut()
        {
            *colour_primaries = (*info_ptr).cicp_colour_primaries;
            *transfer_function = (*info_ptr).cicp_transfer_function;
            *matrix_coefficients = (*info_ptr).cicp_matrix_coefficients;
            *video_full_range_flag = (*info_ptr).cicp_video_full_range_flag;
            return PNG_INFO_cICP;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_cLLI_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    maxCLL: png_uint_32p,
    maxFALL: png_uint_32p,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_cLLI) != 0
        {
            if maxCLL != core::ptr::null_mut() {
                *maxCLL = (*info_ptr).maxCLL;
            }
            if maxFALL != core::ptr::null_mut() {
                *maxFALL = (*info_ptr).maxFALL;
            }
            return PNG_INFO_cLLI;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_cLLI(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    maxCLL: *mut c_double,
    maxFALL: *mut c_double,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_cLLI) != 0
        {
            if maxCLL != core::ptr::null_mut() {
                *maxCLL = (*info_ptr).maxCLL as c_double * 0.0001;
            }
            if maxFALL != core::ptr::null_mut() {
                *maxFALL = (*info_ptr).maxFALL as c_double * 0.0001;
            }
            return PNG_INFO_cLLI;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_mDCV_fixed(
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
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_mDCV) != 0
        {
            if white_x != core::ptr::null_mut() {
                *white_x = (*info_ptr).mastering_white_x as png_fixed_point * 2;
            }
            if white_y != core::ptr::null_mut() {
                *white_y = (*info_ptr).mastering_white_y as png_fixed_point * 2;
            }
            if red_x != core::ptr::null_mut() {
                *red_x = (*info_ptr).mastering_red_x as png_fixed_point * 2;
            }
            if red_y != core::ptr::null_mut() {
                *red_y = (*info_ptr).mastering_red_y as png_fixed_point * 2;
            }
            if green_x != core::ptr::null_mut() {
                *green_x = (*info_ptr).mastering_green_x as png_fixed_point * 2;
            }
            if green_y != core::ptr::null_mut() {
                *green_y = (*info_ptr).mastering_green_y as png_fixed_point * 2;
            }
            if blue_x != core::ptr::null_mut() {
                *blue_x = (*info_ptr).mastering_blue_x as png_fixed_point * 2;
            }
            if blue_y != core::ptr::null_mut() {
                *blue_y = (*info_ptr).mastering_blue_y as png_fixed_point * 2;
            }
            if mastering_maxDL != core::ptr::null_mut() {
                *mastering_maxDL = (*info_ptr).mastering_maxDL;
            }
            if mastering_minDL != core::ptr::null_mut() {
                *mastering_minDL = (*info_ptr).mastering_minDL;
            }
            return PNG_INFO_mDCV;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_mDCV(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    white_x: *mut c_double,
    white_y: *mut c_double,
    red_x: *mut c_double,
    red_y: *mut c_double,
    green_x: *mut c_double,
    green_y: *mut c_double,
    blue_x: *mut c_double,
    blue_y: *mut c_double,
    mastering_maxDL: *mut c_double,
    mastering_minDL: *mut c_double,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_mDCV) != 0
        {
            if white_x != core::ptr::null_mut() {
                *white_x = (*info_ptr).mastering_white_x as c_double * 0.00002;
            }
            if white_y != core::ptr::null_mut() {
                *white_y = (*info_ptr).mastering_white_y as c_double * 0.00002;
            }
            if red_x != core::ptr::null_mut() {
                *red_x = (*info_ptr).mastering_red_x as c_double * 0.00002;
            }
            if red_y != core::ptr::null_mut() {
                *red_y = (*info_ptr).mastering_red_y as c_double * 0.00002;
            }
            if green_x != core::ptr::null_mut() {
                *green_x = (*info_ptr).mastering_green_x as c_double * 0.00002;
            }
            if green_y != core::ptr::null_mut() {
                *green_y = (*info_ptr).mastering_green_y as c_double * 0.00002;
            }
            if blue_x != core::ptr::null_mut() {
                *blue_x = (*info_ptr).mastering_blue_x as c_double * 0.00002;
            }
            if blue_y != core::ptr::null_mut() {
                *blue_y = (*info_ptr).mastering_blue_y as c_double * 0.00002;
            }
            if mastering_maxDL != core::ptr::null_mut() {
                *mastering_maxDL = (*info_ptr).mastering_maxDL as c_double * 0.0001;
            }
            if mastering_minDL != core::ptr::null_mut() {
                *mastering_minDL = (*info_ptr).mastering_minDL as c_double * 0.0001;
            }
            return PNG_INFO_mDCV;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_eXIf(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    exif: *mut png_bytep,
) -> png_uint_32 {
    unsafe {
        png_warning(
            png_ptr,
            c"png_get_eXIf does not work; use png_get_eXIf_1".as_ptr(),
        );
        let _ = info_ptr;
        let _ = exif;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_eXIf_1(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    num_exif: *mut png_uint_32,
    exif: *mut png_bytep,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_eXIf) != 0
            && exif != core::ptr::null_mut()
        {
            *num_exif = (*info_ptr).num_exif;
            *exif = (*info_ptr).exif;
            return PNG_INFO_eXIf;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_hIST(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    hist: *mut png_uint_16p,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null_mut()
            && ((*info_ptr).valid & PNG_INFO_hIST) != 0
            && hist != core::ptr::null_mut()
        {
            *hist = (*info_ptr).hist;
            return PNG_INFO_hIST;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_IHDR(
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
    unsafe {
        if png_ptr == core::ptr::null() || info_ptr == core::ptr::null() {
            return 0;
        }

        if width != core::ptr::null_mut() {
            *width = (*info_ptr).width;
        }

        if height != core::ptr::null_mut() {
            *height = (*info_ptr).height;
        }

        if bit_depth != core::ptr::null_mut() {
            *bit_depth = (*info_ptr).bit_depth as c_int;
        }

        if color_type != core::ptr::null_mut() {
            *color_type = (*info_ptr).color_type as c_int;
        }

        if compression_type != core::ptr::null_mut() {
            *compression_type = (*info_ptr).compression_type as c_int;
        }

        if filter_type != core::ptr::null_mut() {
            *filter_type = (*info_ptr).filter_type as c_int;
        }

        if interlace_type != core::ptr::null_mut() {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_oFFs(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    offset_x: *mut png_int_32,
    offset_y: *mut png_int_32,
    unit_type: *mut c_int,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_oFFs) != 0
            && offset_x != core::ptr::null_mut()
            && offset_y != core::ptr::null_mut()
            && unit_type != core::ptr::null_mut()
        {
            *offset_x = (*info_ptr).x_offset;
            *offset_y = (*info_ptr).y_offset;
            *unit_type = (*info_ptr).offset_unit_type as c_int;
            return PNG_INFO_oFFs;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_pCAL(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    purpose: *mut png_charp,
    X0: *mut png_int_32,
    X1: *mut png_int_32,
    r#type: *mut c_int,
    nparams: *mut c_int,
    units: *mut png_charp,
    params: *mut png_charpp,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null_mut()
            && ((*info_ptr).valid & PNG_INFO_pCAL) != 0
            && purpose != core::ptr::null_mut()
            && X0 != core::ptr::null_mut()
            && X1 != core::ptr::null_mut()
            && r#type != core::ptr::null_mut()
            && nparams != core::ptr::null_mut()
            && units != core::ptr::null_mut()
            && params != core::ptr::null_mut()
        {
            *purpose = (*info_ptr).pcal_purpose;
            *X0 = (*info_ptr).pcal_X0;
            *X1 = (*info_ptr).pcal_X1;
            *r#type = (*info_ptr).pcal_type as c_int;
            *nparams = (*info_ptr).pcal_nparams as c_int;
            *units = (*info_ptr).pcal_units;
            *params = (*info_ptr).pcal_params;
            return PNG_INFO_pCAL;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_sCAL_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    unit: *mut c_int,
    width: *mut png_fixed_point,
    height: *mut png_fixed_point,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_sCAL) != 0
        {
            *unit = (*info_ptr).scal_unit as c_int;
            /*TODO: make this work without FP support; the API is currently eliminated
             * if neither floating point APIs nor internal floating point arithmetic
             * are enabled.
             */
            *width = png_fixed(png_ptr, atof((*info_ptr).scal_s_width), c"sCAL width".as_ptr());
            *height = png_fixed(
                png_ptr,
                atof((*info_ptr).scal_s_height),
                c"sCAL height".as_ptr(),
            );
            return PNG_INFO_sCAL;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_sCAL(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    unit: *mut c_int,
    width: *mut c_double,
    height: *mut c_double,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_sCAL) != 0
        {
            *unit = (*info_ptr).scal_unit as c_int;
            *width = atof((*info_ptr).scal_s_width);
            *height = atof((*info_ptr).scal_s_height);
            return PNG_INFO_sCAL;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_sCAL_s(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    unit: *mut c_int,
    width: *mut png_charp,
    height: *mut png_charp,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
            && ((*info_ptr).valid & PNG_INFO_sCAL) != 0
        {
            *unit = (*info_ptr).scal_unit as c_int;
            *width = (*info_ptr).scal_s_width;
            *height = (*info_ptr).scal_s_height;
            return PNG_INFO_sCAL;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_pHYs(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    res_x: *mut png_uint_32,
    res_y: *mut png_uint_32,
    unit_type: *mut c_int,
) -> png_uint_32 {
    unsafe {
        let mut retval: png_uint_32 = 0;

        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null()
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
            }
        }

        retval
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_PLTE(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    palette: *mut png_colorp,
    num_palette: *mut c_int,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null_mut()
            && ((*info_ptr).valid & PNG_INFO_PLTE) != 0
            && palette != core::ptr::null_mut()
        {
            *palette = (*info_ptr).palette;
            *num_palette = (*info_ptr).num_palette as c_int;
            return PNG_INFO_PLTE;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_sBIT(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    sig_bit: *mut png_color_8p,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null_mut()
            && ((*info_ptr).valid & PNG_INFO_sBIT) != 0
            && sig_bit != core::ptr::null_mut()
        {
            *sig_bit = &raw mut (*info_ptr).sig_bit;
            return PNG_INFO_sBIT;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_text(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    text_ptr: *mut png_textp,
    num_text: *mut c_int,
) -> c_int {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null_mut()
            && (*info_ptr).num_text > 0
        {
            if text_ptr != core::ptr::null_mut() {
                *text_ptr = (*info_ptr).text;
            }

            if num_text != core::ptr::null_mut() {
                *num_text = (*info_ptr).num_text;
            }

            return (*info_ptr).num_text;
        }

        if num_text != core::ptr::null_mut() {
            *num_text = 0;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_tIME(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mod_time: *mut png_timep,
) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null_mut()
            && ((*info_ptr).valid & PNG_INFO_tIME) != 0
            && mod_time != core::ptr::null_mut()
        {
            *mod_time = &raw mut (*info_ptr).mod_time;
            return PNG_INFO_tIME;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_tRNS(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    trans_alpha: *mut png_bytep,
    num_trans: *mut c_int,
    trans_color: *mut png_color_16p,
) -> png_uint_32 {
    unsafe {
        let mut retval: png_uint_32 = 0;

        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null_mut()
            && ((*info_ptr).valid & PNG_INFO_tRNS) != 0
        {
            if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
                if trans_alpha != core::ptr::null_mut() {
                    *trans_alpha = (*info_ptr).trans_alpha;
                    retval |= PNG_INFO_tRNS;
                }

                if trans_color != core::ptr::null_mut() {
                    *trans_color = &raw mut (*info_ptr).trans_color;
                }
            } else
            /* if (info_ptr->color_type != PNG_COLOR_TYPE_PALETTE) */
            {
                if trans_color != core::ptr::null_mut() {
                    *trans_color = &raw mut (*info_ptr).trans_color;
                    retval |= PNG_INFO_tRNS;
                }

                if trans_alpha != core::ptr::null_mut() {
                    *trans_alpha = core::ptr::null_mut();
                }
            }

            if num_trans != core::ptr::null_mut() {
                *num_trans = (*info_ptr).num_trans as c_int;
                retval |= PNG_INFO_tRNS;
            }
        }

        retval
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_unknown_chunks(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    unknowns: png_unknown_chunkpp,
) -> c_int {
    unsafe {
        if png_ptr != core::ptr::null()
            && info_ptr != core::ptr::null_mut()
            && unknowns != core::ptr::null_mut()
        {
            *unknowns = (*info_ptr).unknown_chunks;
            return (*info_ptr).unknown_chunks_num;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_rgb_to_gray_status(
    png_ptr: png_const_structrp,
) -> png_byte {
    unsafe {
        (if png_ptr != core::ptr::null() {
            (*png_ptr).rgb_to_gray_status
        } else {
            0
        }) as png_byte
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_user_chunk_ptr(
    png_ptr: png_const_structrp,
) -> png_voidp {
    unsafe {
        if png_ptr != core::ptr::null() {
            (*png_ptr).user_chunk_ptr
        } else {
            core::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_compression_buffer_size(
    png_ptr: png_const_structrp,
) -> usize {
    unsafe {
        if png_ptr == core::ptr::null() {
            return 0;
        }

        if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 {
            (*png_ptr).IDAT_read_size as usize
        } else {
            (*png_ptr).zbuffer_size as usize
        }
    }
}

/* These functions were added to libpng 1.2.6 and were enabled
 * by default in libpng-1.4.0 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_user_width_max(png_ptr: png_const_structrp) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null() {
            (*png_ptr).user_width_max
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_user_height_max(png_ptr: png_const_structrp) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null() {
            (*png_ptr).user_height_max
        } else {
            0
        }
    }
}

/* This function was added to libpng 1.4.0 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_chunk_cache_max(png_ptr: png_const_structrp) -> png_uint_32 {
    unsafe {
        if png_ptr != core::ptr::null() {
            (*png_ptr).user_chunk_cache_max
        } else {
            0
        }
    }
}

/* This function was added to libpng 1.4.1 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_chunk_malloc_max(
    png_ptr: png_const_structrp,
) -> png_alloc_size_t {
    unsafe {
        if png_ptr != core::ptr::null() {
            (*png_ptr).user_chunk_malloc_max
        } else {
            0
        }
    }
}

/* These functions were added to libpng 1.4.0 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_io_state(png_ptr: png_const_structrp) -> png_uint_32 {
    unsafe { (*png_ptr).io_state }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_io_chunk_type(png_ptr: png_const_structrp) -> png_uint_32 {
    unsafe { (*png_ptr).chunk_name }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_palette_max(
    png_ptr: png_const_structp,
    info_ptr: png_const_infop,
) -> c_int {
    unsafe {
        if png_ptr != core::ptr::null() && info_ptr != core::ptr::null() {
            return (*png_ptr).num_palette_max;
        }

        -1
    }
}
