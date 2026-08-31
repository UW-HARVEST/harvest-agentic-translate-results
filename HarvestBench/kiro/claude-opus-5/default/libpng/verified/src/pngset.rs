//! Translation of pngset.c

use crate::*;

/* png_set_bKGD */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_bKGD(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    background: png_const_color_16p,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() || background.is_null() {
            return;
        }

        (*info_ptr).background = *background;
        (*info_ptr).valid |= PNG_INFO_bKGD;
    }
}

/* png_set_cHRM_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_cHRM_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    white_x: png_fixed_point,
    white_y: png_fixed_point,
    red_x: png_fixed_point,
    red_y: png_fixed_point,
    green_x: png_fixed_point,
    green_y: png_fixed_point,
    blue_x: png_fixed_point,
    blue_y: png_fixed_point,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        (*info_ptr).cHRM.redx = red_x;
        (*info_ptr).cHRM.redy = red_y;
        (*info_ptr).cHRM.greenx = green_x;
        (*info_ptr).cHRM.greeny = green_y;
        (*info_ptr).cHRM.bluex = blue_x;
        (*info_ptr).cHRM.bluey = blue_y;
        (*info_ptr).cHRM.whitex = white_x;
        (*info_ptr).cHRM.whitey = white_y;

        (*info_ptr).valid |= PNG_INFO_cHRM;
    }
}

/* png_set_cHRM_XYZ_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_cHRM_XYZ_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    int_red_X: png_fixed_point,
    int_red_Y: png_fixed_point,
    int_red_Z: png_fixed_point,
    int_green_X: png_fixed_point,
    int_green_Y: png_fixed_point,
    int_green_Z: png_fixed_point,
    int_blue_X: png_fixed_point,
    int_blue_Y: png_fixed_point,
    int_blue_Z: png_fixed_point,
) {
    unsafe {
        let mut XYZ: png_XYZ = png_XYZ::default();
        let mut xy: png_xy = png_xy::default();

        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        XYZ.red_X = int_red_X;
        XYZ.red_Y = int_red_Y;
        XYZ.red_Z = int_red_Z;
        XYZ.green_X = int_green_X;
        XYZ.green_Y = int_green_Y;
        XYZ.green_Z = int_green_Z;
        XYZ.blue_X = int_blue_X;
        XYZ.blue_Y = int_blue_Y;
        XYZ.blue_Z = int_blue_Z;

        if png_xy_from_XYZ(&mut xy, &XYZ) == 0 {
            (*info_ptr).cHRM = xy;
            (*info_ptr).valid |= PNG_INFO_cHRM;
        } else {
            png_app_error(png_ptr, c"invalid cHRM XYZ".as_ptr());
        }
    }
}

/* png_set_cHRM */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_cHRM(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    white_x: f64,
    white_y: f64,
    red_x: f64,
    red_y: f64,
    green_x: f64,
    green_y: f64,
    blue_x: f64,
    blue_y: f64,
) {
    unsafe {
        png_set_cHRM_fixed(
            png_ptr,
            info_ptr,
            png_fixed(png_ptr, white_x, c"cHRM White X".as_ptr()),
            png_fixed(png_ptr, white_y, c"cHRM White Y".as_ptr()),
            png_fixed(png_ptr, red_x, c"cHRM Red X".as_ptr()),
            png_fixed(png_ptr, red_y, c"cHRM Red Y".as_ptr()),
            png_fixed(png_ptr, green_x, c"cHRM Green X".as_ptr()),
            png_fixed(png_ptr, green_y, c"cHRM Green Y".as_ptr()),
            png_fixed(png_ptr, blue_x, c"cHRM Blue X".as_ptr()),
            png_fixed(png_ptr, blue_y, c"cHRM Blue Y".as_ptr()),
        );
    }
}

/* png_set_cHRM_XYZ */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_cHRM_XYZ(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    red_X: f64,
    red_Y: f64,
    red_Z: f64,
    green_X: f64,
    green_Y: f64,
    green_Z: f64,
    blue_X: f64,
    blue_Y: f64,
    blue_Z: f64,
) {
    unsafe {
        png_set_cHRM_XYZ_fixed(
            png_ptr,
            info_ptr,
            png_fixed(png_ptr, red_X, c"cHRM Red X".as_ptr()),
            png_fixed(png_ptr, red_Y, c"cHRM Red Y".as_ptr()),
            png_fixed(png_ptr, red_Z, c"cHRM Red Z".as_ptr()),
            png_fixed(png_ptr, green_X, c"cHRM Green X".as_ptr()),
            png_fixed(png_ptr, green_Y, c"cHRM Green Y".as_ptr()),
            png_fixed(png_ptr, green_Z, c"cHRM Green Z".as_ptr()),
            png_fixed(png_ptr, blue_X, c"cHRM Blue X".as_ptr()),
            png_fixed(png_ptr, blue_Y, c"cHRM Blue Y".as_ptr()),
            png_fixed(png_ptr, blue_Z, c"cHRM Blue Z".as_ptr()),
        );
    }
}

/* png_set_cICP */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_cICP(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    colour_primaries: png_byte,
    transfer_function: png_byte,
    matrix_coefficients: png_byte,
    video_full_range_flag: png_byte,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        (*info_ptr).cicp_colour_primaries = colour_primaries;
        (*info_ptr).cicp_transfer_function = transfer_function;
        (*info_ptr).cicp_matrix_coefficients = matrix_coefficients;
        (*info_ptr).cicp_video_full_range_flag = video_full_range_flag;

        if (*info_ptr).cicp_matrix_coefficients != 0 {
            png_warning(png_ptr, c"Invalid cICP matrix coefficients".as_ptr());
            return;
        }

        (*info_ptr).valid |= PNG_INFO_cICP;
    }
}

/* png_set_cLLI_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_cLLI_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    /* The values below are in cd/m2 (nits) and are scaled by 10,000; not
     * 100,000 as in the case of png_fixed_point.
     */
    maxCLL: png_uint_32,
    maxFALL: png_uint_32,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        /* Check the light level range: */
        if maxCLL > 0x7FFFFFFFu32 || maxFALL > 0x7FFFFFFFu32 {
            /* The limit is 200kcd/m2; somewhat bright but not inconceivable because
             * human vision is said to run up to 100Mcd/m2.  The sun is about 2Gcd/m2.
             *
             * The reference sRGB monitor is 80cd/m2 and the limit of PQ encoding is
             * 2kcd/m2.
             */
            png_chunk_report(
                png_ptr,
                c"cLLI light level exceeds PNG limit".as_ptr(),
                PNG_CHUNK_WRITE_ERROR,
            );
            return;
        }

        (*info_ptr).maxCLL = maxCLL;
        (*info_ptr).maxFALL = maxFALL;
        (*info_ptr).valid |= PNG_INFO_cLLI;
    }
}

/* png_set_cLLI */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_cLLI(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    maxCLL: f64,
    maxFALL: f64,
) {
    unsafe {
        png_set_cLLI_fixed(
            png_ptr,
            info_ptr,
            png_fixed_ITU(png_ptr, maxCLL, c"png_set_cLLI(maxCLL)".as_ptr()),
            png_fixed_ITU(png_ptr, maxFALL, c"png_set_cLLI(maxFALL)".as_ptr()),
        );
    }
}

/* png_ITU_fixed_16 */
unsafe fn png_ITU_fixed_16(error: *mut c_int, mut v: png_fixed_point) -> png_uint_16 {
    unsafe {
        /* Return a safe uint16_t value scaled according to the ITU H273 rules for
         * 16-bit display chromaticities.  Functions like the corresponding
         * png_fixed() internal function with regard to errors: it's an error on
         * write, a chunk_benign_error on read: See the definition of
         * png_chunk_report in pngpriv.h.
         */
        v /= 2; /* rounds to 0 in C: avoids insignificant arithmetic errors */
        if v > 65535 || v < 0 {
            *error = 1;
            return 0;
        }

        v as png_uint_16 /*SAFE*/
    }
}

