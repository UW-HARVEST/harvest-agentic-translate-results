// pngset.c - storage of image information into info struct
//
// This file is part of the translation of libpng.  It contains the functions
// png_set_sCAL_s() .. png_set_text_2().
//
// The functions here are used during reads to store data from the file
// into the info struct, and during writes to store application data
// into the info struct for writing into the file.  This abstracts the
// info struct and allows limited backwards compatibility when adding
// functionality.

use crate::*;

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
        png_error(png_ptr, cstr!("Invalid sCAL unit"));
    }

    if swidth.is_null()
        || ({
            lengthw = strlen(swidth);
            lengthw
        }) == 0
        || *swidth.offset(0) == 45 /* '-' */
        || png_check_fp_string(swidth, lengthw) == 0
    {
        png_error(png_ptr, cstr!("Invalid sCAL width"));
    }

    if sheight.is_null()
        || ({
            lengthh = strlen(sheight);
            lengthh
        }) == 0
        || *sheight.offset(0) == 45 /* '-' */
        || png_check_fp_string(sheight, lengthh) == 0
    {
        png_error(png_ptr, cstr!("Invalid sCAL height"));
    }

    (*info_ptr).scal_unit = unit as png_byte;

    lengthw += 1;

    (*info_ptr).scal_s_width = png_malloc_warn(png_ptr, lengthw) as png_charp;

    if (*info_ptr).scal_s_width.is_null() {
        png_warning(
            png_ptr,
            cstr!("Memory allocation failed while processing sCAL"),
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
            cstr!("Memory allocation failed while processing sCAL"),
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
        png_warning(png_ptr, cstr!("Invalid sCAL width ignored"));
    } else if height <= 0.0 {
        png_warning(png_ptr, cstr!("Invalid sCAL height ignored"));
    } else {
        /* Convert 'width' and 'height' to ASCII. */
        let mut swidth: [c_char; PNG_sCAL_MAX_DIGITS + 1] = [0; PNG_sCAL_MAX_DIGITS + 1];
        let mut sheight: [c_char; PNG_sCAL_MAX_DIGITS + 1] = [0; PNG_sCAL_MAX_DIGITS + 1];

        png_ascii_from_fp(
            png_ptr,
            swidth.as_mut_ptr(),
            core::mem::size_of::<[c_char; PNG_sCAL_MAX_DIGITS + 1]>(),
            width,
            PNG_sCAL_PRECISION as c_uint,
        );
        png_ascii_from_fp(
            png_ptr,
            sheight.as_mut_ptr(),
            core::mem::size_of::<[c_char; PNG_sCAL_MAX_DIGITS + 1]>(),
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
pub unsafe extern "C" fn png_set_sCAL_fixed(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    unit: c_int,
    width: png_fixed_point,
    height: png_fixed_point,
) {
    /* Check the arguments. */
    if width <= 0 {
        png_warning(png_ptr, cstr!("Invalid sCAL width ignored"));
    } else if height <= 0 {
        png_warning(png_ptr, cstr!("Invalid sCAL height ignored"));
    } else {
        /* Convert 'width' and 'height' to ASCII. */
        let mut swidth: [c_char; PNG_sCAL_MAX_DIGITS + 1] = [0; PNG_sCAL_MAX_DIGITS + 1];
        let mut sheight: [c_char; PNG_sCAL_MAX_DIGITS + 1] = [0; PNG_sCAL_MAX_DIGITS + 1];

        png_ascii_from_fixed(
            png_ptr,
            swidth.as_mut_ptr(),
            core::mem::size_of::<[c_char; PNG_sCAL_MAX_DIGITS + 1]>(),
            width,
        );
        png_ascii_from_fixed(
            png_ptr,
            sheight.as_mut_ptr(),
            core::mem::size_of::<[c_char; PNG_sCAL_MAX_DIGITS + 1]>(),
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
    palette: png_const_colorp,
    num_palette: c_int,
) {
    let mut palette: png_const_colorp = palette;
    let mut safe_palette: [png_color; PNG_MAX_PALETTE_LENGTH as usize] = [png_color {
        red: 0,
        green: 0,
        blue: 0,
    }; PNG_MAX_PALETTE_LENGTH as usize];
    let max_palette_length: png_uint_32;

    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    max_palette_length = if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        (1 << (*info_ptr).bit_depth as c_int) as png_uint_32
    } else {
        PNG_MAX_PALETTE_LENGTH as png_uint_32
    };

    if num_palette < 0 || num_palette > max_palette_length as c_int {
        if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            png_error(png_ptr, cstr!("Invalid palette length"));
        } else {
            png_warning(png_ptr, cstr!("Invalid palette length"));

            return;
        }
    }

    if (num_palette > 0 && palette.is_null())
        || (num_palette == 0
            && ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_EMPTY_PLTE) == 0)
    {
        png_error(png_ptr, cstr!("Invalid palette"));
    }

    /* Snapshot the caller's palette before freeing, in case it points to
     * info_ptr->palette (getter-to-setter aliasing).
     */
    if num_palette > 0 {
        memcpy(
            safe_palette.as_mut_ptr() as *mut c_void,
            palette as *const c_void,
            (num_palette as c_uint) as usize * core::mem::size_of::<png_color>(),
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
            (num_palette as c_uint) as usize * core::mem::size_of::<png_color>(),
        );
        memcpy(
            (*png_ptr).palette as *mut c_void,
            palette as *const c_void,
            (num_palette as c_uint) as usize * core::mem::size_of::<png_color>(),
        );
    }

    (*info_ptr).free_me |= PNG_FREE_PLTE;
    (*info_ptr).valid |= PNG_INFO_PLTE;
}

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
        png_app_error(png_ptr, cstr!("Invalid iCCP compression method"));
    }

    length = strlen(name) + 1;
    new_iccp_name = png_malloc_warn(png_ptr, length) as png_charp;

    if new_iccp_name.is_null() {
        png_benign_error(png_ptr, cstr!("Insufficient memory to process iCCP chunk"));

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
        png_benign_error(png_ptr, cstr!("Insufficient memory to process iCCP profile"));

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
        png_error(png_ptr, cstr!("Insufficient memory to store text"));
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
                cstr!("too many text chunks"),
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
        let textp: png_textp = (*info_ptr).text.offset((*info_ptr).num_text as isize);
        /* &text_ptr[i] */
        let tp: png_const_textp = text_ptr.offset(i as isize);

        if (*tp).key.is_null() {
            i += 1;
            continue;
        }

        if (*tp).compression < PNG_TEXT_COMPRESSION_NONE
            || (*tp).compression >= PNG_TEXT_COMPRESSION_LAST
        {
            png_chunk_report(
                png_ptr,
                cstr!("text compression mode is out of range"),
                PNG_CHUNK_WRITE_ERROR,
            );
            i += 1;
            continue;
        }

        key_len = strlen((*tp).key);

        if (*tp).compression <= 0 {
            lang_len = 0;
            lang_key_len = 0;
        } else {
            /* Set iTXt data */

            if !(*tp).lang.is_null() {
                lang_len = strlen((*tp).lang);
            } else {
                lang_len = 0;
            }

            if !(*tp).lang_key.is_null() {
                lang_key_len = strlen((*tp).lang_key);
            } else {
                lang_key_len = 0;
            }
        }

        if (*tp).text.is_null() || *(*tp).text.offset(0) == 0 {
            text_length = 0;

            if (*tp).compression > 0 {
                (*textp).compression = PNG_ITXT_COMPRESSION_NONE;
            } else {
                (*textp).compression = PNG_TEXT_COMPRESSION_NONE;
            }
        } else {
            text_length = strlen((*tp).text);
            (*textp).compression = (*tp).compression;
        }

        (*textp).key = png_malloc_base(
            png_ptr,
            key_len + text_length + lang_len + lang_key_len + 4,
        ) as png_charp;

        if (*textp).key.is_null() {
            png_chunk_report(
                png_ptr,
                cstr!("text chunk: out of memory"),
                PNG_CHUNK_WRITE_ERROR,
            );
            png_free(png_ptr, old_text as png_voidp);

            return 1;
        }

        memcpy(
            (*textp).key as *mut c_void,
            (*tp).key as *const c_void,
            key_len,
        );
        *(*textp).key.add(key_len) = 0; /* '\0' */

        if (*tp).compression > 0 {
            (*textp).lang = (*textp).key.add(key_len + 1);
            memcpy(
                (*textp).lang as *mut c_void,
                (*tp).lang as *const c_void,
                lang_len,
            );
            *(*textp).lang.add(lang_len) = 0; /* '\0' */
            (*textp).lang_key = (*textp).lang.add(lang_len + 1);
            memcpy(
                (*textp).lang_key as *mut c_void,
                (*tp).lang_key as *const c_void,
                lang_key_len,
            );
            *(*textp).lang_key.add(lang_key_len) = 0; /* '\0' */
            (*textp).text = (*textp).lang_key.add(lang_key_len + 1);
        } else {
            (*textp).lang = core::ptr::null_mut();
            (*textp).lang_key = core::ptr::null_mut();
            (*textp).text = (*textp).key.add(key_len + 1);
        }

        if text_length != 0 {
            memcpy(
                (*textp).text as *mut c_void,
                (*tp).text as *const c_void,
                text_length,
            );
        }

        *(*textp).text.add(text_length) = 0; /* '\0' */

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

    return 0;
}
