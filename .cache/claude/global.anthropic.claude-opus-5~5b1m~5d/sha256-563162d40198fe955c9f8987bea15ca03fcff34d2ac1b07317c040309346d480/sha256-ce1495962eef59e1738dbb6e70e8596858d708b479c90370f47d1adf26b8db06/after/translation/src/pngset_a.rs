//! pngset.c lines 1-1155: storage of image information into info struct
//!
//! The functions here are used during reads to store data from the file
//! into the info struct, and during writes to store application data
//! into the info struct for writing into the file.  This abstracts the
//! info struct and allows us to change the structure in the future.
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_bKGD(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    background: png_const_color_16p,
) {
    if png_ptr.is_null() || info_ptr.is_null() || background.is_null() {
        return;
    }

    (*info_ptr).background = *background;
    (*info_ptr).valid |= PNG_INFO_bKGD;
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_cHRM(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    white_x: c_double,
    white_y: c_double,
    red_x: c_double,
    red_y: c_double,
    green_x: c_double,
    green_y: c_double,
    blue_x: c_double,
    blue_y: c_double,
) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_cHRM_XYZ(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    red_X: c_double,
    red_Y: c_double,
    red_Z: c_double,
    green_X: c_double,
    green_Y: c_double,
    green_Z: c_double,
    blue_X: c_double,
    blue_Y: c_double,
    blue_Z: c_double,
) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_cICP(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    colour_primaries: png_byte,
    transfer_function: png_byte,
    matrix_coefficients: png_byte,
    video_full_range_flag: png_byte,
) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_cLLI(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    maxCLL: c_double,
    maxFALL: c_double,
) {
    png_set_cLLI_fixed(
        png_ptr,
        info_ptr,
        png_fixed_ITU(png_ptr, maxCLL, c"png_set_cLLI(maxCLL)".as_ptr()),
        png_fixed_ITU(png_ptr, maxFALL, c"png_set_cLLI(maxFALL)".as_ptr()),
    );
}

pub unsafe fn png_ITU_fixed_16(error: *mut c_int, v: png_fixed_point) -> png_uint_16 {
    /* Return a safe uint16_t value scaled according to the ITU H273 rules for
     * 16-bit display chromaticities.  Functions like the corresponding
     * png_fixed() internal function with regard to errors: it's an error on
     * write, a chunk_benign_error on read: See the definition of
     * png_chunk_report in pngpriv.h.
     */
    let mut v = v;
    v /= 2; /* rounds to 0 in C: avoids insignificant arithmetic errors */
    if v > 65535 || v < 0 {
        *error = 1;
        return 0;
    }

    v as png_uint_16 /*SAFE*/
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_mDCV(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    white_x: c_double,
    white_y: c_double,
    red_x: c_double,
    red_y: c_double,
    green_x: c_double,
    green_y: c_double,
    blue_x: c_double,
    blue_y: c_double,
    maxDL: c_double,
    minDL: c_double,
) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_eXIf(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    exif: png_bytep,
) {
    png_warning(
        png_ptr,
        c"png_set_eXIf does not work; use png_set_eXIf_1".as_ptr(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_eXIf_1(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    num_exif: png_uint_32,
    exif: png_bytep,
) {
    let new_exif: png_bytep;

    if png_ptr.is_null()
        || info_ptr.is_null()
        || ((*png_ptr).mode & PNG_WROTE_eXIf) != 0
        || exif.is_null()
    {
        return;
    }

    new_exif = png_malloc_warn(png_ptr, num_exif as usize) as png_bytep;

    if new_exif.is_null() {
        png_warning(
            png_ptr,
            c"Insufficient memory for eXIf chunk data".as_ptr(),
        );
        return;
    }

    memcpy(new_exif as *mut u8, exif as *const u8, num_exif as usize);

    png_free_data(png_ptr, info_ptr, PNG_FREE_EXIF, 0);

    (*info_ptr).num_exif = num_exif;
    (*info_ptr).exif = new_exif;
    (*info_ptr).free_me |= PNG_FREE_EXIF;
    (*info_ptr).valid |= PNG_INFO_eXIf;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_gAMA_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    file_gamma: png_fixed_point,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    (*info_ptr).gamma = file_gamma;
    (*info_ptr).valid |= PNG_INFO_gAMA;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_gAMA(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    file_gamma: c_double,
) {
    png_set_gAMA_fixed(
        png_ptr,
        info_ptr,
        png_fixed(png_ptr, file_gamma, c"png_set_gAMA".as_ptr()),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_hIST(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    hist: png_const_uint_16p,
) {
    let mut safe_hist: [png_uint_16; PNG_MAX_PALETTE_LENGTH as usize] =
        [0; PNG_MAX_PALETTE_LENGTH as usize];
    let mut i: c_int;
    let mut hist = hist;

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
        safe_hist.as_mut_ptr() as *mut u8,
        hist as *const u8,
        ((*info_ptr).num_palette as c_uint as usize) * core::mem::size_of::<png_uint_16>(),
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
        png_warning(
            png_ptr,
            c"Insufficient memory for hIST chunk data".as_ptr(),
        );
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
        (*info_ptr).channels = (*info_ptr).channels.wrapping_add(1);
    }

    (*info_ptr).pixel_depth =
        (((*info_ptr).channels as c_int) * ((*info_ptr).bit_depth as c_int)) as png_byte;

    (*info_ptr).rowbytes = PNG_ROWBYTES((*info_ptr).pixel_depth as u32, width);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_oFFs(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    offset_x: png_int_32,
    offset_y: png_int_32,
    unit_type: c_int,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    (*info_ptr).x_offset = offset_x;
    (*info_ptr).y_offset = offset_y;
    (*info_ptr).offset_unit_type = unit_type as png_byte;
    (*info_ptr).valid |= PNG_INFO_oFFs;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_pCAL(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    purpose: png_const_charp,
    X0: png_int_32,
    X1: png_int_32,
    type_: c_int,
    nparams: c_int,
    units: png_const_charp,
    params: png_charpp,
) {
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
    if type_ < 0 || type_ > 3 {
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
        (*info_ptr).pcal_purpose as *mut u8,
        purpose as *const u8,
        length,
    );

    (*info_ptr).free_me |= PNG_FREE_PCAL;

    (*info_ptr).pcal_X0 = X0;
    (*info_ptr).pcal_X1 = X1;
    (*info_ptr).pcal_type = type_ as png_byte;
    (*info_ptr).pcal_nparams = nparams as png_byte;

    length = strlen(units) + 1;

    (*info_ptr).pcal_units = png_malloc_warn(png_ptr, length) as png_charp;

    if (*info_ptr).pcal_units.is_null() {
        png_warning(png_ptr, c"Insufficient memory for pCAL units".as_ptr());
        return;
    }

    memcpy(
        (*info_ptr).pcal_units as *mut u8,
        units as *const u8,
        length,
    );

    (*info_ptr).pcal_params = png_malloc_warn(
        png_ptr,
        ((nparams as c_uint).wrapping_add(1) as usize) * core::mem::size_of::<png_charp>(),
    ) as png_charpp;

    if (*info_ptr).pcal_params.is_null() {
        png_warning(png_ptr, c"Insufficient memory for pCAL params".as_ptr());
        return;
    }

    memset(
        (*info_ptr).pcal_params as *mut u8,
        0,
        ((nparams as c_uint).wrapping_add(1) as usize) * core::mem::size_of::<png_charp>(),
    );

    i = 0;
    while i < nparams {
        length = strlen(*params.add(i as usize)) + 1;

        *(*info_ptr).pcal_params.add(i as usize) =
            png_malloc_warn(png_ptr, length) as png_charp;

        if (*(*info_ptr).pcal_params.add(i as usize)).is_null() {
            png_warning(
                png_ptr,
                c"Insufficient memory for pCAL parameter".as_ptr(),
            );
            return;
        }

        memcpy(
            *(*info_ptr).pcal_params.add(i as usize) as *mut u8,
            *params.add(i as usize) as *const u8,
            length,
        );
        i += 1;
    }

    (*info_ptr).valid |= PNG_INFO_pCAL;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sCAL_s(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    unit: c_int,
    swidth: png_const_charp,
    sheight: png_const_charp,
) {
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
        || *swidth.add(0) as c_int == 45 /* '-' */
        || png_check_fp_string(swidth, lengthw) == 0
    {
        png_error(png_ptr, c"Invalid sCAL width".as_ptr());
    }

    if sheight.is_null()
        || {
            lengthh = strlen(sheight);
            lengthh == 0
        }
        || *sheight.add(0) as c_int == 45 /* '-' */
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
        (*info_ptr).scal_s_width as *mut u8,
        swidth as *const u8,
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
        (*info_ptr).scal_s_height as *mut u8,
        sheight as *const u8,
        lengthh,
    );

    (*info_ptr).free_me |= PNG_FREE_SCAL;
    (*info_ptr).valid |= PNG_INFO_sCAL;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sCAL(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    unit: c_int,
    width: c_double,
    height: c_double,
) {
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
            PNG_sCAL_MAX_DIGITS + 1,
            width,
            PNG_sCAL_PRECISION as c_uint,
        );
        png_ascii_from_fp(
            png_ptr,
            sheight.as_mut_ptr(),
            PNG_sCAL_MAX_DIGITS + 1,
            height,
            PNG_sCAL_PRECISION as c_uint,
        );

        png_set_sCAL_s(
            png_ptr,
            info_ptr,
            unit,
            swidth.as_ptr(),
            sheight.as_ptr(),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sCAL_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    unit: c_int,
    width: png_fixed_point,
    height: png_fixed_point,
) {
    /* Check the arguments. */
    if width <= 0 {
        png_warning(png_ptr, c"Invalid sCAL width ignored".as_ptr());
    } else if height <= 0 {
        png_warning(png_ptr, c"Invalid sCAL height ignored".as_ptr());
    } else {
        /* Convert 'width' and 'height' to ASCII. */
        let mut swidth: [c_char; PNG_sCAL_MAX_DIGITS + 1] = [0; PNG_sCAL_MAX_DIGITS + 1];
        let mut sheight: [c_char; PNG_sCAL_MAX_DIGITS + 1] = [0; PNG_sCAL_MAX_DIGITS + 1];

        png_ascii_from_fixed(png_ptr, swidth.as_mut_ptr(), PNG_sCAL_MAX_DIGITS + 1, width);
        png_ascii_from_fixed(
            png_ptr,
            sheight.as_mut_ptr(),
            PNG_sCAL_MAX_DIGITS + 1,
            height,
        );

        png_set_sCAL_s(
            png_ptr,
            info_ptr,
            unit,
            swidth.as_ptr(),
            sheight.as_ptr(),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_pHYs(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    res_x: png_uint_32,
    res_y: png_uint_32,
    unit_type: c_int,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    (*info_ptr).x_pixels_per_unit = res_x;
    (*info_ptr).y_pixels_per_unit = res_y;
    (*info_ptr).phys_unit_type = unit_type as png_byte;
    (*info_ptr).valid |= PNG_INFO_pHYs;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_PLTE(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    palette: png_const_colorp,
    num_palette: c_int,
) {
    let mut safe_palette: [png_color; PNG_MAX_PALETTE_LENGTH as usize] =
        [png_color {
            red: 0,
            green: 0,
            blue: 0,
        }; PNG_MAX_PALETTE_LENGTH as usize];
    let max_palette_length: png_uint_32;
    let mut palette = palette;

    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    max_palette_length = if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        (1i32 << (*info_ptr).bit_depth as c_int) as png_uint_32
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
            safe_palette.as_mut_ptr() as *mut u8,
            palette as *const u8,
            (num_palette as c_uint as usize) * core::mem::size_of::<png_color>(),
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
            (*info_ptr).palette as *mut u8,
            palette as *const u8,
            (num_palette as c_uint as usize) * core::mem::size_of::<png_color>(),
        );
        memcpy(
            (*png_ptr).palette as *mut u8,
            palette as *const u8,
            (num_palette as c_uint as usize) * core::mem::size_of::<png_color>(),
        );
    }

    (*info_ptr).free_me |= PNG_FREE_PLTE;
    (*info_ptr).valid |= PNG_INFO_PLTE;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sBIT(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    sig_bit: png_const_color_8p,
) {
    if png_ptr.is_null() || info_ptr.is_null() || sig_bit.is_null() {
        return;
    }

    (*info_ptr).sig_bit = *sig_bit;
    (*info_ptr).valid |= PNG_INFO_sBIT;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sRGB(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    srgb_intent: c_int,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    (*info_ptr).rendering_intent = srgb_intent;
    (*info_ptr).valid |= PNG_INFO_sRGB;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sRGB_gAMA_and_cHRM(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    srgb_intent: c_int,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    png_set_sRGB(png_ptr, info_ptr, srgb_intent);

    png_set_gAMA_fixed(png_ptr, info_ptr, PNG_GAMMA_sRGB_INVERSE);

    png_set_cHRM_fixed(
        png_ptr,
        info_ptr,
        /* color      x       y */
        /* white */ 31270, 32900,
        /* red   */ 64000, 33000,
        /* green */ 30000, 60000,
        /* blue  */ 15000, 6000,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_iCCP(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    name: png_const_charp,
    compression_type: c_int,
    profile: png_const_bytep,
    proflen: png_uint_32,
) {
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

    memcpy(new_iccp_name as *mut u8, name as *const u8, length);
    new_iccp_profile = png_malloc_warn(png_ptr, proflen as usize) as png_bytep;

    if new_iccp_profile.is_null() {
        png_free(png_ptr, new_iccp_name as png_voidp);
        png_benign_error(
            png_ptr,
            c"Insufficient memory to process iCCP profile".as_ptr(),
        );

        return;
    }

    memcpy(
        new_iccp_profile as *mut u8,
        profile as *const u8,
        proflen as usize,
    );

    png_free_data(png_ptr, info_ptr, PNG_FREE_ICCP, 0);

    (*info_ptr).iccp_proflen = proflen;
    (*info_ptr).iccp_name = new_iccp_name;
    (*info_ptr).iccp_profile = new_iccp_profile;
    (*info_ptr).free_me |= PNG_FREE_ICCP;
    (*info_ptr).valid |= PNG_INFO_iCCP;
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_text(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    text_ptr: png_const_textp,
    num_text: c_int,
) {
    let ret: c_int;
    ret = png_set_text_2(png_ptr, info_ptr, text_ptr, num_text);

    if ret != 0 {
        png_error(png_ptr, c"Insufficient memory to store text".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_text_2(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    text_ptr: png_const_textp,
    num_text: c_int,
) -> c_int {
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
            (*textp).key as *mut u8,
            (*text_ptr.add(i as usize)).key as *const u8,
            key_len,
        );
        *((*textp).key.add(key_len)) = b'\0' as c_char;

        if (*text_ptr.add(i as usize)).compression > 0 {
            (*textp).lang = (*textp).key.add(key_len + 1);
            memcpy(
                (*textp).lang as *mut u8,
                (*text_ptr.add(i as usize)).lang as *const u8,
                lang_len,
            );
            *((*textp).lang.add(lang_len)) = b'\0' as c_char;
            (*textp).lang_key = (*textp).lang.add(lang_len + 1);
            memcpy(
                (*textp).lang_key as *mut u8,
                (*text_ptr.add(i as usize)).lang_key as *const u8,
                lang_key_len,
            );
            *((*textp).lang_key.add(lang_key_len)) = b'\0' as c_char;
            (*textp).text = (*textp).lang_key.add(lang_key_len + 1);
        } else {
            (*textp).lang = core::ptr::null_mut();
            (*textp).lang_key = core::ptr::null_mut();
            (*textp).text = (*textp).key.add(key_len + 1);
        }

        if text_length != 0 {
            memcpy(
                (*textp).text as *mut u8,
                (*text_ptr.add(i as usize)).text as *const u8,
                text_length,
            );
        }

        *((*textp).text.add(text_length)) = b'\0' as c_char;

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