/* png_set_mDCV_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_mDCV_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    white_x: png_fixed_point,
    white_y: png_fixed_point,
    red_x: png_fixed_point,
    red_y: png_fixed_point,
    green_x: png_fixed_point,
    green_y: png_fixed_point,
    blue_x: png_fixed_point,
    blue_y: png_fixed_point,
    maxDL: png_uint_32,
    minDL: png_uint_32,
) {
    unsafe {
        let rx: png_uint_16;
        let ry: png_uint_16;
        let gx: png_uint_16;
        let gy: png_uint_16;
        let bx: png_uint_16;
        let by: png_uint_16;
        let wx: png_uint_16;
        let wy: png_uint_16;
        let mut error: c_int;

        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        /* Check the input values to ensure they are in the expected range: */
        error = 0;
        rx = png_ITU_fixed_16(&mut error, red_x);
        ry = png_ITU_fixed_16(&mut error, red_y);
        gx = png_ITU_fixed_16(&mut error, green_x);
        gy = png_ITU_fixed_16(&mut error, green_y);
        bx = png_ITU_fixed_16(&mut error, blue_x);
        by = png_ITU_fixed_16(&mut error, blue_y);
        wx = png_ITU_fixed_16(&mut error, white_x);
        wy = png_ITU_fixed_16(&mut error, white_y);

        if error != 0 {
            png_chunk_report(
                png_ptr,
                c"mDCV chromaticities outside representable range".as_ptr(),
                PNG_CHUNK_WRITE_ERROR,
            );
            return;
        }

        /* Check the light level range: */
        if maxDL > 0x7FFFFFFFu32 || minDL > 0x7FFFFFFFu32 {
            /* The limit is 200kcd/m2; somewhat bright but not inconceivable because
             * human vision is said to run up to 100Mcd/m2.  The sun is about 2Gcd/m2.
             *
             * The reference sRGB monitor is 80cd/m2 and the limit of PQ encoding is
             * 2kcd/m2.
             */
            png_chunk_report(
                png_ptr,
                c"mDCV display light level exceeds PNG limit".as_ptr(),
                PNG_CHUNK_WRITE_ERROR,
            );
            return;
        }

        /* All values are safe, the settings are accepted.
         *
         * IMPLEMENTATION NOTE: in practice the values can be checked and assigned
         * but the result is confusing if a writing app calls png_set_mDCV more than
         * once, the second time with an invalid value.  This approach is more
         * obviously correct at the cost of typing and a very slight machine
         * overhead.
         */
        (*info_ptr).mastering_red_x = rx;
        (*info_ptr).mastering_red_y = ry;
        (*info_ptr).mastering_green_x = gx;
        (*info_ptr).mastering_green_y = gy;
        (*info_ptr).mastering_blue_x = bx;
        (*info_ptr).mastering_blue_y = by;
        (*info_ptr).mastering_white_x = wx;
        (*info_ptr).mastering_white_y = wy;
        (*info_ptr).mastering_maxDL = maxDL;
        (*info_ptr).mastering_minDL = minDL;
        (*info_ptr).valid |= PNG_INFO_mDCV;
    }
}

/* png_set_mDCV */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_mDCV(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    white_x: f64,
    white_y: f64,
    red_x: f64,
    red_y: f64,
    green_x: f64,
    green_y: f64,
    blue_x: f64,
    blue_y: f64,
    maxDL: f64,
    minDL: f64,
) {
    unsafe {
        png_set_mDCV_fixed(
            png_ptr,
            info_ptr,
            png_fixed(png_ptr, white_x, c"png_set_mDCV(white(x))".as_ptr()),
            png_fixed(png_ptr, white_y, c"png_set_mDCV(white(y))".as_ptr()),
            png_fixed(png_ptr, red_x, c"png_set_mDCV(red(x))".as_ptr()),
            png_fixed(png_ptr, red_y, c"png_set_mDCV(red(y))".as_ptr()),
            png_fixed(png_ptr, green_x, c"png_set_mDCV(green(x))".as_ptr()),
            png_fixed(png_ptr, green_y, c"png_set_mDCV(green(y))".as_ptr()),
            png_fixed(png_ptr, blue_x, c"png_set_mDCV(blue(x))".as_ptr()),
            png_fixed(png_ptr, blue_y, c"png_set_mDCV(blue(y))".as_ptr()),
            png_fixed_ITU(png_ptr, maxDL, c"png_set_mDCV(maxDL)".as_ptr()),
            png_fixed_ITU(png_ptr, minDL, c"png_set_mDCV(minDL)".as_ptr()),
        );
    }
}

/* png_set_eXIf */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_eXIf(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    exif: png_bytep,
) {
    unsafe {
        png_warning(
            png_ptr,
            c"png_set_eXIf does not work; use png_set_eXIf_1".as_ptr(),
        );
        PNG_UNUSED(info_ptr);
        PNG_UNUSED(exif);
    }
}

/* png_set_eXIf_1 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_eXIf_1(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    num_exif: png_uint_32,
    exif: png_bytep,
) {
    unsafe {
        let new_exif: png_bytep;

        if png_ptr.is_null()
            || info_ptr.is_null()
            || ((*png_ptr).mode & PNG_WROTE_eXIf) != 0
            || exif.is_null()
        {
            return;
        }

        new_exif = png_malloc_warn(png_ptr, num_exif as png_alloc_size_t) as png_bytep;

        if new_exif.is_null() {
            png_warning(png_ptr, c"Insufficient memory for eXIf chunk data".as_ptr());
            return;
        }

        memcpy(
            new_exif as *mut c_void,
            exif as *const c_void,
            num_exif as usize,
        );

        png_free_data(png_ptr, info_ptr, PNG_FREE_EXIF, 0);

        (*info_ptr).num_exif = num_exif;
        (*info_ptr).exif = new_exif;
        (*info_ptr).free_me |= PNG_FREE_EXIF;
        (*info_ptr).valid |= PNG_INFO_eXIf;
    }
}

/* png_set_gAMA_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_gAMA_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    file_gamma: png_fixed_point,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        (*info_ptr).gamma = file_gamma;
        (*info_ptr).valid |= PNG_INFO_gAMA;
    }
}

/* png_set_gAMA */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_gAMA(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    file_gamma: f64,
) {
    unsafe {
        png_set_gAMA_fixed(
            png_ptr,
            info_ptr,
            png_fixed(png_ptr, file_gamma, c"png_set_gAMA".as_ptr()),
        );
    }
}

/* png_set_hIST */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_hIST(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mut hist: png_const_uint_16p,
) {
    unsafe {
        let mut safe_hist: [png_uint_16; PNG_MAX_PALETTE_LENGTH as usize] =
            [0; PNG_MAX_PALETTE_LENGTH as usize];
        let mut i: c_int;

        if png_ptr.is_null() || info_ptr.is_null() || hist.is_null() {
            return;
        }

        if (*info_ptr).num_palette == 0
            || (*info_ptr).num_palette as c_int > PNG_MAX_PALETTE_LENGTH
        {
            png_warning(
                png_ptr,
                c"Invalid palette size, hIST allocation skipped".as_ptr(),
            );

            return;
        }

        /* Snapshot the caller's hist before freeing, in case it points to
         * info_ptr->hist (getter-to-setter aliasing).
         */
        memcpy(
            safe_hist.as_mut_ptr() as *mut c_void,
            hist as *const c_void,
            (*info_ptr).num_palette as c_uint as usize
                * core::mem::size_of::<png_uint_16>(),
        );
        hist = safe_hist.as_ptr();

        png_free_data(png_ptr, info_ptr, PNG_FREE_HIST, 0);

        /* Changed from info->num_palette to PNG_MAX_PALETTE_LENGTH in
         * version 1.2.1
         */
        (*info_ptr).hist = png_malloc_warn(
            png_ptr,
            PNG_MAX_PALETTE_LENGTH as usize * core::mem::size_of::<png_uint_16>(),
        ) as png_uint_16p;

        if (*info_ptr).hist.is_null() {
            png_warning(png_ptr, c"Insufficient memory for hIST chunk data".as_ptr());
            return;
        }

        i = 0;
        while i < (*info_ptr).num_palette as c_int {
            *(*info_ptr).hist.add(i as usize) = *hist.add(i as usize);
            i += 1;
        }

        (*info_ptr).free_me |= PNG_FREE_HIST;
        (*info_ptr).valid |= PNG_INFO_hIST;
    }
}

/* png_set_IHDR */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_IHDR(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    width: png_uint_32,
    height: png_uint_32,
    bit_depth: c_int,
    color_type: c_int,
    interlace_type: c_int,
    compression_type: c_int,
    filter_type: c_int,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        (*info_ptr).width = width;
        (*info_ptr).height = height;
        (*info_ptr).bit_depth = bit_depth as png_byte;
        (*info_ptr).color_type = color_type as png_byte;
        (*info_ptr).compression_type = compression_type as png_byte;
        (*info_ptr).filter_type = filter_type as png_byte;
        (*info_ptr).interlace_type = interlace_type as png_byte;

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

        if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            (*info_ptr).channels = 1;
        } else if ((*info_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
            (*info_ptr).channels = 3;
        } else {
            (*info_ptr).channels = 1;
        }

        if ((*info_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0 {
            (*info_ptr).channels += 1;
        }

        (*info_ptr).pixel_depth =
            ((*info_ptr).channels as c_int * (*info_ptr).bit_depth as c_int) as png_byte;

        (*info_ptr).rowbytes = PNG_ROWBYTES((*info_ptr).pixel_depth as usize, width as usize);
    }
}

/* png_set_oFFs */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_oFFs(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    offset_x: png_int_32,
    offset_y: png_int_32,
    unit_type: c_int,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        (*info_ptr).x_offset = offset_x;
        (*info_ptr).y_offset = offset_y;
        (*info_ptr).offset_unit_type = unit_type as png_byte;
        (*info_ptr).valid |= PNG_INFO_oFFs;
    }
}

