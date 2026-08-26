/* pngset.c lines 435..748 */

/* png_set_IHDR */
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
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
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

    (*info_ptr).rowbytes = PNG_ROWBYTES((*info_ptr).pixel_depth as usize, width as usize);
}

/* png_set_oFFs */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_oFFs(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    offset_x: png_int_32,
    offset_y: png_int_32,
    unit_type: c_int,
) {
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    (*info_ptr).x_offset = offset_x;
    (*info_ptr).y_offset = offset_y;
    (*info_ptr).offset_unit_type = unit_type as png_byte;
    (*info_ptr).valid |= PNG_INFO_oFFs;
}

/* png_set_pCAL */
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

    if png_ptr == core::ptr::null_mut()
        || info_ptr == core::ptr::null_mut()
        || purpose == core::ptr::null()
        || units == core::ptr::null()
        || (nparams > 0 && params == core::ptr::null_mut())
    {
        return;
    }

    length = strlen(purpose) + 1;

    /* TODO: validate format of calibration name and unit name */

    /* Check that the type matches the specification. */
    if type_ < 0 || type_ > 3 {
        png_chunk_report(
            png_ptr,
            b"Invalid pCAL equation type\0".as_ptr() as png_const_charp,
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    if nparams < 0 || nparams > 255 {
        png_chunk_report(
            png_ptr,
            b"Invalid pCAL parameter count\0".as_ptr() as png_const_charp,
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }

    /* Validate params[nparams] */
    i = 0;
    while i < nparams {
        if (*params.offset(i as isize)) == core::ptr::null_mut()
            || png_check_fp_string(
                *params.offset(i as isize),
                strlen(*params.offset(i as isize)),
            ) == 0
        {
            png_chunk_report(
                png_ptr,
                b"Invalid format for pCAL parameter\0".as_ptr() as png_const_charp,
                PNG_CHUNK_WRITE_ERROR,
            );
            return;
        }
        i += 1;
    }

    (*info_ptr).pcal_purpose = png_malloc_warn(png_ptr, length) as png_charp;

    if (*info_ptr).pcal_purpose == core::ptr::null_mut() {
        png_chunk_report(
            png_ptr,
            b"Insufficient memory for pCAL purpose\0".as_ptr() as png_const_charp,
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

    if (*info_ptr).pcal_units == core::ptr::null_mut() {
        png_warning(
            png_ptr,
            b"Insufficient memory for pCAL units\0".as_ptr() as png_const_charp,
        );
        return;
    }

    memcpy(
        (*info_ptr).pcal_units as *mut c_void,
        units as *const c_void,
        length,
    );

    (*info_ptr).pcal_params = png_malloc_warn(
        png_ptr,
        (((nparams as c_uint).wrapping_add(1) as usize)
            .wrapping_mul(core::mem::size_of::<png_charp>())) as png_alloc_size_t,
    ) as png_charpp;

    if (*info_ptr).pcal_params == core::ptr::null_mut() {
        png_warning(
            png_ptr,
            b"Insufficient memory for pCAL params\0".as_ptr() as png_const_charp,
        );
        return;
    }

    memset(
        (*info_ptr).pcal_params as *mut c_void,
        0,
        ((nparams as c_uint).wrapping_add(1) as usize)
            .wrapping_mul(core::mem::size_of::<png_charp>()),
    );

    i = 0;
    while i < nparams {
        length = strlen(*params.offset(i as isize)) + 1;

        *(*info_ptr).pcal_params.offset(i as isize) =
            png_malloc_warn(png_ptr, length) as png_charp;

        if (*(*info_ptr).pcal_params.offset(i as isize)) == core::ptr::null_mut() {
            png_warning(
                png_ptr,
                b"Insufficient memory for pCAL parameter\0".as_ptr() as png_const_charp,
            );
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

/* png_set_sCAL_s */
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

    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    /* Double check the unit (should never get here with an invalid
     * unit unless this is an API call.)
     */
    if unit != 1 && unit != 2 {
        png_error(png_ptr, b"Invalid sCAL unit\0".as_ptr() as png_const_charp);
    }

    if swidth == core::ptr::null() {
        png_error(png_ptr, b"Invalid sCAL width\0".as_ptr() as png_const_charp);
    }

    lengthw = strlen(swidth);

    if lengthw == 0
        || *swidth == 45
        /* '-' */
        || png_check_fp_string(swidth, lengthw) == 0
    {
        png_error(png_ptr, b"Invalid sCAL width\0".as_ptr() as png_const_charp);
    }

    if sheight == core::ptr::null() {
        png_error(png_ptr, b"Invalid sCAL height\0".as_ptr() as png_const_charp);
    }

    lengthh = strlen(sheight);

    if lengthh == 0
        || *sheight == 45
        /* '-' */
        || png_check_fp_string(sheight, lengthh) == 0
    {
        png_error(png_ptr, b"Invalid sCAL height\0".as_ptr() as png_const_charp);
    }

    (*info_ptr).scal_unit = unit as png_byte;

    lengthw += 1;

    (*info_ptr).scal_s_width = png_malloc_warn(png_ptr, lengthw) as png_charp;

    if (*info_ptr).scal_s_width == core::ptr::null_mut() {
        png_warning(
            png_ptr,
            b"Memory allocation failed while processing sCAL\0".as_ptr() as png_const_charp,
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

    if (*info_ptr).scal_s_height == core::ptr::null_mut() {
        png_free(png_ptr, (*info_ptr).scal_s_width as png_voidp);
        (*info_ptr).scal_s_width = core::ptr::null_mut();

        png_warning(
            png_ptr,
            b"Memory allocation failed while processing sCAL\0".as_ptr() as png_const_charp,
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

/* png_set_sCAL */
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
        png_warning(
            png_ptr,
            b"Invalid sCAL width ignored\0".as_ptr() as png_const_charp,
        );
    } else if height <= 0.0 {
        png_warning(
            png_ptr,
            b"Invalid sCAL height ignored\0".as_ptr() as png_const_charp,
        );
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
            swidth.as_ptr() as png_const_charp,
            sheight.as_ptr() as png_const_charp,
        );
    }
}

/* png_set_sCAL_fixed */
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
        png_warning(
            png_ptr,
            b"Invalid sCAL width ignored\0".as_ptr() as png_const_charp,
        );
    } else if height <= 0 {
        png_warning(
            png_ptr,
            b"Invalid sCAL height ignored\0".as_ptr() as png_const_charp,
        );
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
            swidth.as_ptr() as png_const_charp,
            sheight.as_ptr() as png_const_charp,
        );
    }
}

/* png_set_pHYs */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_pHYs(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    res_x: png_uint_32,
    res_y: png_uint_32,
    unit_type: c_int,
) {
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    (*info_ptr).x_pixels_per_unit = res_x;
    (*info_ptr).y_pixels_per_unit = res_y;
    (*info_ptr).phys_unit_type = unit_type as png_byte;
    (*info_ptr).valid |= PNG_INFO_pHYs;
}
