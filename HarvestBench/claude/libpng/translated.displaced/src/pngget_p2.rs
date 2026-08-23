use crate::*;

/* png_get_channels really belongs in here, too, but it's been around longer */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_channels(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_byte {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).channels;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_signature(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_const_bytep {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return core::ptr::addr_of!((*info_ptr).signature) as png_const_bytep;
    }

    core::ptr::null()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_bKGD(png_ptr: png_const_structrp, info_ptr: png_inforp, background: *mut png_color_16p) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_bKGD) != 0
        && !background.is_null()
    {
        *background = core::ptr::addr_of_mut!((*info_ptr).background);
        return PNG_INFO_bKGD;
    }

    0
}

/* The XYZ APIs were added in 1.5.5 to take advantage of the code added at the
 * same time to correct the rgb grayscale coefficient defaults obtained from the
 * cHRM chunk in 1.5.4
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cHRM(png_ptr: png_const_structrp, info_ptr: png_const_inforp, white_x: *mut f64, white_y: *mut f64, red_x: *mut f64, red_y: *mut f64, green_x: *mut f64, green_y: *mut f64, blue_x: *mut f64, blue_y: *mut f64) -> png_uint_32 {
    /* PNGv3: this just returns the values store from the cHRM, if any. */
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_cHRM) != 0 {
        if !white_x.is_null() {
            *white_x = png_float_of((*info_ptr).cHRM.whitex);
        }
        if !white_y.is_null() {
            *white_y = png_float_of((*info_ptr).cHRM.whitey);
        }
        if !red_x.is_null() {
            *red_x = png_float_of((*info_ptr).cHRM.redx);
        }
        if !red_y.is_null() {
            *red_y = png_float_of((*info_ptr).cHRM.redy);
        }
        if !green_x.is_null() {
            *green_x = png_float_of((*info_ptr).cHRM.greenx);
        }
        if !green_y.is_null() {
            *green_y = png_float_of((*info_ptr).cHRM.greeny);
        }
        if !blue_x.is_null() {
            *blue_x = png_float_of((*info_ptr).cHRM.bluex);
        }
        if !blue_y.is_null() {
            *blue_y = png_float_of((*info_ptr).cHRM.bluey);
        }
        return PNG_INFO_cHRM;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cHRM_XYZ(png_ptr: png_const_structrp, info_ptr: png_const_inforp, red_X: *mut f64, red_Y: *mut f64, red_Z: *mut f64, green_X: *mut f64, green_Y: *mut f64, green_Z: *mut f64, blue_X: *mut f64, blue_Y: *mut f64, blue_Z: *mut f64) -> png_uint_32 {
    let mut XYZ: png_XYZ = core::mem::zeroed();

    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_cHRM) != 0
        && png_XYZ_from_xy(&mut XYZ as *mut png_XYZ, core::ptr::addr_of!((*info_ptr).cHRM)) == 0
    {
        if !red_X.is_null() {
            *red_X = png_float_of(XYZ.red_X);
        }
        if !red_Y.is_null() {
            *red_Y = png_float_of(XYZ.red_Y);
        }
        if !red_Z.is_null() {
            *red_Z = png_float_of(XYZ.red_Z);
        }
        if !green_X.is_null() {
            *green_X = png_float_of(XYZ.green_X);
        }
        if !green_Y.is_null() {
            *green_Y = png_float_of(XYZ.green_Y);
        }
        if !green_Z.is_null() {
            *green_Z = png_float_of(XYZ.green_Z);
        }
        if !blue_X.is_null() {
            *blue_X = png_float_of(XYZ.blue_X);
        }
        if !blue_Y.is_null() {
            *blue_Y = png_float_of(XYZ.blue_Y);
        }
        if !blue_Z.is_null() {
            *blue_Z = png_float_of(XYZ.blue_Z);
        }
        return PNG_INFO_cHRM;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cHRM_XYZ_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp, int_red_X: *mut png_fixed_point, int_red_Y: *mut png_fixed_point, int_red_Z: *mut png_fixed_point, int_green_X: *mut png_fixed_point, int_green_Y: *mut png_fixed_point, int_green_Z: *mut png_fixed_point, int_blue_X: *mut png_fixed_point, int_blue_Y: *mut png_fixed_point, int_blue_Z: *mut png_fixed_point) -> png_uint_32 {
    let mut XYZ: png_XYZ = core::mem::zeroed();

    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_cHRM) != 0u32
        && png_XYZ_from_xy(&mut XYZ as *mut png_XYZ, core::ptr::addr_of!((*info_ptr).cHRM)) == 0
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
pub unsafe extern "C" fn png_get_cHRM_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp, int_white_x: *mut png_fixed_point, int_white_y: *mut png_fixed_point, int_red_x: *mut png_fixed_point, int_red_y: *mut png_fixed_point, int_green_x: *mut png_fixed_point, int_green_y: *mut png_fixed_point, int_blue_x: *mut png_fixed_point, int_blue_y: *mut png_fixed_point) -> png_uint_32 {
    /* PNGv3: this just returns the values store from the cHRM, if any. */
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_cHRM) != 0 {
        if !int_white_x.is_null() {
            *int_white_x = (*info_ptr).cHRM.whitex;
        }
        if !int_white_y.is_null() {
            *int_white_y = (*info_ptr).cHRM.whitey;
        }
        if !int_red_x.is_null() {
            *int_red_x = (*info_ptr).cHRM.redx;
        }
        if !int_red_y.is_null() {
            *int_red_y = (*info_ptr).cHRM.redy;
        }
        if !int_green_x.is_null() {
            *int_green_x = (*info_ptr).cHRM.greenx;
        }
        if !int_green_y.is_null() {
            *int_green_y = (*info_ptr).cHRM.greeny;
        }
        if !int_blue_x.is_null() {
            *int_blue_x = (*info_ptr).cHRM.bluex;
        }
        if !int_blue_y.is_null() {
            *int_blue_y = (*info_ptr).cHRM.bluey;
        }
        return PNG_INFO_cHRM;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_gAMA_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp, int_file_gamma: *mut png_fixed_point) -> png_uint_32 {
    /* PNGv3 compatibility: only report gAMA if it is really present. */
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_gAMA) != 0 {
        if !int_file_gamma.is_null() {
            *int_file_gamma = (*info_ptr).gamma;
        }
        return PNG_INFO_gAMA;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_gAMA(png_ptr: png_const_structrp, info_ptr: png_const_inforp, file_gamma: *mut f64) -> png_uint_32 {
    /* PNGv3 compatibility: only report gAMA if it is really present. */
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_gAMA) != 0 {
        if !file_gamma.is_null() {
            *file_gamma = png_float_of((*info_ptr).gamma);
        }

        return PNG_INFO_gAMA;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sRGB(png_ptr: png_const_structrp, info_ptr: png_const_inforp, file_srgb_intent: *mut c_int) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_sRGB) != 0 {
        if !file_srgb_intent.is_null() {
            *file_srgb_intent = (*info_ptr).rendering_intent;
        }
        return PNG_INFO_sRGB;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_iCCP(png_ptr: png_const_structrp, info_ptr: png_inforp, name: png_charpp, compression_type: *mut c_int, profile: png_bytepp, proflen: *mut png_uint_32) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_iCCP) != 0
        && !name.is_null()
        && !profile.is_null()
        && !proflen.is_null()
    {
        *name = (*info_ptr).iccp_name;
        *profile = (*info_ptr).iccp_profile;
        *proflen = png_get_uint_32((*info_ptr).iccp_profile as png_const_bytep);
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
pub unsafe extern "C" fn png_get_sPLT(png_ptr: png_const_structrp, info_ptr: png_inforp, entries: png_sPLT_tpp) -> c_int {
    if !png_ptr.is_null() && !info_ptr.is_null() && !entries.is_null() {
        *entries = (*info_ptr).splt_palettes;
        return (*info_ptr).splt_palettes_num;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cICP(png_ptr: png_const_structrp, info_ptr: png_const_inforp, colour_primaries: png_bytep, transfer_function: png_bytep, matrix_coefficients: png_bytep, video_full_range_flag: png_bytep) -> png_uint_32 {
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
pub unsafe extern "C" fn png_get_cLLI_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp, maximum_content_light_level_scaled_by_10000: png_uint_32p, maximum_frame_average_light_level_scaled_by_10000: png_uint_32p) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_cLLI) != 0 {
        if !maximum_content_light_level_scaled_by_10000.is_null() {
            *maximum_content_light_level_scaled_by_10000 = (*info_ptr).maxCLL;
        }
        if !maximum_frame_average_light_level_scaled_by_10000.is_null() {
            *maximum_frame_average_light_level_scaled_by_10000 = (*info_ptr).maxFALL;
        }
        return PNG_INFO_cLLI;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cLLI(png_ptr: png_const_structrp, info_ptr: png_const_inforp, maximum_content_light_level: *mut f64, maximum_frame_average_light_level: *mut f64) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_cLLI) != 0 {
        if !maximum_content_light_level.is_null() {
            *maximum_content_light_level = (*info_ptr).maxCLL as f64 * 0.0001;
        }
        if !maximum_frame_average_light_level.is_null() {
            *maximum_frame_average_light_level = (*info_ptr).maxFALL as f64 * 0.0001;
        }
        return PNG_INFO_cLLI;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_mDCV_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp, int_white_x: *mut png_fixed_point, int_white_y: *mut png_fixed_point, int_red_x: *mut png_fixed_point, int_red_y: *mut png_fixed_point, int_green_x: *mut png_fixed_point, int_green_y: *mut png_fixed_point, int_blue_x: *mut png_fixed_point, int_blue_y: *mut png_fixed_point, mastering_display_maximum_luminance_scaled_by_10000: png_uint_32p, mastering_display_minimum_luminance_scaled_by_10000: png_uint_32p) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_mDCV) != 0 {
        if !int_white_x.is_null() {
            *int_white_x = (*info_ptr).mastering_white_x as c_int * 2;
        }
        if !int_white_y.is_null() {
            *int_white_y = (*info_ptr).mastering_white_y as c_int * 2;
        }
        if !int_red_x.is_null() {
            *int_red_x = (*info_ptr).mastering_red_x as c_int * 2;
        }
        if !int_red_y.is_null() {
            *int_red_y = (*info_ptr).mastering_red_y as c_int * 2;
        }
        if !int_green_x.is_null() {
            *int_green_x = (*info_ptr).mastering_green_x as c_int * 2;
        }
        if !int_green_y.is_null() {
            *int_green_y = (*info_ptr).mastering_green_y as c_int * 2;
        }
        if !int_blue_x.is_null() {
            *int_blue_x = (*info_ptr).mastering_blue_x as c_int * 2;
        }
        if !int_blue_y.is_null() {
            *int_blue_y = (*info_ptr).mastering_blue_y as c_int * 2;
        }
        if !mastering_display_maximum_luminance_scaled_by_10000.is_null() {
            *mastering_display_maximum_luminance_scaled_by_10000 = (*info_ptr).mastering_maxDL;
        }
        if !mastering_display_minimum_luminance_scaled_by_10000.is_null() {
            *mastering_display_minimum_luminance_scaled_by_10000 = (*info_ptr).mastering_minDL;
        }
        return PNG_INFO_mDCV;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_mDCV(png_ptr: png_const_structrp, info_ptr: png_const_inforp, white_x: *mut f64, white_y: *mut f64, red_x: *mut f64, red_y: *mut f64, green_x: *mut f64, green_y: *mut f64, blue_x: *mut f64, blue_y: *mut f64, mastering_display_maximum_luminance: *mut f64, mastering_display_minimum_luminance: *mut f64) -> png_uint_32 {
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
        if !mastering_display_maximum_luminance.is_null() {
            *mastering_display_maximum_luminance = (*info_ptr).mastering_maxDL as f64 * 0.0001;
        }
        if !mastering_display_minimum_luminance.is_null() {
            *mastering_display_minimum_luminance = (*info_ptr).mastering_minDL as f64 * 0.0001;
        }
        return PNG_INFO_mDCV;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_eXIf(png_ptr: png_const_structrp, info_ptr: png_inforp, exif: *mut png_bytep) -> png_uint_32 {
    png_warning(png_ptr, cstr!("png_get_eXIf does not work; use png_get_eXIf_1"));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_eXIf_1(png_ptr: png_const_structrp, info_ptr: png_const_inforp, num_exif: *mut png_uint_32, exif: *mut png_bytep) -> png_uint_32 {
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
pub unsafe extern "C" fn png_get_hIST(png_ptr: png_const_structrp, info_ptr: png_inforp, hist: *mut png_uint_16p) -> png_uint_32 {
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
pub unsafe extern "C" fn png_get_IHDR(png_ptr: png_const_structrp, info_ptr: png_const_inforp, width: *mut png_uint_32, height: *mut png_uint_32, bit_depth: *mut c_int, color_type: *mut c_int, interlace_method: *mut c_int, compression_method: *mut c_int, filter_method: *mut c_int) -> png_uint_32 {
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

    if !compression_method.is_null() {
        *compression_method = (*info_ptr).compression_type as c_int;
    }

    if !filter_method.is_null() {
        *filter_method = (*info_ptr).filter_type as c_int;
    }

    if !interlace_method.is_null() {
        *interlace_method = (*info_ptr).interlace_type as c_int;
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
pub unsafe extern "C" fn png_get_oFFs(png_ptr: png_const_structrp, info_ptr: png_const_inforp, offset_x: *mut png_int_32, offset_y: *mut png_int_32, unit_type: *mut c_int) -> png_uint_32 {
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
pub unsafe extern "C" fn png_get_pCAL(png_ptr: png_const_structrp, info_ptr: png_inforp, purpose: *mut png_charp, X0: *mut png_int_32, X1: *mut png_int_32, type_: *mut c_int, nparams: *mut c_int, units: *mut png_charp, params: *mut png_charpp) -> png_uint_32 {
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
pub unsafe extern "C" fn png_get_sCAL_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp, unit: *mut c_int, width: *mut png_fixed_point, height: *mut png_fixed_point) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_sCAL) != 0 {
        *unit = (*info_ptr).scal_unit as c_int;
        /*TODO: make this work without FP support; the API is currently eliminated
         * if neither floating point APIs nor internal floating point arithmetic
         * are enabled.
         */
        *width = png_fixed(
            png_ptr,
            strtod((*info_ptr).scal_s_width as *const c_char, core::ptr::null_mut()),
            cstr!("sCAL width"),
        );
        *height = png_fixed(
            png_ptr,
            strtod((*info_ptr).scal_s_height as *const c_char, core::ptr::null_mut()),
            cstr!("sCAL height"),
        );
        return PNG_INFO_sCAL;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sCAL(png_ptr: png_const_structrp, info_ptr: png_const_inforp, unit: *mut c_int, width: *mut f64, height: *mut f64) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_sCAL) != 0 {
        *unit = (*info_ptr).scal_unit as c_int;
        *width = strtod((*info_ptr).scal_s_width as *const c_char, core::ptr::null_mut());
        *height = strtod((*info_ptr).scal_s_height as *const c_char, core::ptr::null_mut());
        return PNG_INFO_sCAL;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sCAL_s(png_ptr: png_const_structrp, info_ptr: png_const_inforp, unit: *mut c_int, swidth: png_charpp, sheight: png_charpp) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_sCAL) != 0 {
        *unit = (*info_ptr).scal_unit as c_int;
        *swidth = (*info_ptr).scal_s_width;
        *sheight = (*info_ptr).scal_s_height;
        return PNG_INFO_sCAL;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pHYs(png_ptr: png_const_structrp, info_ptr: png_const_inforp, res_x: *mut png_uint_32, res_y: *mut png_uint_32, unit_type: *mut c_int) -> png_uint_32 {
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
pub unsafe extern "C" fn png_get_PLTE(png_ptr: png_const_structrp, info_ptr: png_inforp, palette: *mut png_colorp, num_palette: *mut c_int) -> png_uint_32 {
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
pub unsafe extern "C" fn png_get_sBIT(png_ptr: png_const_structrp, info_ptr: png_inforp, sig_bit: *mut png_color_8p) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_sBIT) != 0
        && !sig_bit.is_null()
    {
        *sig_bit = core::ptr::addr_of_mut!((*info_ptr).sig_bit);
        return PNG_INFO_sBIT;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_text(png_ptr: png_const_structrp, info_ptr: png_inforp, text_ptr: *mut png_textp, num_text: *mut c_int) -> c_int {
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
pub unsafe extern "C" fn png_get_tIME(png_ptr: png_const_structrp, info_ptr: png_inforp, mod_time: *mut png_timep) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && ((*info_ptr).valid & PNG_INFO_tIME) != 0
        && !mod_time.is_null()
    {
        *mod_time = core::ptr::addr_of_mut!((*info_ptr).mod_time);
        return PNG_INFO_tIME;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_tRNS(png_ptr: png_const_structrp, info_ptr: png_inforp, trans_alpha: *mut png_bytep, num_trans: *mut c_int, trans_color: *mut png_color_16p) -> png_uint_32 {
    let mut retval: png_uint_32 = 0;

    if !png_ptr.is_null() && !info_ptr.is_null() && ((*info_ptr).valid & PNG_INFO_tRNS) != 0 {
        if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            if !trans_alpha.is_null() {
                *trans_alpha = (*info_ptr).trans_alpha;
                retval |= PNG_INFO_tRNS;
            }

            if !trans_color.is_null() {
                *trans_color = core::ptr::addr_of_mut!((*info_ptr).trans_color);
            }
        } else
        /* if (info_ptr->color_type != PNG_COLOR_TYPE_PALETTE) */
        {
            if !trans_color.is_null() {
                *trans_color = core::ptr::addr_of_mut!((*info_ptr).trans_color);
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
pub unsafe extern "C" fn png_get_unknown_chunks(png_ptr: png_const_structrp, info_ptr: png_inforp, entries: png_unknown_chunkpp) -> c_int {
    if !png_ptr.is_null() && !info_ptr.is_null() && !entries.is_null() {
        *entries = (*info_ptr).unknown_chunks;
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

/* These functions were added to libpng 1.2.6 and were enabled
 * by default in libpng-1.4.0 */
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

/* This function was added to libpng 1.4.0 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_chunk_cache_max(png_ptr: png_const_structrp) -> png_uint_32 {
    if !png_ptr.is_null() {
        (*png_ptr).user_chunk_cache_max
    } else {
        0
    }
}

/* This function was added to libpng 1.4.1 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_chunk_malloc_max(png_ptr: png_const_structrp) -> png_alloc_size_t {
    if !png_ptr.is_null() {
        (*png_ptr).user_chunk_malloc_max
    } else {
        0
    }
}

/* These functions were added to libpng 1.4.0 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_io_state(png_ptr: png_const_structrp) -> png_uint_32 {
    (*png_ptr).io_state
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_io_chunk_type(png_ptr: png_const_structrp) -> png_uint_32 {
    (*png_ptr).chunk_name
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_palette_max(png_ptr: png_const_structp, info_ptr: png_const_infop) -> c_int {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*png_ptr).num_palette_max;
    }

    -1
}