/* png_set_pCAL */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_pCAL(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    purpose: png_const_charp,
    X0: png_int_32,
    X1: png_int_32,
    r#type: c_int,
    nparams: c_int,
    units: png_const_charp,
    params: png_charpp,
) {
    unsafe {
        let mut length: usize;
        let mut i: c_int;

        if png_ptr.is_null()
            || info_ptr.is_null()
            || purpose.is_null()
            || units.is_null()
            || (nparams > 0 && params.is_null())
        {
            return;
        }

        length = strlen(purpose) + 1;

        /* TODO: validate format of calibration name and unit name */

        /* Check that the type matches the specification. */
        if r#type < 0 || r#type > 3 {
            png_chunk_report(
                png_ptr,
                c"Invalid pCAL equation type".as_ptr(),
                PNG_CHUNK_WRITE_ERROR,
            );
            return;
        }

        if nparams < 0 || nparams > 255 {
            png_chunk_report(
                png_ptr,
                c"Invalid pCAL parameter count".as_ptr(),
                PNG_CHUNK_WRITE_ERROR,
            );
            return;
        }

        /* Validate params[nparams] */
        i = 0;
        while i < nparams {
            if (*params.add(i as usize)).is_null()
                || png_check_fp_string(
                    *params.add(i as usize),
                    strlen(*params.add(i as usize)),
                ) == 0
            {
                png_chunk_report(
                    png_ptr,
                    c"Invalid format for pCAL parameter".as_ptr(),
                    PNG_CHUNK_WRITE_ERROR,
                );
                return;
            }
            i += 1;
        }

        (*info_ptr).pcal_purpose = png_malloc_warn(png_ptr, length) as png_charp;

        if (*info_ptr).pcal_purpose.is_null() {
            png_chunk_report(
                png_ptr,
                c"Insufficient memory for pCAL purpose".as_ptr(),
                PNG_CHUNK_WRITE_ERROR,
            );
            return;
        }

        memcpy(
            (*info_ptr).pcal_purpose as *mut c_void,
            purpose as *const c_void,
            length,
        );

        (*info_ptr).free_me |= PNG_FREE_PCAL;

        (*info_ptr).pcal_X0 = X0;
        (*info_ptr).pcal_X1 = X1;
        (*info_ptr).pcal_type = r#type as png_byte;
        (*info_ptr).pcal_nparams = nparams as png_byte;

        length = strlen(units) + 1;

        (*info_ptr).pcal_units = png_malloc_warn(png_ptr, length) as png_charp;

        if (*info_ptr).pcal_units.is_null() {
            png_warning(png_ptr, c"Insufficient memory for pCAL units".as_ptr());
            return;
        }

        memcpy(
            (*info_ptr).pcal_units as *mut c_void,
            units as *const c_void,
            length,
        );

        (*info_ptr).pcal_params = png_malloc_warn(
            png_ptr,
            ((nparams as c_uint + 1) as usize) * core::mem::size_of::<png_charp>(),
        ) as png_charpp;

        if (*info_ptr).pcal_params.is_null() {
            png_warning(png_ptr, c"Insufficient memory for pCAL params".as_ptr());
            return;
        }

        memset(
            (*info_ptr).pcal_params as *mut c_void,
            0,
            (nparams as c_uint + 1) as usize * core::mem::size_of::<png_charp>(),
        );

        i = 0;
        while i < nparams {
            length = strlen(*params.add(i as usize)) + 1;

            *(*info_ptr).pcal_params.add(i as usize) =
                png_malloc_warn(png_ptr, length) as png_charp;

            if (*(*info_ptr).pcal_params.add(i as usize)).is_null() {
                png_warning(png_ptr, c"Insufficient memory for pCAL parameter".as_ptr());
                return;
            }

            memcpy(
                *(*info_ptr).pcal_params.add(i as usize) as *mut c_void,
                *params.add(i as usize) as *const c_void,
                length,
            );
            i += 1;
        }

        (*info_ptr).valid |= PNG_INFO_pCAL;
    }
}

/* png_set_sCAL_s */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sCAL_s(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    unit: c_int,
    swidth: png_const_charp,
    sheight: png_const_charp,
) {
    unsafe {
        let mut lengthw: usize = 0;
        let mut lengthh: usize = 0;

        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        /* Double check the unit (should never get here with an invalid
         * unit unless this is an API call.)
         */
        if unit != 1 && unit != 2 {
            png_error(png_ptr, c"Invalid sCAL unit".as_ptr());
        }

        if swidth.is_null()
            || {
                lengthw = strlen(swidth);
                lengthw == 0
            }
            || *swidth == 45 /* '-' */
            || png_check_fp_string(swidth, lengthw) == 0
        {
            png_error(png_ptr, c"Invalid sCAL width".as_ptr());
        }

        if sheight.is_null()
            || {
                lengthh = strlen(sheight);
                lengthh == 0
            }
            || *sheight == 45 /* '-' */
            || png_check_fp_string(sheight, lengthh) == 0
        {
            png_error(png_ptr, c"Invalid sCAL height".as_ptr());
        }

        (*info_ptr).scal_unit = unit as png_byte;

        lengthw += 1;

        (*info_ptr).scal_s_width = png_malloc_warn(png_ptr, lengthw) as png_charp;

        if (*info_ptr).scal_s_width.is_null() {
            png_warning(
                png_ptr,
                c"Memory allocation failed while processing sCAL".as_ptr(),
            );

            return;
        }

        memcpy(
            (*info_ptr).scal_s_width as *mut c_void,
            swidth as *const c_void,
            lengthw,
        );

        lengthh += 1;

        (*info_ptr).scal_s_height = png_malloc_warn(png_ptr, lengthh) as png_charp;

        if (*info_ptr).scal_s_height.is_null() {
            png_free(png_ptr, (*info_ptr).scal_s_width as png_voidp);
            (*info_ptr).scal_s_width = core::ptr::null_mut();

            png_warning(
                png_ptr,
                c"Memory allocation failed while processing sCAL".as_ptr(),
            );
            return;
        }

        memcpy(
            (*info_ptr).scal_s_height as *mut c_void,
            sheight as *const c_void,
            lengthh,
        );

        (*info_ptr).free_me |= PNG_FREE_SCAL;
        (*info_ptr).valid |= PNG_INFO_sCAL;
    }
}

/* png_set_sCAL */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sCAL(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    unit: c_int,
    width: f64,
    height: f64,
) {
    unsafe {
        /* Check the arguments. */
        if width <= 0.0 {
            png_warning(png_ptr, c"Invalid sCAL width ignored".as_ptr());
        } else if height <= 0.0 {
            png_warning(png_ptr, c"Invalid sCAL height ignored".as_ptr());
        } else {
            /* Convert 'width' and 'height' to ASCII. */
            let mut swidth: [c_char; PNG_sCAL_MAX_DIGITS + 1] = [0; PNG_sCAL_MAX_DIGITS + 1];
            let mut sheight: [c_char; PNG_sCAL_MAX_DIGITS + 1] = [0; PNG_sCAL_MAX_DIGITS + 1];

            png_ascii_from_fp(
                png_ptr,
                swidth.as_mut_ptr(),
                core::mem::size_of_val(&swidth),
                width,
                PNG_sCAL_PRECISION as c_uint,
            );
            png_ascii_from_fp(
                png_ptr,
                sheight.as_mut_ptr(),
                core::mem::size_of_val(&sheight),
                height,
                PNG_sCAL_PRECISION as c_uint,
            );

            png_set_sCAL_s(png_ptr, info_ptr, unit, swidth.as_ptr(), sheight.as_ptr());
        }
    }
}

