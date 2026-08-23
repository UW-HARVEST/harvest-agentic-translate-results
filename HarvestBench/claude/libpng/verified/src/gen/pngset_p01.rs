/* pngset.c lines 1..434 */

/* png_set_bKGD */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_bKGD(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    background: png_const_color_16p,
) {
    if png_ptr == core::ptr::null_mut()
        || info_ptr == core::ptr::null_mut()
        || background == core::ptr::null()
    {
        return;
    }

    (*info_ptr).background = *background;
    (*info_ptr).valid |= PNG_INFO_bKGD;
}

/* png_set_cHRM_fixed */
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
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
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

/* png_set_cHRM_XYZ_fixed */
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
    let mut XYZ: png_XYZ = Default::default();
    let mut xy: png_xy = Default::default();

    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
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

    if png_xy_from_XYZ(core::ptr::addr_of_mut!(xy), core::ptr::addr_of!(XYZ)) == 0 {
        (*info_ptr).cHRM = xy;
        (*info_ptr).valid |= PNG_INFO_cHRM;
    } else {
        png_app_error(png_ptr, b"invalid cHRM XYZ\0".as_ptr() as png_const_charp);
    }
}

/* png_set_cHRM */
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
    /* The C argument list is evaluated RIGHT TO LEFT by the reference compiler,
     * and png_fixed() is a diverging call (png_fixed_error) for an out-of-range
     * value, so the *last* bad coordinate is the one that is reported.  Bind the
     * conversions in that order to reproduce it exactly.
     */
    let f_blue_y = png_fixed(
        png_ptr,
        blue_y,
        b"cHRM Blue Y\0".as_ptr() as png_const_charp,
    );
    let f_blue_x = png_fixed(
        png_ptr,
        blue_x,
        b"cHRM Blue X\0".as_ptr() as png_const_charp,
    );
    let f_green_y = png_fixed(
        png_ptr,
        green_y,
        b"cHRM Green Y\0".as_ptr() as png_const_charp,
    );
    let f_green_x = png_fixed(
        png_ptr,
        green_x,
        b"cHRM Green X\0".as_ptr() as png_const_charp,
    );
    let f_red_y = png_fixed(png_ptr, red_y, b"cHRM Red Y\0".as_ptr() as png_const_charp);
    let f_red_x = png_fixed(png_ptr, red_x, b"cHRM Red X\0".as_ptr() as png_const_charp);
    let f_white_y = png_fixed(
        png_ptr,
        white_y,
        b"cHRM White Y\0".as_ptr() as png_const_charp,
    );
    let f_white_x = png_fixed(
        png_ptr,
        white_x,
        b"cHRM White X\0".as_ptr() as png_const_charp,
    );
    png_set_cHRM_fixed(
        png_ptr, info_ptr, f_white_x, f_white_y, f_red_x, f_red_y, f_green_x, f_green_y,
        f_blue_x, f_blue_y,
    );
}

/* png_set_cHRM_XYZ */
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
    /* Right-to-left argument evaluation -- see png_set_cHRM above. */
    let f_blue_Z = png_fixed(
        png_ptr,
        blue_Z,
        b"cHRM Blue Z\0".as_ptr() as png_const_charp,
    );
    let f_blue_Y = png_fixed(
        png_ptr,
        blue_Y,
        b"cHRM Blue Y\0".as_ptr() as png_const_charp,
    );
    let f_blue_X = png_fixed(
        png_ptr,
        blue_X,
        b"cHRM Blue X\0".as_ptr() as png_const_charp,
    );
    let f_green_Z = png_fixed(
        png_ptr,
        green_Z,
        b"cHRM Green Z\0".as_ptr() as png_const_charp,
    );
    let f_green_Y = png_fixed(
        png_ptr,
        green_Y,
        b"cHRM Green Y\0".as_ptr() as png_const_charp,
    );
    let f_green_X = png_fixed(
        png_ptr,
        green_X,
        b"cHRM Green X\0".as_ptr() as png_const_charp,
    );
    let f_red_Z = png_fixed(png_ptr, red_Z, b"cHRM Red Z\0".as_ptr() as png_const_charp);
    let f_red_Y = png_fixed(png_ptr, red_Y, b"cHRM Red Y\0".as_ptr() as png_const_charp);
    let f_red_X = png_fixed(png_ptr, red_X, b"cHRM Red X\0".as_ptr() as png_const_charp);
    png_set_cHRM_XYZ_fixed(
        png_ptr, info_ptr, f_red_X, f_red_Y, f_red_Z, f_green_X, f_green_Y, f_green_Z,
        f_blue_X, f_blue_Y, f_blue_Z,
    );
}

