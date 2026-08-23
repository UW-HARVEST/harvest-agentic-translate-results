/* pngget.c lines 934..1369 */

/* png_get_IHDR */
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
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
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

/* png_get_oFFs */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_oFFs(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    offset_x: *mut png_int_32,
    offset_y: *mut png_int_32,
    unit_type: *mut c_int,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
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

/* png_get_pCAL */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pCAL(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    purpose: png_charpp,
    X0: *mut png_int_32,
    X1: *mut png_int_32,
    type_: *mut c_int,
    nparams: *mut c_int,
    units: png_charpp,
    params: *mut png_charpp,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_pCAL) != 0
        && purpose != core::ptr::null_mut()
        && X0 != core::ptr::null_mut()
        && X1 != core::ptr::null_mut()
        && type_ != core::ptr::null_mut()
        && nparams != core::ptr::null_mut()
        && units != core::ptr::null_mut()
        && params != core::ptr::null_mut()
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

/* png_get_sCAL_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sCAL_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    unit: *mut c_int,
    width: *mut png_fixed_point,
    height: *mut png_fixed_point,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_sCAL) != 0
    {
        *unit = (*info_ptr).scal_unit as c_int;
        /*TODO: make this work without FP support; the API is currently eliminated
         * if neither floating point APIs nor internal floating point arithmetic
         * are enabled.
         */
        *width = png_fixed(
            png_ptr,
            atof((*info_ptr).scal_s_width),
            b"sCAL width\0".as_ptr() as png_const_charp,
        );
        *height = png_fixed(
            png_ptr,
            atof((*info_ptr).scal_s_height),
            b"sCAL height\0".as_ptr() as png_const_charp,
        );
        return PNG_INFO_sCAL;
    }

    0
}

/* png_get_sCAL */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sCAL(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    unit: *mut c_int,
    width: *mut f64,
    height: *mut f64,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_sCAL) != 0
    {
        *unit = (*info_ptr).scal_unit as c_int;
        *width = atof((*info_ptr).scal_s_width);
        *height = atof((*info_ptr).scal_s_height);
        return PNG_INFO_sCAL;
    }

    0
}

/* png_get_sCAL_s */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sCAL_s(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    unit: *mut c_int,
    width: png_charpp,
    height: png_charpp,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_sCAL) != 0
    {
        *unit = (*info_ptr).scal_unit as c_int;
        *width = (*info_ptr).scal_s_width;
        *height = (*info_ptr).scal_s_height;
        return PNG_INFO_sCAL;
    }

    0
}

/* png_get_pHYs */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pHYs(
    png_ptr: png_const_structrp,
    info_ptr: png_const_inforp,
    res_x: *mut png_uint_32,
    res_y: *mut png_uint_32,
    unit_type: *mut c_int,
) -> png_uint_32 {
    let mut retval: png_uint_32 = 0;

    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
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

/* png_get_PLTE */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_PLTE(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    palette: *mut png_colorp,
    num_palette: *mut c_int,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
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

/* png_get_sBIT */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sBIT(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    sig_bit: *mut png_color_8p,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_sBIT) != 0
        && sig_bit != core::ptr::null_mut()
    {
        *sig_bit = core::ptr::addr_of_mut!((*info_ptr).sig_bit);
        return PNG_INFO_sBIT;
    }

    0
}

/* png_get_text */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_text(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    text_ptr: *mut png_textp,
    num_text: *mut c_int,
) -> c_int {
    if png_ptr != core::ptr::null_mut()
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

/* png_get_tIME */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_tIME(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mod_time: *mut png_timep,
) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_tIME) != 0
        && mod_time != core::ptr::null_mut()
    {
        *mod_time = core::ptr::addr_of_mut!((*info_ptr).mod_time);
        return PNG_INFO_tIME;
    }

    0
}