/* png_set_sCAL_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sCAL_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    unit: c_int,
    width: png_fixed_point,
    height: png_fixed_point,
) {
    unsafe {
        /* Check the arguments. */
        if width <= 0 {
            png_warning(png_ptr, c"Invalid sCAL width ignored".as_ptr());
        } else if height <= 0 {
            png_warning(png_ptr, c"Invalid sCAL height ignored".as_ptr());
        } else {
            /* Convert 'width' and 'height' to ASCII. */
            let mut swidth: [c_char; PNG_sCAL_MAX_DIGITS + 1] = [0; PNG_sCAL_MAX_DIGITS + 1];
            let mut sheight: [c_char; PNG_sCAL_MAX_DIGITS + 1] = [0; PNG_sCAL_MAX_DIGITS + 1];

            png_ascii_from_fixed(
                png_ptr,
                swidth.as_mut_ptr(),
                core::mem::size_of_val(&swidth),
                width,
            );
            png_ascii_from_fixed(
                png_ptr,
                sheight.as_mut_ptr(),
                core::mem::size_of_val(&sheight),
                height,
            );

            png_set_sCAL_s(png_ptr, info_ptr, unit, swidth.as_ptr(), sheight.as_ptr());
        }
    }
}

/* png_set_pHYs */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_pHYs(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    res_x: png_uint_32,
    res_y: png_uint_32,
    unit_type: c_int,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        (*info_ptr).x_pixels_per_unit = res_x;
        (*info_ptr).y_pixels_per_unit = res_y;
        (*info_ptr).phys_unit_type = unit_type as png_byte;
        (*info_ptr).valid |= PNG_INFO_pHYs;
    }
}

/* png_set_PLTE */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_PLTE(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    mut palette: png_const_colorp,
    num_palette: c_int,
) {
    unsafe {
        let mut safe_palette: [png_color; PNG_MAX_PALETTE_LENGTH as usize] =
            [png_color::default(); PNG_MAX_PALETTE_LENGTH as usize];
        let max_palette_length: png_uint_32;

        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        max_palette_length = if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            1u32 << (*info_ptr).bit_depth
        } else {
            PNG_MAX_PALETTE_LENGTH as png_uint_32
        };

        if num_palette < 0 || num_palette > max_palette_length as c_int {
            if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
                png_error(png_ptr, c"Invalid palette length".as_ptr());
            } else {
                png_warning(png_ptr, c"Invalid palette length".as_ptr());

                return;
            }
        }

        if (num_palette > 0 && palette.is_null())
            || (num_palette == 0
                && ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_EMPTY_PLTE) == 0)
        {
            png_error(png_ptr, c"Invalid palette".as_ptr());
        }

        /* Snapshot the caller's palette before freeing, in case it points to
         * info_ptr->palette (getter-to-setter aliasing).
         */
        if num_palette > 0 {
            memcpy(
                safe_palette.as_mut_ptr() as *mut c_void,
                palette as *const c_void,
                num_palette as c_uint as usize * core::mem::size_of::<png_color>(),
            );
        }

        palette = safe_palette.as_ptr();

        png_free_data(png_ptr, info_ptr, PNG_FREE_PLTE, 0);

        /* Changed in libpng-1.2.1 to allocate PNG_MAX_PALETTE_LENGTH instead
         * of num_palette entries, in case of an invalid PNG file or incorrect
         * call to png_set_PLTE() with too-large sample values.
         *
         * Allocate independent buffers for info_ptr and png_ptr so that the
         * lifetime of png_ptr->palette is decoupled from the lifetime of
         * info_ptr->palette.  Previously, these two pointers were aliased,
         * which caused a use-after-free vulnerability if png_free_data freed
         * info_ptr->palette while png_ptr->palette was still in use by the
         * row transform functions (e.g. png_do_expand_palette).
         *
         * Both buffers are allocated with png_calloc to zero-fill, because
         * the ARM NEON palette riffle reads all 256 entries unconditionally,
         * regardless of num_palette.
         */
        png_free(png_ptr, (*png_ptr).palette as png_voidp);
        (*png_ptr).palette = core::ptr::null_mut();
        (*png_ptr).palette = png_calloc(
            png_ptr,
            PNG_MAX_PALETTE_LENGTH as usize * core::mem::size_of::<png_color>(),
        ) as png_colorp;
        (*info_ptr).palette = png_calloc(
            png_ptr,
            PNG_MAX_PALETTE_LENGTH as usize * core::mem::size_of::<png_color>(),
        ) as png_colorp;
        (*info_ptr).num_palette = num_palette as png_uint_16;
        (*png_ptr).num_palette = (*info_ptr).num_palette;

        if num_palette > 0 {
            memcpy(
                (*info_ptr).palette as *mut c_void,
                palette as *const c_void,
                num_palette as c_uint as usize * core::mem::size_of::<png_color>(),
            );
            memcpy(
                (*png_ptr).palette as *mut c_void,
                palette as *const c_void,
                num_palette as c_uint as usize * core::mem::size_of::<png_color>(),
            );
        }

        (*info_ptr).free_me |= PNG_FREE_PLTE;
        (*info_ptr).valid |= PNG_INFO_PLTE;
    }
}

/* png_set_sBIT */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sBIT(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    sig_bit: png_const_color_8p,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() || sig_bit.is_null() {
            return;
        }

        (*info_ptr).sig_bit = *sig_bit;
        (*info_ptr).valid |= PNG_INFO_sBIT;
    }
}

/* png_set_sRGB */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sRGB(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    srgb_intent: c_int,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        (*info_ptr).rendering_intent = srgb_intent;
        (*info_ptr).valid |= PNG_INFO_sRGB;
    }
}

/* png_set_sRGB_gAMA_and_cHRM */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sRGB_gAMA_and_cHRM(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    srgb_intent: c_int,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        png_set_sRGB(png_ptr, info_ptr, srgb_intent);

        png_set_gAMA_fixed(png_ptr, info_ptr, PNG_GAMMA_sRGB_INVERSE);

        png_set_cHRM_fixed(
            png_ptr,
            info_ptr,
            /* color      x       y */
            /* white */ 31270, 32900, /* red   */ 64000, 33000, /* green */ 30000, 60000,
            /* blue  */ 15000, 6000,
        );
    }
}

/* png_set_iCCP */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_iCCP(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    name: png_const_charp,
    compression_type: c_int,
    profile: png_const_bytep,
    proflen: png_uint_32,
) {
    unsafe {
        let new_iccp_name: png_charp;
        let new_iccp_profile: png_bytep;
        let length: usize;

        if png_ptr.is_null() || info_ptr.is_null() || name.is_null() || profile.is_null() {
            return;
        }

        if compression_type != PNG_COMPRESSION_TYPE_BASE {
            png_app_error(png_ptr, c"Invalid iCCP compression method".as_ptr());
        }

        length = strlen(name) + 1;
        new_iccp_name = png_malloc_warn(png_ptr, length) as png_charp;

        if new_iccp_name.is_null() {
            png_benign_error(
                png_ptr,
                c"Insufficient memory to process iCCP chunk".as_ptr(),
            );

            return;
        }

        memcpy(
            new_iccp_name as *mut c_void,
            name as *const c_void,
            length,
        );
        new_iccp_profile = png_malloc_warn(png_ptr, proflen as png_alloc_size_t) as png_bytep;

        if new_iccp_profile.is_null() {
            png_free(png_ptr, new_iccp_name as png_voidp);
            png_benign_error(
                png_ptr,
                c"Insufficient memory to process iCCP profile".as_ptr(),
            );

            return;
        }

        memcpy(
            new_iccp_profile as *mut c_void,
            profile as *const c_void,
            proflen as usize,
        );

        png_free_data(png_ptr, info_ptr, PNG_FREE_ICCP, 0);

        (*info_ptr).iccp_proflen = proflen;
        (*info_ptr).iccp_name = new_iccp_name;
        (*info_ptr).iccp_profile = new_iccp_profile;
        (*info_ptr).free_me |= PNG_FREE_ICCP;
        (*info_ptr).valid |= PNG_INFO_iCCP;
    }
}

/* png_set_text */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_text(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    text_ptr: png_const_textp,
    num_text: c_int,
) {
    unsafe {
        let ret: c_int;
        ret = png_set_text_2(png_ptr, info_ptr, text_ptr, num_text);

        if ret != 0 {
            png_error(png_ptr, c"Insufficient memory to store text".as_ptr());
        }
    }
}

