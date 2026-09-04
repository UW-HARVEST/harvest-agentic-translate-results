//! Translation of c_src/src/pngset.c lines 1..1153
use crate::prelude::*;

/* #ifdef PNG_bKGD_SUPPORTED */
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

/* #ifdef PNG_cHRM_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cHRM_fixed(
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
        png_app_error(png_ptr, cstr(b"invalid cHRM XYZ\0"));
    }
}

/* #ifdef PNG_FLOATING_POINT_SUPPORTED */
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
        png_fixed(png_ptr, white_x, cstr(b"cHRM White X\0")),
        png_fixed(png_ptr, white_y, cstr(b"cHRM White Y\0")),
        png_fixed(png_ptr, red_x, cstr(b"cHRM Red X\0")),
        png_fixed(png_ptr, red_y, cstr(b"cHRM Red Y\0")),
        png_fixed(png_ptr, green_x, cstr(b"cHRM Green X\0")),
        png_fixed(png_ptr, green_y, cstr(b"cHRM Green Y\0")),
        png_fixed(png_ptr, blue_x, cstr(b"cHRM Blue X\0")),
        png_fixed(png_ptr, blue_y, cstr(b"cHRM Blue Y\0")),
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
        png_fixed(png_ptr, red_X, cstr(b"cHRM Red X\0")),
        png_fixed(png_ptr, red_Y, cstr(b"cHRM Red Y\0")),
        png_fixed(png_ptr, red_Z, cstr(b"cHRM Red Z\0")),
        png_fixed(png_ptr, green_X, cstr(b"cHRM Green X\0")),
        png_fixed(png_ptr, green_Y, cstr(b"cHRM Green Y\0")),
        png_fixed(png_ptr, green_Z, cstr(b"cHRM Green Z\0")),
        png_fixed(png_ptr, blue_X, cstr(b"cHRM Blue X\0")),
        png_fixed(png_ptr, blue_Y, cstr(b"cHRM Blue Y\0")),
        png_fixed(png_ptr, blue_Z, cstr(b"cHRM Blue Z\0")),
    );
}

/* #ifdef PNG_cICP_SUPPORTED */
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
        png_warning(png_ptr, cstr(b"Invalid cICP matrix coefficients\0"));
        return;
    }

    (*info_ptr).valid |= PNG_INFO_cICP;
}