/* png_set_cICP */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cICP(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    colour_primaries: png_byte,
    transfer_function: png_byte,
    matrix_coefficients: png_byte,
    video_full_range_flag: png_byte,
) {
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    (*info_ptr).cicp_colour_primaries = colour_primaries;
    (*info_ptr).cicp_transfer_function = transfer_function;
    (*info_ptr).cicp_matrix_coefficients = matrix_coefficients;
    (*info_ptr).cicp_video_full_range_flag = video_full_range_flag;

    if (*info_ptr).cicp_matrix_coefficients != 0 {
        png_warning(
            png_ptr,
            b"Invalid cICP matrix coefficients\0".as_ptr() as png_const_charp,
        );
        return;
    }

    (*info_ptr).valid |= PNG_INFO_cICP;
}

/* png_set_cLLI_fixed */
/* The values below are in cd/m2 (nits) and are scaled by 10,000; not
 * 100,000 as in the case of png_fixed_point.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cLLI_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    maxCLL: png_uint_32,
    maxFALL: png_uint_32,
) {
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
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
            b"cLLI light level exceeds PNG limit\0".as_ptr() as png_const_charp,
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    (*info_ptr).maxCLL = maxCLL;
    (*info_ptr).maxFALL = maxFALL;
    (*info_ptr).valid |= PNG_INFO_cLLI;
}

/* png_set_cLLI */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cLLI(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    maxCLL: f64,
    maxFALL: f64,
) {
    /* Right-to-left argument evaluation -- see png_set_cHRM above. */
    let f_maxFALL = png_fixed_ITU(
        png_ptr,
        maxFALL,
        b"png_set_cLLI(maxFALL)\0".as_ptr() as png_const_charp,
    );
    let f_maxCLL = png_fixed_ITU(
        png_ptr,
        maxCLL,
        b"png_set_cLLI(maxCLL)\0".as_ptr() as png_const_charp,
    );
    png_set_cLLI_fixed(png_ptr, info_ptr, f_maxCLL, f_maxFALL);
}

/* png_ITU_fixed_16 */
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

/* png_set_mDCV_fixed */
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

    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
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
            b"mDCV chromaticities outside representable range\0".as_ptr() as png_const_charp,
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
            b"mDCV display light level exceeds PNG limit\0".as_ptr() as png_const_charp,
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

/* png_set_mDCV */
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
    /* Right-to-left argument evaluation -- see png_set_cHRM above. */
    let f_minDL = png_fixed_ITU(
        png_ptr,
        minDL,
        b"png_set_mDCV(minDL)\0".as_ptr() as png_const_charp,
    );
    let f_maxDL = png_fixed_ITU(
        png_ptr,
        maxDL,
        b"png_set_mDCV(maxDL)\0".as_ptr() as png_const_charp,
    );
    let f_blue_y = png_fixed(
        png_ptr,
        blue_y,
        b"png_set_mDCV(blue(y))\0".as_ptr() as png_const_charp,
    );
    let f_blue_x = png_fixed(
        png_ptr,
        blue_x,
        b"png_set_mDCV(blue(x))\0".as_ptr() as png_const_charp,
    );
    let f_green_y = png_fixed(
        png_ptr,
        green_y,
        b"png_set_mDCV(green(y))\0".as_ptr() as png_const_charp,
    );
    let f_green_x = png_fixed(
        png_ptr,
        green_x,
        b"png_set_mDCV(green(x))\0".as_ptr() as png_const_charp,
    );
    let f_red_y = png_fixed(
        png_ptr,
        red_y,
        b"png_set_mDCV(red(y))\0".as_ptr() as png_const_charp,
    );
    let f_red_x = png_fixed(
        png_ptr,
        red_x,
        b"png_set_mDCV(red(x))\0".as_ptr() as png_const_charp,
    );
    let f_white_y = png_fixed(
        png_ptr,
        white_y,
        b"png_set_mDCV(white(y))\0".as_ptr() as png_const_charp,
    );
    let f_white_x = png_fixed(
        png_ptr,
        white_x,
        b"png_set_mDCV(white(x))\0".as_ptr() as png_const_charp,
    );
    png_set_mDCV_fixed(
        png_ptr, info_ptr, f_white_x, f_white_y, f_red_x, f_red_y, f_green_x, f_green_y,
        f_blue_x, f_blue_y, f_maxDL, f_minDL,
    );
}