/* png_set_text_2 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_text_2(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    text_ptr: png_const_textp,
    num_text: c_int,
) -> c_int {
    unsafe {
        let mut i: c_int;
        let mut old_text: png_textp = core::ptr::null_mut();

        if png_ptr.is_null() || info_ptr.is_null() || num_text <= 0 || text_ptr.is_null() {
            return 0;
        }

        /* Make sure we have enough space in the "text" array in info_struct
         * to hold all of the incoming text_ptr objects.  This compare can't overflow
         * because max_text >= num_text (anyway, subtract of two positive integers
         * can't overflow in any case.)
         */
        if num_text > (*info_ptr).max_text - (*info_ptr).num_text {
            let old_num_text: c_int = (*info_ptr).num_text;
            let mut max_text: c_int;
            let mut new_text: png_textp = core::ptr::null_mut();

            /* Calculate an appropriate max_text, checking for overflow. */
            max_text = old_num_text;
            if num_text <= c_int::MAX - max_text {
                max_text += num_text;

                /* Round up to a multiple of 8 */
                if max_text < c_int::MAX - 8 {
                    max_text = (max_text + 8) & !0x7;
                } else {
                    max_text = c_int::MAX;
                }

                /* Now allocate a new array and copy the old members in; this does all
                 * the overflow checks.
                 */
                new_text = png_realloc_array(
                    png_ptr,
                    (*info_ptr).text as png_const_voidp,
                    old_num_text,
                    max_text - old_num_text,
                    core::mem::size_of::<png_text>(),
                ) as png_textp;
            }

            if new_text.is_null() {
                png_chunk_report(
                    png_ptr,
                    c"too many text chunks".as_ptr(),
                    PNG_CHUNK_WRITE_ERROR,
                );

                return 1;
            }

            /* Defer freeing the old array until after the copy loop below,
             * in case text_ptr aliases info_ptr->text (getter-to-setter).
             */
            old_text = (*info_ptr).text;

            (*info_ptr).text = new_text;
            (*info_ptr).free_me |= PNG_FREE_TEXT;
            (*info_ptr).max_text = max_text;
            /* num_text is adjusted below as the entries are copied in */
        }

        i = 0;
        while i < num_text {
            let text_length: usize;
            let key_len: usize;
            let lang_len: usize;
            let lang_key_len: usize;
            let textp: png_textp = (*info_ptr).text.add((*info_ptr).num_text as usize);

            if (*text_ptr.add(i as usize)).key.is_null() {
                i += 1;
                continue;
            }

            if (*text_ptr.add(i as usize)).compression < PNG_TEXT_COMPRESSION_NONE
                || (*text_ptr.add(i as usize)).compression >= PNG_TEXT_COMPRESSION_LAST
            {
                png_chunk_report(
                    png_ptr,
                    c"text compression mode is out of range".as_ptr(),
                    PNG_CHUNK_WRITE_ERROR,
                );
                i += 1;
                continue;
            }

            key_len = strlen((*text_ptr.add(i as usize)).key);

            if (*text_ptr.add(i as usize)).compression <= 0 {
                lang_len = 0;
                lang_key_len = 0;
            } else {
                /* Set iTXt data */

                if !(*text_ptr.add(i as usize)).lang.is_null() {
                    lang_len = strlen((*text_ptr.add(i as usize)).lang);
                } else {
                    lang_len = 0;
                }

                if !(*text_ptr.add(i as usize)).lang_key.is_null() {
                    lang_key_len = strlen((*text_ptr.add(i as usize)).lang_key);
                } else {
                    lang_key_len = 0;
                }
            }

            if (*text_ptr.add(i as usize)).text.is_null()
                || *(*text_ptr.add(i as usize)).text == b'\0' as c_char
            {
                text_length = 0;
                if (*text_ptr.add(i as usize)).compression > 0 {
                    (*textp).compression = PNG_ITXT_COMPRESSION_NONE;
                } else {
                    (*textp).compression = PNG_TEXT_COMPRESSION_NONE;
                }
            } else {
                text_length = strlen((*text_ptr.add(i as usize)).text);
                (*textp).compression = (*text_ptr.add(i as usize)).compression;
            }

            (*textp).key = png_malloc_base(
                png_ptr,
                key_len + text_length + lang_len + lang_key_len + 4,
            ) as png_charp;

            if (*textp).key.is_null() {
                png_chunk_report(
                    png_ptr,
                    c"text chunk: out of memory".as_ptr(),
                    PNG_CHUNK_WRITE_ERROR,
                );
                png_free(png_ptr, old_text as png_voidp);

                return 1;
            }

            memcpy(
                (*textp).key as *mut c_void,
                (*text_ptr.add(i as usize)).key as *const c_void,
                key_len,
            );
            *(*textp).key.add(key_len) = b'\0' as c_char;

            if (*text_ptr.add(i as usize)).compression > 0 {
                (*textp).lang = (*textp).key.add(key_len + 1);
                memcpy(
                    (*textp).lang as *mut c_void,
                    (*text_ptr.add(i as usize)).lang as *const c_void,
                    lang_len,
                );
                *(*textp).lang.add(lang_len) = b'\0' as c_char;
                (*textp).lang_key = (*textp).lang.add(lang_len + 1);
                memcpy(
                    (*textp).lang_key as *mut c_void,
                    (*text_ptr.add(i as usize)).lang_key as *const c_void,
                    lang_key_len,
                );
                *(*textp).lang_key.add(lang_key_len) = b'\0' as c_char;
                (*textp).text = (*textp).lang_key.add(lang_key_len + 1);
            } else {
                (*textp).lang = core::ptr::null_mut();
                (*textp).lang_key = core::ptr::null_mut();
                (*textp).text = (*textp).key.add(key_len + 1);
            }

            if text_length != 0 {
                memcpy(
                    (*textp).text as *mut c_void,
                    (*text_ptr.add(i as usize)).text as *const c_void,
                    text_length,
                );
            }

            *(*textp).text.add(text_length) = b'\0' as c_char;

            if (*textp).compression > 0 {
                (*textp).text_length = 0;
                (*textp).itxt_length = text_length;
            } else {
                (*textp).text_length = text_length;
                (*textp).itxt_length = 0;
            }

            (*info_ptr).num_text += 1;
            i += 1;
        }

        png_free(png_ptr, old_text as png_voidp);

        0
    }
}

/* png_set_tIME */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_tIME(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mod_time: png_const_timep,
) {
    unsafe {
        if png_ptr.is_null()
            || info_ptr.is_null()
            || mod_time.is_null()
            || ((*png_ptr).mode & PNG_WROTE_tIME) != 0
        {
            return;
        }

        if (*mod_time).month == 0
            || (*mod_time).month > 12
            || (*mod_time).day == 0
            || (*mod_time).day > 31
            || (*mod_time).hour > 23
            || (*mod_time).minute > 59
            || (*mod_time).second > 60
        {
            png_warning(png_ptr, c"Ignoring invalid time value".as_ptr());

            return;
        }

        (*info_ptr).mod_time = *mod_time;
        (*info_ptr).valid |= PNG_INFO_tIME;
    }
}