/* #ifdef PNG_cLLI_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cLLI_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    maxCLL: png_uint_32,
    maxFALL: png_uint_32,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    /* Check the light level range: */
    if maxCLL > 0x7FFFFFFFu32 || maxFALL > 0x7FFFFFFFu32 {
        png_chunk_report(
            png_ptr,
            cstr(b"cLLI light level exceeds PNG limit\0"),
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    (*info_ptr).maxCLL = maxCLL;
    (*info_ptr).maxFALL = maxFALL;
    (*info_ptr).valid |= PNG_INFO_cLLI;
}

/* #ifdef PNG_FLOATING_POINT_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cLLI(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    maxCLL: f64,
    maxFALL: f64,
) {
    png_set_cLLI_fixed(
        png_ptr,
        info_ptr,
        png_fixed_ITU(png_ptr, maxCLL, cstr(b"png_set_cLLI(maxCLL)\0")),
        png_fixed_ITU(png_ptr, maxFALL, cstr(b"png_set_cLLI(maxFALL)\0")),
    );
}

/* #ifdef PNG_mDCV_SUPPORTED */
pub unsafe extern "C" fn png_ITU_fixed_16(
    error: *mut c_int,
    mut v: png_fixed_point,
) -> png_uint_16 {
    /* Return a safe uint16_t value scaled according to the ITU H273 rules for
     * 16-bit display chromaticities.
     */
    v /= 2; /* rounds to 0 in C: avoids insignificant arithmetic errors */
    if v > 65535 || v < 0 {
        *error = 1;
        return 0;
    }

    v as png_uint_16
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_mDCV_fixed(
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
            cstr(b"mDCV chromaticities outside representable range\0"),
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    /* Check the light level range: */
    if maxDL > 0x7FFFFFFFu32 || minDL > 0x7FFFFFFFu32 {
        png_chunk_report(
            png_ptr,
            cstr(b"mDCV display light level exceeds PNG limit\0"),
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

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

/* #ifdef PNG_FLOATING_POINT_SUPPORTED */
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
    maxDL: f64,
    minDL: f64,
) {
    png_set_mDCV_fixed(
        png_ptr,
        info_ptr,
        png_fixed(png_ptr, white_x, cstr(b"png_set_mDCV(white(x))\0")),
        png_fixed(png_ptr, white_y, cstr(b"png_set_mDCV(white(y))\0")),
        png_fixed(png_ptr, red_x, cstr(b"png_set_mDCV(red(x))\0")),
        png_fixed(png_ptr, red_y, cstr(b"png_set_mDCV(red(y))\0")),
        png_fixed(png_ptr, green_x, cstr(b"png_set_mDCV(green(x))\0")),
        png_fixed(png_ptr, green_y, cstr(b"png_set_mDCV(green(y))\0")),
        png_fixed(png_ptr, blue_x, cstr(b"png_set_mDCV(blue(x))\0")),
        png_fixed(png_ptr, blue_y, cstr(b"png_set_mDCV(blue(y))\0")),
        png_fixed_ITU(png_ptr, maxDL, cstr(b"png_set_mDCV(maxDL)\0")),
        png_fixed_ITU(png_ptr, minDL, cstr(b"png_set_mDCV(minDL)\0")),
    );
}

/* #ifdef PNG_eXIf_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_eXIf(
    png_ptr: png_const_structrp,
    _info_ptr: png_inforp,
    _exif: png_bytep,
) {
    png_warning(
        png_ptr,
        cstr(b"png_set_eXIf does not work; use png_set_eXIf_1\0"),
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
        png_warning(png_ptr, cstr(b"Insufficient memory for eXIf chunk data\0"));
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

/* #ifdef PNG_gAMA_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gAMA_fixed(
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

/* #ifdef PNG_FLOATING_POINT_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gAMA(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    file_gamma: f64,
) {
    png_set_gAMA_fixed(
        png_ptr,
        info_ptr,
        png_fixed(png_ptr, file_gamma, cstr(b"png_set_gAMA\0")),
    );
}

/* #ifdef PNG_hIST_SUPPORTED */
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
        png_warning(
            png_ptr,
            cstr(b"Invalid palette size, hIST allocation skipped\0"),
        );
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

    (*info_ptr).hist = png_malloc_warn(
        png_ptr,
        (PNG_MAX_PALETTE_LENGTH as usize * core::mem::size_of::<png_uint_16>()) as png_alloc_size_t,
    ) as png_uint_16p;

    if (*info_ptr).hist.is_null() {
        png_warning(png_ptr, cstr(b"Insufficient memory for hIST chunk data\0"));
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
pub unsafe extern "C" fn png_set_IHDR(
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
        (*info_ptr).channels += 1;
    }

    (*info_ptr).pixel_depth =
        ((*info_ptr).channels as c_int * (*info_ptr).bit_depth as c_int) as png_byte;

    (*info_ptr).rowbytes = PNG_ROWBYTES((*info_ptr).pixel_depth as usize, width as usize);
}

/* #ifdef PNG_oFFs_SUPPORTED */
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

/* #ifdef PNG_pCAL_SUPPORTED */
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
            cstr(b"Invalid pCAL equation type\0"),
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    if nparams < 0 || nparams > 255 {
        png_chunk_report(
            png_ptr,
            cstr(b"Invalid pCAL parameter count\0"),
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    /* Validate params[nparams] */
    i = 0;
    while i < nparams {
        let p = *params.add(i as usize);
        if p.is_null() || png_check_fp_string(p, strlen(p)) == 0 {
            png_chunk_report(
                png_ptr,
                cstr(b"Invalid format for pCAL parameter\0"),
                PNG_CHUNK_WRITE_ERROR,
            );
            return;
        }
        i += 1;
    }

    (*info_ptr).pcal_purpose = png_malloc_warn(png_ptr, length as png_alloc_size_t) as png_charp;

    if (*info_ptr).pcal_purpose.is_null() {
        png_chunk_report(
            png_ptr,
            cstr(b"Insufficient memory for pCAL purpose\0"),
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

    (*info_ptr).pcal_units = png_malloc_warn(png_ptr, length as png_alloc_size_t) as png_charp;

    if (*info_ptr).pcal_units.is_null() {
        png_warning(png_ptr, cstr(b"Insufficient memory for pCAL units\0"));
        return;
    }

    memcpy(
        (*info_ptr).pcal_units as *mut c_void,
        units as *const c_void,
        length,
    );

    (*info_ptr).pcal_params = png_malloc_warn(
        png_ptr,
        ((nparams as c_uint + 1) as usize * core::mem::size_of::<png_charp>()) as png_alloc_size_t,
    ) as png_charpp;

    if (*info_ptr).pcal_params.is_null() {
        png_warning(png_ptr, cstr(b"Insufficient memory for pCAL params\0"));
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
            png_malloc_warn(png_ptr, length as png_alloc_size_t) as png_charp;

        if (*(*info_ptr).pcal_params.add(i as usize)).is_null() {
            png_warning(png_ptr, cstr(b"Insufficient memory for pCAL parameter\0"));
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

/* #ifdef PNG_sCAL_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sCAL_s(
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
        png_error(png_ptr, cstr(b"Invalid sCAL unit\0"));
    }

    if swidth.is_null()
        || {
            lengthw = strlen(swidth);
            lengthw == 0
        }
        || *swidth.add(0) == 45
        || png_check_fp_string(swidth, lengthw) == 0
    {
        png_error(png_ptr, cstr(b"Invalid sCAL width\0"));
    }

    if sheight.is_null()
        || {
            lengthh = strlen(sheight);
            lengthh == 0
        }
        || *sheight.add(0) == 45
        || png_check_fp_string(sheight, lengthh) == 0
    {
        png_error(png_ptr, cstr(b"Invalid sCAL height\0"));
    }

    (*info_ptr).scal_unit = unit as png_byte;

    lengthw += 1;

    (*info_ptr).scal_s_width = png_malloc_warn(png_ptr, lengthw as png_alloc_size_t) as png_charp;

    if (*info_ptr).scal_s_width.is_null() {
        png_warning(
            png_ptr,
            cstr(b"Memory allocation failed while processing sCAL\0"),
        );
        return;
    }

    memcpy(
        (*info_ptr).scal_s_width as *mut c_void,
        swidth as *const c_void,
        lengthw,
    );

    lengthh += 1;

    (*info_ptr).scal_s_height = png_malloc_warn(png_ptr, lengthh as png_alloc_size_t) as png_charp;

    if (*info_ptr).scal_s_height.is_null() {
        png_free(png_ptr, (*info_ptr).scal_s_width as png_voidp);
        (*info_ptr).scal_s_width = core::ptr::null_mut();

        png_warning(
            png_ptr,
            cstr(b"Memory allocation failed while processing sCAL\0"),
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

/* #ifdef PNG_FLOATING_POINT_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sCAL(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    unit: c_int,
    width: f64,
    height: f64,
) {
    /* Check the arguments. */
    if width <= 0.0 {
        png_warning(png_ptr, cstr(b"Invalid sCAL width ignored\0"));
    } else if height <= 0.0 {
        png_warning(png_ptr, cstr(b"Invalid sCAL height ignored\0"));
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

/* #ifdef PNG_FIXED_POINT_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sCAL_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    unit: c_int,
    width: png_fixed_point,
    height: png_fixed_point,
) {
    /* Check the arguments. */
    if width <= 0 {
        png_warning(png_ptr, cstr(b"Invalid sCAL width ignored\0"));
    } else if height <= 0 {
        png_warning(png_ptr, cstr(b"Invalid sCAL height ignored\0"));
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

/* #ifdef PNG_pHYs_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_pHYs(
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
pub unsafe extern "C" fn png_set_PLTE(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    mut palette: png_const_colorp,
    num_palette: c_int,
) {
    let mut safe_palette: [png_color; PNG_MAX_PALETTE_LENGTH as usize] =
        [png_color::default(); PNG_MAX_PALETTE_LENGTH as usize];
    let max_palette_length: png_uint_32;

    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    max_palette_length = if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        (1u32 << (*info_ptr).bit_depth as c_int) as png_uint_32
    } else {
        PNG_MAX_PALETTE_LENGTH as png_uint_32
    };

    if num_palette < 0 || num_palette > max_palette_length as c_int {
        if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            png_error(png_ptr, cstr(b"Invalid palette length\0"));
        } else {
            png_warning(png_ptr, cstr(b"Invalid palette length\0"));
            return;
        }
    }

    if (num_palette > 0 && palette.is_null())
        || (num_palette == 0 && ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_EMPTY_PLTE) == 0)
    {
        png_error(png_ptr, cstr(b"Invalid palette\0"));
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

    png_free(png_ptr, (*png_ptr).palette as png_voidp);
    (*png_ptr).palette = core::ptr::null_mut();
    (*png_ptr).palette = png_calloc(
        png_ptr,
        (PNG_MAX_PALETTE_LENGTH as usize * core::mem::size_of::<png_color>()) as png_alloc_size_t,
    ) as png_colorp;
    (*info_ptr).palette = png_calloc(
        png_ptr,
        (PNG_MAX_PALETTE_LENGTH as usize * core::mem::size_of::<png_color>()) as png_alloc_size_t,
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

/* #ifdef PNG_sBIT_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sBIT(
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

/* #ifdef PNG_sRGB_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sRGB(
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
pub unsafe extern "C" fn png_set_sRGB_gAMA_and_cHRM(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    srgb_intent: c_int,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    png_set_sRGB(png_ptr, info_ptr, srgb_intent);

    /* #ifdef PNG_gAMA_SUPPORTED */
    png_set_gAMA_fixed(png_ptr, info_ptr, PNG_GAMMA_sRGB_INVERSE);

    /* #ifdef PNG_cHRM_SUPPORTED */
    png_set_cHRM_fixed(
        png_ptr, info_ptr, /* color      x       y */
        /* white */ 31270, 32900,
        /* red   */ 64000, 33000, /* green */ 30000, 60000, /* blue  */ 15000, 6000,
    );
}

/* #ifdef PNG_iCCP_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_iCCP(
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
        png_app_error(png_ptr, cstr(b"Invalid iCCP compression method\0"));
    }

    length = strlen(name) + 1;
    new_iccp_name = png_malloc_warn(png_ptr, length as png_alloc_size_t) as png_charp;

    if new_iccp_name.is_null() {
        png_benign_error(
            png_ptr,
            cstr(b"Insufficient memory to process iCCP chunk\0"),
        );
        return;
    }

    memcpy(new_iccp_name as *mut c_void, name as *const c_void, length);
    new_iccp_profile = png_malloc_warn(png_ptr, proflen as png_alloc_size_t) as png_bytep;

    if new_iccp_profile.is_null() {
        png_free(png_ptr, new_iccp_name as png_voidp);
        png_benign_error(
            png_ptr,
            cstr(b"Insufficient memory to process iCCP profile\0"),
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

/* #ifdef PNG_TEXT_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    text_ptr: png_const_textp,
    num_text: c_int,
) {
    let ret: c_int;
    ret = png_set_text_2(png_ptr, info_ptr, text_ptr, num_text);

    if ret != 0 {
        png_error(png_ptr, cstr(b"Insufficient memory to store text\0"));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_2(
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
     * to hold all of the incoming text_ptr objects.
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
                cstr(b"too many text chunks\0"),
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
        let textp: png_textp = &mut *(*info_ptr).text.add((*info_ptr).num_text as usize);

        if (*text_ptr.add(i as usize)).key.is_null() {
            i += 1;
            continue;
        }

        if (*text_ptr.add(i as usize)).compression < PNG_TEXT_COMPRESSION_NONE
            || (*text_ptr.add(i as usize)).compression >= PNG_TEXT_COMPRESSION_LAST
        {
            png_chunk_report(
                png_ptr,
                cstr(b"text compression mode is out of range\0"),
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
            /* #ifdef PNG_iTXt_SUPPORTED */
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
            || *(*text_ptr.add(i as usize)).text.add(0) == 0
        {
            text_length = 0;
            /* #ifdef PNG_iTXt_SUPPORTED */
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
            (key_len + text_length + lang_len + lang_key_len + 4) as png_alloc_size_t,
        ) as png_charp;

        if (*textp).key.is_null() {
            png_chunk_report(
                png_ptr,
                cstr(b"text chunk: out of memory\0"),
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

        /* #ifdef PNG_iTXt_SUPPORTED */
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
