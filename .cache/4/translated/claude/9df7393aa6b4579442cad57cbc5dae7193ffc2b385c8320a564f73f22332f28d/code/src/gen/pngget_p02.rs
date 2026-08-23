/* pngget.c lines 502..936 */

/* png_get_bKGD */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_bKGD(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    background: *mut png_color_16p,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_bKGD) != 0
        && background != core::ptr::null_mut()
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
/* png_get_cHRM */
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
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_cHRM) != 0
    {
        if whitex != core::ptr::null_mut() {
            *whitex = png_float(
                png_ptr,
                (*info_ptr).cHRM.whitex,
                b"cHRM wx\0".as_ptr() as png_const_charp,
            );
        }
        if whitey != core::ptr::null_mut() {
            *whitey = png_float(
                png_ptr,
                (*info_ptr).cHRM.whitey,
                b"cHRM wy\0".as_ptr() as png_const_charp,
            );
        }
        if redx != core::ptr::null_mut() {
            *redx = png_float(
                png_ptr,
                (*info_ptr).cHRM.redx,
                b"cHRM rx\0".as_ptr() as png_const_charp,
            );
        }
        if redy != core::ptr::null_mut() {
            *redy = png_float(
                png_ptr,
                (*info_ptr).cHRM.redy,
                b"cHRM ry\0".as_ptr() as png_const_charp,
            );
        }
        if greenx != core::ptr::null_mut() {
            *greenx = png_float(
                png_ptr,
                (*info_ptr).cHRM.greenx,
                b"cHRM gx\0".as_ptr() as png_const_charp,
            );
        }
        if greeny != core::ptr::null_mut() {
            *greeny = png_float(
                png_ptr,
                (*info_ptr).cHRM.greeny,
                b"cHRM gy\0".as_ptr() as png_const_charp,
            );
        }
        if bluex != core::ptr::null_mut() {
            *bluex = png_float(
                png_ptr,
                (*info_ptr).cHRM.bluex,
                b"cHRM bx\0".as_ptr() as png_const_charp,
            );
        }
        if bluey != core::ptr::null_mut() {
            *bluey = png_float(
                png_ptr,
                (*info_ptr).cHRM.bluey,
                b"cHRM by\0".as_ptr() as png_const_charp,
            );
        }
        return PNG_INFO_cHRM;
    }

    0
}

/* png_get_cHRM_XYZ */
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
    let mut XYZ: png_XYZ = Default::default();

    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_cHRM) != 0
        && png_XYZ_from_xy(
            core::ptr::addr_of_mut!(XYZ),
            core::ptr::addr_of!((*info_ptr).cHRM),
        ) == 0
    {
        if red_X != core::ptr::null_mut() {
            *red_X = png_float(
                png_ptr,
                XYZ.red_X,
                b"cHRM red X\0".as_ptr() as png_const_charp,
            );
        }
        if red_Y != core::ptr::null_mut() {
            *red_Y = png_float(
                png_ptr,
                XYZ.red_Y,
                b"cHRM red Y\0".as_ptr() as png_const_charp,
            );
        }
        if red_Z != core::ptr::null_mut() {
            *red_Z = png_float(
                png_ptr,
                XYZ.red_Z,
                b"cHRM red Z\0".as_ptr() as png_const_charp,
            );
        }
        if green_X != core::ptr::null_mut() {
            *green_X = png_float(
                png_ptr,
                XYZ.green_X,
                b"cHRM green X\0".as_ptr() as png_const_charp,
            );
        }
        if green_Y != core::ptr::null_mut() {
            *green_Y = png_float(
                png_ptr,
                XYZ.green_Y,
                b"cHRM green Y\0".as_ptr() as png_const_charp,
            );
        }
        if green_Z != core::ptr::null_mut() {
            *green_Z = png_float(
                png_ptr,
                XYZ.green_Z,
                b"cHRM green Z\0".as_ptr() as png_const_charp,
            );
        }
        if blue_X != core::ptr::null_mut() {
            *blue_X = png_float(
                png_ptr,
                XYZ.blue_X,
                b"cHRM blue X\0".as_ptr() as png_const_charp,
            );
        }
        if blue_Y != core::ptr::null_mut() {
            *blue_Y = png_float(
                png_ptr,
                XYZ.blue_Y,
                b"cHRM blue Y\0".as_ptr() as png_const_charp,
            );
        }
        if blue_Z != core::ptr::null_mut() {
            *blue_Z = png_float(
                png_ptr,
                XYZ.blue_Z,
                b"cHRM blue Z\0".as_ptr() as png_const_charp,
            );
        }
        return PNG_INFO_cHRM;
    }

    0
}