/* png_set_tRNS */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_tRNS(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    mut trans_alpha: png_const_bytep,
    mut num_trans: c_int,
    trans_color: png_const_color_16p,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        if !trans_alpha.is_null() {
            /* Snapshot the caller's trans_alpha before freeing, in case it
             * points to info_ptr->trans_alpha (getter-to-setter aliasing).
             */
            let mut safe_trans: [png_byte; PNG_MAX_PALETTE_LENGTH as usize] =
                [0; PNG_MAX_PALETTE_LENGTH as usize];

            if num_trans > 0 && num_trans <= PNG_MAX_PALETTE_LENGTH {
                memcpy(
                    safe_trans.as_mut_ptr() as *mut c_void,
                    trans_alpha as *const c_void,
                    num_trans as usize,
                );
            }

            trans_alpha = safe_trans.as_ptr();

            png_free_data(png_ptr, info_ptr, PNG_FREE_TRNS, 0);

            if num_trans > 0 && num_trans <= PNG_MAX_PALETTE_LENGTH {
                /* Allocate info_ptr's copy of the transparency data.
                 * Initialize all entries to fully opaque (0xff), then overwrite
                 * the first num_trans entries with the actual values.
                 */
                (*info_ptr).trans_alpha =
                    png_malloc(png_ptr, PNG_MAX_PALETTE_LENGTH as png_alloc_size_t) as png_bytep;
                memset(
                    (*info_ptr).trans_alpha as *mut c_void,
                    0xff,
                    PNG_MAX_PALETTE_LENGTH as usize,
                );
                memcpy(
                    (*info_ptr).trans_alpha as *mut c_void,
                    trans_alpha as *const c_void,
                    num_trans as usize,
                );
                (*info_ptr).free_me |= PNG_FREE_TRNS;
                (*info_ptr).valid |= PNG_INFO_tRNS;

                /* Allocate an independent copy for png_struct, so that the
                 * lifetime of png_ptr->trans_alpha is decoupled from the
                 * lifetime of info_ptr->trans_alpha.  Previously these two
                 * pointers were aliased, which caused a use-after-free if
                 * png_free_data freed info_ptr->trans_alpha while
                 * png_ptr->trans_alpha was still in use by the row transform
                 * functions (e.g. png_do_expand_palette).
                 */
                png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
                (*png_ptr).trans_alpha = core::ptr::null_mut();
                (*png_ptr).trans_alpha =
                    png_malloc(png_ptr, PNG_MAX_PALETTE_LENGTH as png_alloc_size_t) as png_bytep;
                memset(
                    (*png_ptr).trans_alpha as *mut c_void,
                    0xff,
                    PNG_MAX_PALETTE_LENGTH as usize,
                );
                memcpy(
                    (*png_ptr).trans_alpha as *mut c_void,
                    trans_alpha as *const c_void,
                    num_trans as usize,
                );
            } else {
                png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
                (*png_ptr).trans_alpha = core::ptr::null_mut();
            }
        }

        if !trans_color.is_null() {
            if ((*info_ptr).bit_depth as c_int) < 16 {
                let sample_max: c_int = (1 << (*info_ptr).bit_depth) - 1;

                if ((*info_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY
                    && (*trans_color).gray as c_int > sample_max)
                    || ((*info_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB
                        && ((*trans_color).red as c_int > sample_max
                            || (*trans_color).green as c_int > sample_max
                            || (*trans_color).blue as c_int > sample_max))
                {
                    png_warning(
                        png_ptr,
                        c"tRNS chunk has out-of-range samples for bit_depth".as_ptr(),
                    );
                }
            }

            (*info_ptr).trans_color = *trans_color;

            if num_trans == 0 {
                num_trans = 1;
            }
        }

        (*info_ptr).num_trans = num_trans as png_uint_16;

        if num_trans != 0 {
            (*info_ptr).free_me |= PNG_FREE_TRNS;
            (*info_ptr).valid |= PNG_INFO_tRNS;
        }
    }
}

/* png_set_sPLT */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sPLT(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mut entries: png_const_sPLT_tp,
    mut nentries: c_int,
) {
    unsafe {
        let mut np: png_sPLT_tp;
        let old_spalettes: png_sPLT_tp;

        if png_ptr.is_null() || info_ptr.is_null() || nentries <= 0 || entries.is_null() {
            return;
        }

        /* Use the internal realloc function, which checks for all the possible
         * overflows.  Notice that the parameters are (int) and (size_t)
         */
        np = png_realloc_array(
            png_ptr,
            (*info_ptr).splt_palettes as png_const_voidp,
            (*info_ptr).splt_palettes_num,
            nentries,
            core::mem::size_of::<png_sPLT_t>(),
        ) as png_sPLT_tp;

        if np.is_null() {
            /* Out of memory or too many chunks */
            png_chunk_report(
                png_ptr,
                c"too many sPLT chunks".as_ptr(),
                PNG_CHUNK_WRITE_ERROR,
            );
            return;
        }

        /* Defer freeing the old array until after the copy loop below,
         * in case entries aliases info_ptr->splt_palettes (getter-to-setter).
         */
        old_spalettes = (*info_ptr).splt_palettes;

        (*info_ptr).splt_palettes = np;
        (*info_ptr).free_me |= PNG_FREE_SPLT;

        np = np.add((*info_ptr).splt_palettes_num as usize);

        loop {
            let length: usize;

            /* Skip invalid input entries */
            if (*entries).name.is_null() || (*entries).entries.is_null() {
                /* png_handle_sPLT doesn't do this, so this is an app error */
                png_app_error(png_ptr, c"png_set_sPLT: invalid sPLT".as_ptr());
                /* Just skip the invalid entry */
                nentries -= 1;
                if nentries == 0 {
                    break;
                }
                continue;
            }

            (*np).depth = (*entries).depth;

            /* In the event of out-of-memory just return - there's no point keeping
             * on trying to add sPLT chunks.
             */
            length = strlen((*entries).name) + 1;
            (*np).name = png_malloc_base(png_ptr, length) as png_charp;

            if (*np).name.is_null() {
                break;
            }

            memcpy(
                (*np).name as *mut c_void,
                (*entries).name as *const c_void,
                length,
            );

            /* IMPORTANT: we have memory now that won't get freed if something else
             * goes wrong; this code must free it.  png_malloc_array produces no
             * warnings; use a png_chunk_report (below) if there is an error.
             */
            (*np).entries = png_malloc_array(
                png_ptr,
                (*entries).nentries,
                core::mem::size_of::<png_sPLT_entry>(),
            ) as png_sPLT_entryp;

            if (*np).entries.is_null() {
                png_free(png_ptr, (*np).name as png_voidp);
                (*np).name = core::ptr::null_mut();
                break;
            }

            (*np).nentries = (*entries).nentries;
            /* This multiply can't overflow because png_malloc_array has already
             * checked it when doing the allocation.
             */
            memcpy(
                (*np).entries as *mut c_void,
                (*entries).entries as *const c_void,
                (*entries).nentries as c_uint as usize * core::mem::size_of::<png_sPLT_entry>(),
            );

            /* Note that 'continue' skips the advance of the out pointer and out
             * count, so an invalid entry is not added.
             */
            (*info_ptr).valid |= PNG_INFO_sPLT;
            (*info_ptr).splt_palettes_num += 1;
            np = np.add(1);
            entries = entries.add(1);

            nentries -= 1;
            if nentries == 0 {
                break;
            }
        }

        png_free(png_ptr, old_spalettes as png_voidp);

        if nentries > 0 {
            png_chunk_report(
                png_ptr,
                c"sPLT out of memory".as_ptr(),
                PNG_CHUNK_WRITE_ERROR,
            );
        }
    }
}

/* check_location */
unsafe fn check_location(png_ptr: png_const_structrp, mut location: c_int) -> png_byte {
    unsafe {
        location &= (PNG_HAVE_IHDR | PNG_HAVE_PLTE | PNG_AFTER_IDAT) as c_int;

        /* New in 1.6.0; copy the location and check it.  This is an API
         * change; previously the app had to use the
         * png_set_unknown_chunk_location API below for each chunk.
         */
        if location == 0 && ((*png_ptr).mode & PNG_IS_READ_STRUCT) == 0 {
            /* Write struct, so unknown chunks come from the app */
            png_app_warning(
                png_ptr,
                c"png_set_unknown_chunks now expects a valid location".as_ptr(),
            );
            /* Use the old behavior */
            location = ((*png_ptr).mode
                & (PNG_HAVE_IHDR | PNG_HAVE_PLTE | PNG_AFTER_IDAT)) as png_byte
                as c_int;
        }

        /* This need not be an internal error - if the app calls
         * png_set_unknown_chunks on a read pointer it must get the location right.
         */
        if location == 0 {
            png_error(png_ptr, c"invalid location in png_set_unknown_chunks".as_ptr());
        }

        /* Now reduce the location to the top-most set bit by removing each least
         * significant bit in turn.
         */
        while location != (location & location.wrapping_neg()) {
            location &= !(location & location.wrapping_neg());
        }

        /* The cast is safe because 'location' is a bit mask and only the low four
         * bits are significant.
         */
        location as png_byte
    }
}

/* png_set_unknown_chunks */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_unknown_chunks(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mut unknowns: png_const_unknown_chunkp,
    mut num_unknowns: c_int,
) {
    unsafe {
        let mut np: png_unknown_chunkp;
        let old_unknowns: png_unknown_chunkp;

        if png_ptr.is_null() || info_ptr.is_null() || num_unknowns <= 0 || unknowns.is_null() {
            return;
        }

        /* Prior to 1.6.0 this code used png_malloc_warn; however, this meant that
         * unknown critical chunks could be lost with just a warning resulting in
         * undefined behavior.  Now png_chunk_report is used to provide behavior
         * appropriate to read or write.
         */
        np = png_realloc_array(
            png_ptr,
            (*info_ptr).unknown_chunks as png_const_voidp,
            (*info_ptr).unknown_chunks_num,
            num_unknowns,
            core::mem::size_of::<png_unknown_chunk>(),
        ) as png_unknown_chunkp;

        if np.is_null() {
            png_chunk_report(
                png_ptr,
                c"too many unknown chunks".as_ptr(),
                PNG_CHUNK_WRITE_ERROR,
            );
            return;
        }

        /* Defer freeing the old array until after the copy loop below,
         * in case unknowns aliases info_ptr->unknown_chunks (getter-to-setter).
         */
        old_unknowns = (*info_ptr).unknown_chunks;

        (*info_ptr).unknown_chunks = np; /* safe because it is initialized */
        (*info_ptr).free_me |= PNG_FREE_UNKN;

        np = np.add((*info_ptr).unknown_chunks_num as usize);

        /* Increment unknown_chunks_num each time round the loop to protect the
         * just-allocated chunk data.
         */
        while num_unknowns > 0 {
            memcpy(
                (*np).name.as_mut_ptr() as *mut c_void,
                (*unknowns).name.as_ptr() as *const c_void,
                core::mem::size_of_val(&(*np).name),
            );
            (*np).name[core::mem::size_of_val(&(*np).name) - 1] = b'\0';
            (*np).location = check_location(png_ptr, (*unknowns).location as c_int);

            if (*unknowns).size == 0 {
                (*np).data = core::ptr::null_mut();
                (*np).size = 0;
            } else {
                (*np).data = png_malloc_base(png_ptr, (*unknowns).size) as png_bytep;

                if (*np).data.is_null() {
                    png_chunk_report(
                        png_ptr,
                        c"unknown chunk: out of memory".as_ptr(),
                        PNG_CHUNK_WRITE_ERROR,
                    );
                    /* But just skip storing the unknown chunk */
                    num_unknowns -= 1;
                    unknowns = unknowns.add(1);
                    continue;
                }

                memcpy(
                    (*np).data as *mut c_void,
                    (*unknowns).data as *const c_void,
                    (*unknowns).size,
                );
                (*np).size = (*unknowns).size;
            }

            /* These increments are skipped on out-of-memory for the data - the
             * unknown chunk entry gets overwritten if the png_chunk_report returns.
             * This is correct in the read case (the chunk is just dropped.)
             */
            np = np.add(1);
            (*info_ptr).unknown_chunks_num += 1;

            num_unknowns -= 1;
            unknowns = unknowns.add(1);
        }

        png_free(png_ptr, old_unknowns as png_voidp);
    }
}

/* png_set_unknown_chunk_location */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_unknown_chunk_location(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    chunk: c_int,
    mut location: c_int,
) {
    unsafe {
        /* This API is pretty pointless in 1.6.0 because the location can be set
         * before the call to png_set_unknown_chunks.
         *
         * TODO: add a png_app_warning in 1.7
         */
        if !png_ptr.is_null()
            && !info_ptr.is_null()
            && chunk >= 0
            && chunk < (*info_ptr).unknown_chunks_num
        {
            if (location & (PNG_HAVE_IHDR | PNG_HAVE_PLTE | PNG_AFTER_IDAT) as c_int) == 0 {
                png_app_error(png_ptr, c"invalid unknown chunk location".as_ptr());
                /* Fake out the pre 1.6.0 behavior: */
                if (location as c_uint & PNG_HAVE_IDAT) != 0 {
                    /* undocumented! */
                    location = PNG_AFTER_IDAT as c_int;
                } else {
                    location = PNG_HAVE_IHDR as c_int; /* also undocumented */
                }
            }

            (*(*info_ptr).unknown_chunks.add(chunk as usize)).location =
                check_location(png_ptr, location);
        }
    }
}

/* png_permit_mng_features */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_permit_mng_features(
    png_ptr: png_structrp,
    mng_features: png_uint_32,
) -> png_uint_32 {
    unsafe {
        if png_ptr.is_null() {
            return 0;
        }

        (*png_ptr).mng_features_permitted = mng_features & PNG_ALL_MNG_FEATURES;

        (*png_ptr).mng_features_permitted
    }
}

/* add_one_chunk */
unsafe fn add_one_chunk(
    mut list: png_bytep,
    count: c_uint,
    add: png_const_bytep,
    keep: c_int,
) -> c_uint {
    unsafe {
        let mut i: c_uint;

        /* Utility function: update the 'keep' state of a chunk if it is already in
         * the list, otherwise add it to the list.
         */
        i = 0;
        while i < count {
            if memcmp(list as *const c_void, add as *const c_void, 4) == 0 {
                *list.add(4) = keep as png_byte;

                return count;
            }
            i += 1;
            list = list.add(5);
        }

        let mut count = count;
        if keep != PNG_HANDLE_CHUNK_AS_DEFAULT {
            count += 1;
            memcpy(list as *mut c_void, add as *const c_void, 4);
            *list.add(4) = keep as png_byte;
        }

        count
    }
}

/* png_set_keep_unknown_chunks */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_keep_unknown_chunks(
    png_ptr: png_structrp,
    keep: c_int,
    mut chunk_list: png_const_bytep,
    num_chunks_in: c_int,
) {
    unsafe {
        let mut new_list: png_bytep;
        let mut num_chunks: c_uint;
        let mut old_num_chunks: c_uint;

        if png_ptr.is_null() {
            return;
        }

        if keep < 0 || keep >= PNG_HANDLE_CHUNK_LAST {
            png_app_error(
                png_ptr,
                c"png_set_keep_unknown_chunks: invalid keep".as_ptr(),
            );

            return;
        }

        if num_chunks_in <= 0 {
            (*png_ptr).unknown_default = keep;

            /* '0' means just set the flags, so stop here */
            if num_chunks_in == 0 {
                return;
            }
        }

        if num_chunks_in < 0 {
            /* Ignore all unknown chunks and all chunks recognized by
             * libpng except for IHDR, PLTE, tRNS, IDAT, and IEND
             */
            static chunks_to_ignore: [png_byte; 105] = [
                98, 75, 71, 68, b'\0', /* bKGD */
                99, 72, 82, 77, b'\0', /* cHRM */
                99, 73, 67, 80, b'\0', /* cICP */
                99, 76, 76, 73, b'\0', /* cLLI */
                101, 88, 73, 102, b'\0', /* eXIf */
                103, 65, 77, 65, b'\0', /* gAMA */
                104, 73, 83, 84, b'\0', /* hIST */
                105, 67, 67, 80, b'\0', /* iCCP */
                105, 84, 88, 116, b'\0', /* iTXt */
                109, 68, 67, 86, b'\0', /* mDCV */
                111, 70, 70, 115, b'\0', /* oFFs */
                112, 67, 65, 76, b'\0', /* pCAL */
                112, 72, 89, 115, b'\0', /* pHYs */
                115, 66, 73, 84, b'\0', /* sBIT */
                115, 67, 65, 76, b'\0', /* sCAL */
                115, 80, 76, 84, b'\0', /* sPLT */
                115, 84, 69, 82, b'\0', /* sTER */
                115, 82, 71, 66, b'\0', /* sRGB */
                116, 69, 88, 116, b'\0', /* tEXt */
                116, 73, 77, 69, b'\0', /* tIME */
                122, 84, 88, 116, b'\0', /* zTXt */
            ];

            chunk_list = chunks_to_ignore.as_ptr();
            num_chunks = (core::mem::size_of_val(&chunks_to_ignore) / 5usize) as c_uint /*SAFE*/;
        } else {
            /* num_chunks_in > 0 */
            if chunk_list.is_null() {
                /* Prior to 1.6.0 this was silently ignored, now it is an app_error
                 * which can be switched off.
                 */
                png_app_error(
                    png_ptr,
                    c"png_set_keep_unknown_chunks: no chunk list".as_ptr(),
                );

                return;
            }

            num_chunks = num_chunks_in as c_uint;
        }

        old_num_chunks = (*png_ptr).num_chunk_list;
        if (*png_ptr).chunk_list.is_null() {
            old_num_chunks = 0;
        }

        /* Since num_chunks is always restricted to UINT_MAX/5 this can't overflow.
         */
        if num_chunks + old_num_chunks > c_uint::MAX / 5 {
            png_app_error(
                png_ptr,
                c"png_set_keep_unknown_chunks: too many chunks".as_ptr(),
            );

            return;
        }

        /* If these chunks are being reset to the default then no more memory is
         * required because add_one_chunk above doesn't extend the list if the 'keep'
         * parameter is the default.
         */
        if keep != 0 {
            new_list = png_malloc(
                png_ptr,
                (5 * (num_chunks + old_num_chunks)) as png_alloc_size_t,
            ) as png_bytep;

            if old_num_chunks > 0 {
                memcpy(
                    new_list as *mut c_void,
                    (*png_ptr).chunk_list as *const c_void,
                    (5 * old_num_chunks) as usize,
                );
            }
        } else if old_num_chunks > 0 {
            new_list = (*png_ptr).chunk_list;
        } else {
            new_list = core::ptr::null_mut();
        }

        /* Add the new chunks together with each one's handling code.  If the chunk
         * already exists the code is updated, otherwise the chunk is added to the
         * end.  (In libpng 1.6.0 order no longer matters because this code enforces
         * the earlier convention that the last setting is the one that is used.)
         */
        if !new_list.is_null() {
            let mut inlist: png_const_bytep;
            let mut outlist: png_bytep;
            let mut i: c_uint;

            i = 0;
            while i < num_chunks {
                old_num_chunks = add_one_chunk(
                    new_list,
                    old_num_chunks,
                    chunk_list.add((5 * i) as usize),
                    keep,
                );
                i += 1;
            }

            /* Now remove any spurious 'default' entries. */
            num_chunks = 0;
            i = 0;
            inlist = new_list;
            outlist = new_list;
            while i < old_num_chunks {
                if *inlist.add(4) != 0 {
                    if outlist != inlist as png_bytep {
                        memcpy(outlist as *mut c_void, inlist as *const c_void, 5);
                    }
                    outlist = outlist.add(5);
                    num_chunks += 1;
                }
                i += 1;
                inlist = inlist.add(5);
            }

            /* This means the application has removed all the specialized handling. */
            if num_chunks == 0 {
                if (*png_ptr).chunk_list != new_list {
                    png_free(png_ptr, new_list as png_voidp);
                }

                new_list = core::ptr::null_mut();
            }
        } else {
            num_chunks = 0;
        }

        (*png_ptr).num_chunk_list = num_chunks;

        if (*png_ptr).chunk_list != new_list {
            if !(*png_ptr).chunk_list.is_null() {
                png_free(png_ptr, (*png_ptr).chunk_list as png_voidp);
            }

            (*png_ptr).chunk_list = new_list;
        }
    }
}

/* png_set_read_user_chunk_fn */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_read_user_chunk_fn(
    png_ptr: png_structrp,
    user_chunk_ptr: png_voidp,
    read_user_chunk_fn: png_user_chunk_ptr,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        (*png_ptr).read_user_chunk_fn = read_user_chunk_fn;
        (*png_ptr).user_chunk_ptr = user_chunk_ptr;
    }
}

