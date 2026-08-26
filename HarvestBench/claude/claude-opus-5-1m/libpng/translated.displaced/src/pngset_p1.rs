// pngset.c - storage of image information into info struct
//
// Copyright (c) 2018-2026 Cosmin Truta
// Copyright (c) 1998-2018 Glenn Randers-Pehrson
// Copyright (c) 1996-1997 Andreas Dilger
// Copyright (c) 1995-1996 Guy Eric Schalnat, Group 42, Inc.
//
// This code is released under the libpng license.
// For conditions of distribution and use, see the disclaimer
// and license in png.h
//
// The functions here are used during reads to store data from the file
// into the info struct, and during writes to store application data
// into the info struct for writing into the file.  This abstracts the
// info struct and allows us to change the structure in the future.

use crate::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_bKGD(
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
pub unsafe extern "C" fn png_set_cHRM_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    int_white_x: png_fixed_point,
    int_white_y: png_fixed_point,
    int_red_x: png_fixed_point,
    int_red_y: png_fixed_point,
    int_green_x: png_fixed_point,
    int_green_y: png_fixed_point,
    int_blue_x: png_fixed_point,
    int_blue_y: png_fixed_point,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    (*info_ptr).cHRM.redx = int_red_x;
    (*info_ptr).cHRM.redy = int_red_y;
    (*info_ptr).cHRM.greenx = int_green_x;
    (*info_ptr).cHRM.greeny = int_green_y;
    (*info_ptr).cHRM.bluex = int_blue_x;
    (*info_ptr).cHRM.bluey = int_blue_y;
    (*info_ptr).cHRM.whitex = int_white_x;
    (*info_ptr).cHRM.whitey = int_white_y;

    (*info_ptr).valid |= PNG_INFO_cHRM;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cHRM_XYZ_fixed(
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
    let mut XYZ: png_XYZ = core::mem::zeroed();
    let mut xy: png_xy = core::mem::zeroed();

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

    if png_xy_from_XYZ(&mut xy as *mut png_xy, &XYZ as *const png_XYZ) == 0 {
        (*info_ptr).cHRM = xy;
        (*info_ptr).valid |= PNG_INFO_cHRM;
    } else {
        png_app_error(png_ptr, cstr!("invalid cHRM XYZ"));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cHRM(
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
    png_set_cHRM_fixed(
        png_ptr,
        info_ptr,
        png_fixed(png_ptr, white_x, cstr!("cHRM White X")),
        png_fixed(png_ptr, white_y, cstr!("cHRM White Y")),
        png_fixed(png_ptr, red_x, cstr!("cHRM Red X")),
        png_fixed(png_ptr, red_y, cstr!("cHRM Red Y")),
        png_fixed(png_ptr, green_x, cstr!("cHRM Green X")),
        png_fixed(png_ptr, green_y, cstr!("cHRM Green Y")),
        png_fixed(png_ptr, blue_x, cstr!("cHRM Blue X")),
        png_fixed(png_ptr, blue_y, cstr!("cHRM Blue Y")),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cHRM_XYZ(
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
    png_set_cHRM_XYZ_fixed(
        png_ptr,
        info_ptr,
        png_fixed(png_ptr, red_X, cstr!("cHRM Red X")),
        png_fixed(png_ptr, red_Y, cstr!("cHRM Red Y")),
        png_fixed(png_ptr, red_Z, cstr!("cHRM Red Z")),
        png_fixed(png_ptr, green_X, cstr!("cHRM Green X")),
        png_fixed(png_ptr, green_Y, cstr!("cHRM Green Y")),
        png_fixed(png_ptr, green_Z, cstr!("cHRM Green Z")),
        png_fixed(png_ptr, blue_X, cstr!("cHRM Blue X")),
        png_fixed(png_ptr, blue_Y, cstr!("cHRM Blue Y")),
        png_fixed(png_ptr, blue_Z, cstr!("cHRM Blue Z")),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cICP(
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
        png_warning(png_ptr, cstr!("Invalid cICP matrix coefficients"));
        return;
    }

    (*info_ptr).valid |= PNG_INFO_cICP;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cLLI_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    /* The values below are in cd/m2 (nits) and are scaled by 10,000; not
     * 100,000 as in the case of png_fixed_point.
     */
    maximum_content_light_level_scaled_by_10000: png_uint_32,
    maximum_frame_average_light_level_scaled_by_10000: png_uint_32,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    /* Check the light level range: */
    if maximum_content_light_level_scaled_by_10000 > 0x7FFFFFFF
        || maximum_frame_average_light_level_scaled_by_10000 > 0x7FFFFFFF
    {
        /* The limit is 200kcd/m2; somewhat bright but not inconceivable because
         * human vision is said to run up to 100Mcd/m2.  The sun is about 2Gcd/m2.
         *
         * The reference sRGB monitor is 80cd/m2 and the limit of PQ encoding is
         * 2kcd/m2.
         */
        png_chunk_report(
            png_ptr,
            cstr!("cLLI light level exceeds PNG limit"),
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    (*info_ptr).maxCLL = maximum_content_light_level_scaled_by_10000;
    (*info_ptr).maxFALL = maximum_frame_average_light_level_scaled_by_10000;
    (*info_ptr).valid |= PNG_INFO_cLLI;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cLLI(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    maximum_content_light_level: f64,
    maximum_frame_average_light_level: f64,
) {
    png_set_cLLI_fixed(
        png_ptr,
        info_ptr,
        png_fixed_ITU(
            png_ptr,
            maximum_content_light_level,
            cstr!("png_set_cLLI(maxCLL)"),
        ),
        png_fixed_ITU(
            png_ptr,
            maximum_frame_average_light_level,
            cstr!("png_set_cLLI(maxFALL)"),
        ),
    );
}

unsafe fn png_ITU_fixed_16(error: *mut c_int, mut v: png_fixed_point) -> png_uint_16 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_mDCV_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    int_white_x: png_fixed_point,
    int_white_y: png_fixed_point,
    int_red_x: png_fixed_point,
    int_red_y: png_fixed_point,
    int_green_x: png_fixed_point,
    int_green_y: png_fixed_point,
    int_blue_x: png_fixed_point,
    int_blue_y: png_fixed_point,
    mastering_display_maximum_luminance_scaled_by_10000: png_uint_32,
    mastering_display_minimum_luminance_scaled_by_10000: png_uint_32,
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
    rx = png_ITU_fixed_16(&mut error, int_red_x);
    ry = png_ITU_fixed_16(&mut error, int_red_y);
    gx = png_ITU_fixed_16(&mut error, int_green_x);
    gy = png_ITU_fixed_16(&mut error, int_green_y);
    bx = png_ITU_fixed_16(&mut error, int_blue_x);
    by = png_ITU_fixed_16(&mut error, int_blue_y);
    wx = png_ITU_fixed_16(&mut error, int_white_x);
    wy = png_ITU_fixed_16(&mut error, int_white_y);

    if error != 0 {
        png_chunk_report(
            png_ptr,
            cstr!("mDCV chromaticities outside representable range"),
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    /* Check the light level range: */
    if mastering_display_maximum_luminance_scaled_by_10000 > 0x7FFFFFFF
        || mastering_display_minimum_luminance_scaled_by_10000 > 0x7FFFFFFF
    {
        /* The limit is 200kcd/m2; somewhat bright but not inconceivable because
         * human vision is said to run up to 100Mcd/m2.  The sun is about 2Gcd/m2.
         *
         * The reference sRGB monitor is 80cd/m2 and the limit of PQ encoding is
         * 2kcd/m2.
         */
        png_chunk_report(
            png_ptr,
            cstr!("mDCV display light level exceeds PNG limit"),
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
    (*info_ptr).mastering_maxDL = mastering_display_maximum_luminance_scaled_by_10000;
    (*info_ptr).mastering_minDL = mastering_display_minimum_luminance_scaled_by_10000;
    (*info_ptr).valid |= PNG_INFO_mDCV;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_mDCV(
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
    mastering_display_maximum_luminance: f64,
    mastering_display_minimum_luminance: f64,
) {
    png_set_mDCV_fixed(
        png_ptr,
        info_ptr,
        png_fixed(png_ptr, white_x, cstr!("png_set_mDCV(white(x))")),
        png_fixed(png_ptr, white_y, cstr!("png_set_mDCV(white(y))")),
        png_fixed(png_ptr, red_x, cstr!("png_set_mDCV(red(x))")),
        png_fixed(png_ptr, red_y, cstr!("png_set_mDCV(red(y))")),
        png_fixed(png_ptr, green_x, cstr!("png_set_mDCV(green(x))")),
        png_fixed(png_ptr, green_y, cstr!("png_set_mDCV(green(y))")),
        png_fixed(png_ptr, blue_x, cstr!("png_set_mDCV(blue(x))")),
        png_fixed(png_ptr, blue_y, cstr!("png_set_mDCV(blue(y))")),
        png_fixed_ITU(
            png_ptr,
            mastering_display_maximum_luminance,
            cstr!("png_set_mDCV(maxDL)"),
        ),
        png_fixed_ITU(
            png_ptr,
            mastering_display_minimum_luminance,
            cstr!("png_set_mDCV(minDL)"),
        ),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_eXIf(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    exif: png_bytep,
) {
    png_warning(
        png_ptr,
        cstr!("png_set_eXIf does not work; use png_set_eXIf_1"),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_eXIf_1(
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

    new_exif = png_malloc_warn(png_ptr, num_exif as png_alloc_size_t) as png_bytep;

    if new_exif.is_null() {
        png_warning(png_ptr, cstr!("Insufficient memory for eXIf chunk data"));
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gAMA_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    int_file_gamma: png_fixed_point,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    (*info_ptr).gamma = int_file_gamma;
    (*info_ptr).valid |= PNG_INFO_gAMA;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gAMA(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    file_gamma: f64,
) {
    png_set_gAMA_fixed(
        png_ptr,
        info_ptr,
        png_fixed(png_ptr, file_gamma, cstr!("png_set_gAMA")),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_hIST(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mut hist: png_const_uint_16p,
) {
    let mut safe_hist: [png_uint_16; PNG_MAX_PALETTE_LENGTH as usize] =
        [0; PNG_MAX_PALETTE_LENGTH as usize];
    let mut i: c_int;

    if png_ptr.is_null() || info_ptr.is_null() || hist.is_null() {
        return;
    }

    if (*info_ptr).num_palette == 0 || (*info_ptr).num_palette as c_int > PNG_MAX_PALETTE_LENGTH {
        png_warning(png_ptr, cstr!("Invalid palette size, hIST allocation skipped"));

        return;
    }

    /* Snapshot the caller's hist before freeing, in case it points to
     * info_ptr->hist (getter-to-setter aliasing).
     */
    memcpy(
        safe_hist.as_mut_ptr() as *mut c_void,
        hist as *const c_void,
        (*info_ptr).num_palette as c_uint as usize * core::mem::size_of::<png_uint_16>(),
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
        png_warning(png_ptr, cstr!("Insufficient memory for hIST chunk data"));
        return;
    }

    i = 0;
    while i < (*info_ptr).num_palette as c_int {
        *(*info_ptr).hist.offset(i as isize) = *hist.offset(i as isize);
        i += 1;
    }

    (*info_ptr).free_me |= PNG_FREE_HIST;
    (*info_ptr).valid |= PNG_INFO_hIST;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_IHDR(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    width: png_uint_32,
    height: png_uint_32,
    bit_depth: c_int,
    color_type: c_int,
    interlace_method: c_int,
    compression_method: c_int,
    filter_method: c_int,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    (*info_ptr).width = width;
    (*info_ptr).height = height;
    (*info_ptr).bit_depth = bit_depth as png_byte;
    (*info_ptr).color_type = color_type as png_byte;
    (*info_ptr).compression_type = compression_method as png_byte;
    (*info_ptr).filter_type = filter_method as png_byte;
    (*info_ptr).interlace_type = interlace_method as png_byte;

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
        ((*info_ptr).channels as c_int * (*info_ptr).bit_depth as c_int) as png_byte;

    (*info_ptr).rowbytes = PNG_ROWBYTES((*info_ptr).pixel_depth as usize, width as usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_oFFs(
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
pub unsafe extern "C" fn png_set_pCAL(
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
            cstr!("Invalid pCAL equation type"),
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    if nparams < 0 || nparams > 255 {
        png_chunk_report(
            png_ptr,
            cstr!("Invalid pCAL parameter count"),
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    /* Validate params[nparams] */
    i = 0;
    while i < nparams {
        let p: png_charp = *params.offset(i as isize);

        if p.is_null() || png_check_fp_string(p as png_const_charp, strlen(p as png_const_charp)) == 0
        {
            png_chunk_report(
                png_ptr,
                cstr!("Invalid format for pCAL parameter"),
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
            cstr!("Insufficient memory for pCAL purpose"),
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
    (*info_ptr).pcal_type = type_ as png_byte;
    (*info_ptr).pcal_nparams = nparams as png_byte;

    length = strlen(units) + 1;

    (*info_ptr).pcal_units = png_malloc_warn(png_ptr, length) as png_charp;

    if (*info_ptr).pcal_units.is_null() {
        png_warning(png_ptr, cstr!("Insufficient memory for pCAL units"));
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
        png_warning(png_ptr, cstr!("Insufficient memory for pCAL params"));
        return;
    }

    memset(
        (*info_ptr).pcal_params as *mut c_void,
        0,
        ((nparams as c_uint + 1) as usize) * core::mem::size_of::<png_charp>(),
    );

    i = 0;
    while i < nparams {
        length = strlen(*params.offset(i as isize) as png_const_charp) + 1;

        *(*info_ptr).pcal_params.offset(i as isize) =
            png_malloc_warn(png_ptr, length) as png_charp;

        if (*(*info_ptr).pcal_params.offset(i as isize)).is_null() {
            png_warning(png_ptr, cstr!("Insufficient memory for pCAL parameter"));
            return;
        }

        memcpy(
            *(*info_ptr).pcal_params.offset(i as isize) as *mut c_void,
            *params.offset(i as isize) as *const c_void,
            length,
        );

        i += 1;
    }

    (*info_ptr).valid |= PNG_INFO_pCAL;
}