/* png_get_cHRM_XYZ_fixed */
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
    let mut XYZ: png_XYZ = Default::default();

    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_cHRM) != 0
        && png_XYZ_from_xy(
            core::ptr::addr_of_mut!(XYZ),
            core::ptr::addr_of!((*info_ptr).cHRM),
        ) == 0
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

/* png_get_cHRM_fixed */
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
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
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

/* png_get_gAMA_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_gAMA_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    file_gamma: *mut png_fixed_point,
) -> png_uint_32 {
    /* PNGv3 compatibility: only report gAMA if it is really present. */
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_gAMA) != 0
    {
        if file_gamma != core::ptr::null_mut() {
            *file_gamma = (*info_ptr).gamma;
        }
        return PNG_INFO_gAMA;
    }

    0
}

/* png_get_gAMA */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_gAMA(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    file_gamma: *mut f64,
) -> png_uint_32 {
    /* PNGv3 compatibility: only report gAMA if it is really present. */
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_gAMA) != 0
    {
        if file_gamma != core::ptr::null_mut() {
            *file_gamma = png_float(
                png_ptr,
                (*info_ptr).gamma,
                b"gAMA\0".as_ptr() as png_const_charp,
            );
        }

        return PNG_INFO_gAMA;
    }

    0
}

/* png_get_sRGB */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sRGB(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    file_srgb_intent: *mut c_int,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_sRGB) != 0
    {
        if file_srgb_intent != core::ptr::null_mut() {
            *file_srgb_intent = (*info_ptr).rendering_intent;
        }
        return PNG_INFO_sRGB;
    }

    0
}