/* png_set_rows */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_rows(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    row_pointers: png_bytepp,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        if !(*info_ptr).row_pointers.is_null() && ((*info_ptr).row_pointers != row_pointers) {
            png_free_data(png_ptr, info_ptr, PNG_FREE_ROWS, 0);
        }

        (*info_ptr).row_pointers = row_pointers;

        if !row_pointers.is_null() {
            (*info_ptr).valid |= PNG_INFO_IDAT;
        }
    }
}

/* png_set_compression_buffer_size */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_compression_buffer_size(
    png_ptr: png_structrp,
    mut size: usize,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        if size == 0 || size > PNG_UINT_31_MAX as usize {
            png_error(png_ptr, c"invalid compression buffer size".as_ptr());
        }

        if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 {
            (*png_ptr).IDAT_read_size = size as png_uint_32; /* checked above */
            return;
        }

        if ((*png_ptr).mode & PNG_IS_READ_STRUCT) == 0 {
            if (*png_ptr).zowner != 0 {
                png_warning(
                    png_ptr,
                    c"Compression buffer size cannot be changed because it is in use".as_ptr(),
                );

                return;
            }

            /* Some compilers complain that this is always false.  However, it
             * can be true when integer overflow happens.
             */
            if size > ZLIB_IO_MAX as usize {
                png_warning(
                    png_ptr,
                    c"Compression buffer size limited to system maximum".as_ptr(),
                );
                size = ZLIB_IO_MAX as usize; /* must fit */
            }

            if size < 6 {
                /* Deflate will potentially go into an infinite loop on a SYNC_FLUSH
                 * if this is permitted.
                 */
                png_warning(
                    png_ptr,
                    c"Compression buffer size cannot be reduced below 6".as_ptr(),
                );

                return;
            }

            if (*png_ptr).zbuffer_size as usize != size {
                png_free_buffer_list(png_ptr, &raw mut (*png_ptr).zbuffer_list);
                (*png_ptr).zbuffer_size = size as uInt;
            }
        }
    }
}