/* png_get_tRNS */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_tRNS(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    trans_alpha: *mut png_bytep,
    num_trans: *mut c_int,
    trans_color: *mut png_color_16p,
) -> png_uint_32 {
    let mut retval: png_uint_32 = 0;

    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && ((*info_ptr).valid & PNG_INFO_tRNS) != 0
    {
        if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            if trans_alpha != core::ptr::null_mut() {
                *trans_alpha = (*info_ptr).trans_alpha;
                retval |= PNG_INFO_tRNS;
            }

            if trans_color != core::ptr::null_mut() {
                *trans_color = core::ptr::addr_of_mut!((*info_ptr).trans_color);
            }
        } else
        /* if (info_ptr->color_type != PNG_COLOR_TYPE_PALETTE) */
        {
            if trans_color != core::ptr::null_mut() {
                *trans_color = core::ptr::addr_of_mut!((*info_ptr).trans_color);
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

/* png_get_unknown_chunks */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_unknown_chunks(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    unknowns: png_unknown_chunkpp,
) -> c_int {
    if png_ptr != core::ptr::null_mut()
        && info_ptr != core::ptr::null_mut()
        && unknowns != core::ptr::null_mut()
    {
        *unknowns = (*info_ptr).unknown_chunks;
        return (*info_ptr).unknown_chunks_num;
    }

    0
}

/* png_get_rgb_to_gray_status */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_rgb_to_gray_status(png_ptr: png_const_structrp) -> png_byte {
    (if png_ptr != core::ptr::null_mut() {
        (*png_ptr).rgb_to_gray_status
    } else {
        0
    }) as png_byte
}

/* png_get_user_chunk_ptr */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_user_chunk_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr != core::ptr::null_mut() {
        (*png_ptr).user_chunk_ptr
    } else {
        core::ptr::null_mut()
    }
}

/* png_get_compression_buffer_size */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_compression_buffer_size(png_ptr: png_const_structrp) -> usize {
    if png_ptr == core::ptr::null_mut() {
        return 0;
    }

    if ((*png_ptr).mode & PNG_IS_READ_STRUCT) != 0 {
        return (*png_ptr).IDAT_read_size as usize;
    } else {
        return (*png_ptr).zbuffer_size as usize;
    }
}

/* These functions were added to libpng 1.2.6 and were enabled
 * by default in libpng-1.4.0 */
/* png_get_user_width_max */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_user_width_max(png_ptr: png_const_structrp) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut() {
        (*png_ptr).user_width_max
    } else {
        0
    }
}

/* png_get_user_height_max */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_user_height_max(png_ptr: png_const_structrp) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut() {
        (*png_ptr).user_height_max
    } else {
        0
    }
}

/* This function was added to libpng 1.4.0 */
/* png_get_chunk_cache_max */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_chunk_cache_max(png_ptr: png_const_structrp) -> png_uint_32 {
    if png_ptr != core::ptr::null_mut() {
        (*png_ptr).user_chunk_cache_max
    } else {
        0
    }
}

/* This function was added to libpng 1.4.1 */
/* png_get_chunk_malloc_max */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_chunk_malloc_max(png_ptr: png_const_structrp) -> png_alloc_size_t {
    if png_ptr != core::ptr::null_mut() {
        (*png_ptr).user_chunk_malloc_max
    } else {
        0
    }
}

/* These functions were added to libpng 1.4.0 */
/* png_get_io_state */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_io_state(png_ptr: png_const_structrp) -> png_uint_32 {
    (*png_ptr).io_state
}

/* png_get_io_chunk_type */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_io_chunk_type(png_ptr: png_const_structrp) -> png_uint_32 {
    (*png_ptr).chunk_name
}

/* png_get_palette_max */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_palette_max(
    png_ptr: png_const_structp,
    info_ptr: png_const_infop,
) -> c_int {
    if png_ptr != core::ptr::null_mut() && info_ptr != core::ptr::null_mut() {
        return (*png_ptr).num_palette_max;
    }

    -1
}