/* png_get_iCCP */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_iCCP(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    name: png_charpp,
    compression_type: *mut c_int,
    profile: png_bytepp,
    proflen: *mut png_uint_32,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_iCCP) != 0
        && name != core::ptr::null_mut()
        && profile != core::ptr::null_mut()
        && proflen != core::ptr::null_mut()
    {
        *name = (*info_ptr).iccp_name;
        *profile = (*info_ptr).iccp_profile;
        *proflen = PNG_get_uint_32((*info_ptr).iccp_profile as png_const_bytep);
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

/* png_get_sPLT */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sPLT(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    spalettes: png_sPLT_tpp,
) -> c_int {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && spalettes != core::ptr::null_mut()
    {
        *spalettes = (*info_ptr).splt_palettes;
        return (*info_ptr).splt_palettes_num;
    }

    0
}

/* png_get_cICP */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cICP(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    colour_primaries: png_bytep,
    transfer_function: png_bytep,
    matrix_coefficients: png_bytep,
    video_full_range_flag: png_bytep,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
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

/* png_get_cLLI_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cLLI_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    maxCLL: png_uint_32p,
    maxFALL: png_uint_32p,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
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

/* png_get_cLLI */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cLLI(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    maxCLL: *mut f64,
    maxFALL: *mut f64,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_cLLI) != 0
    {
        if maxCLL != core::ptr::null_mut() {
            *maxCLL = (*info_ptr).maxCLL as f64 * 0.0001;
        }
        if maxFALL != core::ptr::null_mut() {
            *maxFALL = (*info_ptr).maxFALL as f64 * 0.0001;
        }
        return PNG_INFO_cLLI;
    }

    0
}

/* png_get_mDCV_fixed */
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
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_mDCV) != 0
    {
        if white_x != core::ptr::null_mut() {
            *white_x = ((*info_ptr).mastering_white_x as c_int * 2) as png_fixed_point;
        }
        if white_y != core::ptr::null_mut() {
            *white_y = ((*info_ptr).mastering_white_y as c_int * 2) as png_fixed_point;
        }
        if red_x != core::ptr::null_mut() {
            *red_x = ((*info_ptr).mastering_red_x as c_int * 2) as png_fixed_point;
        }
        if red_y != core::ptr::null_mut() {
            *red_y = ((*info_ptr).mastering_red_y as c_int * 2) as png_fixed_point;
        }
        if green_x != core::ptr::null_mut() {
            *green_x = ((*info_ptr).mastering_green_x as c_int * 2) as png_fixed_point;
        }
        if green_y != core::ptr::null_mut() {
            *green_y = ((*info_ptr).mastering_green_y as c_int * 2) as png_fixed_point;
        }
        if blue_x != core::ptr::null_mut() {
            *blue_x = ((*info_ptr).mastering_blue_x as c_int * 2) as png_fixed_point;
        }
        if blue_y != core::ptr::null_mut() {
            *blue_y = ((*info_ptr).mastering_blue_y as c_int * 2) as png_fixed_point;
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

/* png_get_mDCV */
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
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_mDCV) != 0
    {
        if white_x != core::ptr::null_mut() {
            *white_x = (*info_ptr).mastering_white_x as f64 * 0.00002;
        }
        if white_y != core::ptr::null_mut() {
            *white_y = (*info_ptr).mastering_white_y as f64 * 0.00002;
        }
        if red_x != core::ptr::null_mut() {
            *red_x = (*info_ptr).mastering_red_x as f64 * 0.00002;
        }
        if red_y != core::ptr::null_mut() {
            *red_y = (*info_ptr).mastering_red_y as f64 * 0.00002;
        }
        if green_x != core::ptr::null_mut() {
            *green_x = (*info_ptr).mastering_green_x as f64 * 0.00002;
        }
        if green_y != core::ptr::null_mut() {
            *green_y = (*info_ptr).mastering_green_y as f64 * 0.00002;
        }
        if blue_x != core::ptr::null_mut() {
            *blue_x = (*info_ptr).mastering_blue_x as f64 * 0.00002;
        }
        if blue_y != core::ptr::null_mut() {
            *blue_y = (*info_ptr).mastering_blue_y as f64 * 0.00002;
        }
        if mastering_maxDL != core::ptr::null_mut() {
            *mastering_maxDL = (*info_ptr).mastering_maxDL as f64 * 0.0001;
        }
        if mastering_minDL != core::ptr::null_mut() {
            *mastering_minDL = (*info_ptr).mastering_minDL as f64 * 0.0001;
        }
        return PNG_INFO_mDCV;
    }

    0
}

/* png_get_eXIf */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_eXIf(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    exif: *mut png_bytep,
) -> png_uint_32 {
    png_warning(
        png_ptr,
        b"png_get_eXIf does not work; use png_get_eXIf_1\0".as_ptr() as png_const_charp,
    );
    0
}

/* png_get_eXIf_1 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_eXIf_1(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    num_exif: *mut png_uint_32,
    exif: *mut png_bytep,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_eXIf) != 0
        && exif != core::ptr::null_mut()
    {
        *num_exif = (*info_ptr).num_exif;
        *exif = (*info_ptr).exif;
        return PNG_INFO_eXIf;
    }

    0
}

/* png_get_hIST */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_hIST(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    hist: *mut png_uint_16p,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_hIST) != 0
        && hist != core::ptr::null_mut()
    {
        *hist = (*info_ptr).hist;
        return PNG_INFO_hIST;
    }

    0
}