/* png_set_invalid */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_invalid(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mask: c_int,
) {
    unsafe {
        if !png_ptr.is_null() && !info_ptr.is_null() {
            (*info_ptr).valid &= !mask as c_uint;
        }
    }
}

/* png_set_user_limits */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_user_limits(
    png_ptr: png_structrp,
    user_width_max: png_uint_32,
    user_height_max: png_uint_32,
) {
    unsafe {
        /* Images with dimensions larger than these limits will be
         * rejected by png_set_IHDR().  To accept any PNG datastream
         * regardless of dimensions, set both limits to 0x7fffffff.
         */
        if png_ptr.is_null() {
            return;
        }

        (*png_ptr).user_width_max = user_width_max;
        (*png_ptr).user_height_max = user_height_max;
    }
}

/* png_set_chunk_cache_max */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_chunk_cache_max(
    png_ptr: png_structrp,
    user_chunk_cache_max: png_uint_32,
) {
    unsafe {
        if !png_ptr.is_null() {
            (*png_ptr).user_chunk_cache_max = user_chunk_cache_max;
        }
    }
}

/* png_set_chunk_malloc_max */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_chunk_malloc_max(
    png_ptr: png_structrp,
    user_chunk_malloc_max: png_alloc_size_t,
) {
    unsafe {
        /* pngstruct::user_chunk_malloc_max is initialized to a non-zero value in
         * png.c.  This API supports '0' for unlimited, make sure the correct
         * (unlimited) value is set here to avoid a need to check for 0 everywhere
         * the parameter is used.
         */
        if !png_ptr.is_null() {
            if user_chunk_malloc_max == 0 {
                /* unlimited */
                (*png_ptr).user_chunk_malloc_max = PNG_SIZE_MAX;
            } else {
                (*png_ptr).user_chunk_malloc_max = user_chunk_malloc_max;
            }
        }
    }
}

/* png_set_benign_errors */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_benign_errors(png_ptr: png_structrp, allowed: c_int) {
    unsafe {
        /* If allowed is 1, png_benign_error() is treated as a warning.
         *
         * If allowed is 0, png_benign_error() is treated as an error (which
         * is the default behavior if png_set_benign_errors() is not called).
         */

        if allowed != 0 {
            (*png_ptr).flags |=
                PNG_FLAG_BENIGN_ERRORS_WARN | PNG_FLAG_APP_WARNINGS_WARN | PNG_FLAG_APP_ERRORS_WARN;
        } else {
            (*png_ptr).flags &= !(PNG_FLAG_BENIGN_ERRORS_WARN
                | PNG_FLAG_APP_WARNINGS_WARN
                | PNG_FLAG_APP_ERRORS_WARN);
        }
    }
}

/* png_set_check_for_invalid_index */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_check_for_invalid_index(
    png_ptr: png_structrp,
    allowed: c_int,
) {
    unsafe {
        if allowed > 0 {
            (*png_ptr).num_palette_max = 0;
        } else {
            (*png_ptr).num_palette_max = -1;
        }
    }
}

/* png_check_keyword */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_check_keyword(
    png_ptr: png_structrp,
    key: png_const_charp,
    mut new_key: png_bytep,
) -> png_uint_32 {
    unsafe {
        let orig_key: png_const_charp = key;
        let mut key: png_const_charp = key;
        let mut key_len: png_uint_32 = 0;
        let mut bad_character: c_int = 0;
        let mut space: c_int = 1;

        if key.is_null() {
            *new_key = 0;
            return 0;
        }

        while *key != 0 && key_len < 79 {
            let ch: png_byte = *key as png_byte;
            key = key.add(1);

            if (ch > 32 && ch <= 126) || (ch >= 161 /*&& ch <= 255*/) {
                *new_key = ch;
                new_key = new_key.add(1);
                key_len += 1;
                space = 0;
            } else if space == 0 {
                /* A space or an invalid character when one wasn't seen immediately
                 * before; output just a space.
                 */
                *new_key = 32;
                new_key = new_key.add(1);
                key_len += 1;
                space = 1;

                /* If the character was not a space then it is invalid. */
                if ch != 32 {
                    bad_character = ch as c_int;
                }
            } else if bad_character == 0 {
                bad_character = ch as c_int; /* just skip it, record the first error */
            }
        }

        if key_len > 0 && space != 0 {
            /* trailing space */
            key_len -= 1;
            new_key = new_key.sub(1);
            if bad_character == 0 {
                bad_character = 32;
            }
        }

        /* Terminate the keyword */
        *new_key = 0;

        if key_len == 0 {
            return 0;
        }

        /* Try to only output one warning per keyword: */
        if *key != 0 {
            /* keyword too long */
            png_warning(png_ptr, c"keyword truncated".as_ptr());
        } else if bad_character != 0 {
            let mut p: png_warning_parameters = [[0; 32]; 8];

            png_warning_parameter(&mut p, 1, orig_key);
            png_warning_parameter_signed(&mut p, 2, PNG_NUMBER_FORMAT_02x, bad_character);

            png_formatted_warning(
                png_ptr,
                &mut p,
                c"keyword \"@1\": bad character '0x@2'".as_ptr(),
            );
        }

        key_len
    }
}