/* png_set_eXIf */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_eXIf(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    exif: png_bytep,
) {
    png_warning(
        png_ptr,
        b"png_set_eXIf does not work; use png_set_eXIf_1\0".as_ptr() as png_const_charp,
    );
}

/* png_set_eXIf_1 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_eXIf_1(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    num_exif: png_uint_32,
    exif: png_bytep,
) {
    let new_exif: png_bytep;

    if png_ptr == core::ptr::null_mut()
        || info_ptr == core::ptr::null_mut()
        || ((*png_ptr).mode & PNG_WROTE_eXIf) != 0
        || exif == core::ptr::null_mut()
    {
        return;
    }

    new_exif = png_malloc_warn(png_ptr, num_exif as png_alloc_size_t) as png_bytep;

    if new_exif == core::ptr::null_mut() {
        png_warning(
            png_ptr,
            b"Insufficient memory for eXIf chunk data\0".as_ptr() as png_const_charp,
        );
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

/* png_set_gAMA_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gAMA_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    file_gamma: png_fixed_point,
) {
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    (*info_ptr).gamma = file_gamma;
    (*info_ptr).valid |= PNG_INFO_gAMA;
}

/* png_set_gAMA */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gAMA(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    file_gamma: f64,
) {
    png_set_gAMA_fixed(
        png_ptr,
        info_ptr,
        png_fixed(
            png_ptr,
            file_gamma,
            b"png_set_gAMA\0".as_ptr() as png_const_charp,
        ),
    );
}

/* png_set_hIST */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_hIST(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mut hist: png_const_uint_16p,
) {
    let mut safe_hist: [png_uint_16; PNG_MAX_PALETTE_LENGTH as usize] =
        [0; PNG_MAX_PALETTE_LENGTH as usize];
    let mut i: c_int;

    if png_ptr == core::ptr::null_mut()
        || info_ptr == core::ptr::null_mut()
        || hist == core::ptr::null()
    {
        return;
    }

    if (*info_ptr).num_palette == 0
        || (*info_ptr).num_palette as c_int > PNG_MAX_PALETTE_LENGTH
    {
        png_warning(
            png_ptr,
            b"Invalid palette size, hIST allocation skipped\0".as_ptr() as png_const_charp,
        );

        return;
    }

    /* Snapshot the caller's hist before freeing, in case it points to
     * info_ptr->hist (getter-to-setter aliasing).
     */
    memcpy(
        safe_hist.as_mut_ptr() as *mut c_void,
        hist as *const c_void,
        ((*info_ptr).num_palette as c_uint as usize) * core::mem::size_of::<png_uint_16>(),
    );
    hist = safe_hist.as_ptr();

    png_free_data(png_ptr, info_ptr, PNG_FREE_HIST, 0);

    /* Changed from info->num_palette to PNG_MAX_PALETTE_LENGTH in
     * version 1.2.1
     */
    (*info_ptr).hist = png_malloc_warn(
        png_ptr,
        (PNG_MAX_PALETTE_LENGTH as png_alloc_size_t) * core::mem::size_of::<png_uint_16>(),
    ) as png_uint_16p;

    if (*info_ptr).hist == core::ptr::null_mut() {
        png_warning(
            png_ptr,
            b"Insufficient memory for hIST chunk data\0".as_ptr() as png_const_charp,
        );
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
