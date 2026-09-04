//! Translation of c_src/src/pngwrite.c lines 1..1533
use crate::prelude::*;

/* PNG_WRITE_UNKNOWN_CHUNKS_SUPPORTED */
/* Write out all the unknown chunks for the current given location */
pub unsafe extern "C" fn write_unknown_chunks(
    png_ptr: png_structrp,
    info_ptr: png_const_inforp,
    r#where: c_uint,
) {
    if (*info_ptr).unknown_chunks_num != 0 {
        let mut up: png_const_unknown_chunkp;

        up = (*info_ptr).unknown_chunks;
        while up
            < (*info_ptr)
                .unknown_chunks
                .add((*info_ptr).unknown_chunks_num as usize)
        {
            if ((*up).location as c_uint & r#where) != 0 {
                /* If per-chunk unknown chunk handling is enabled use it, otherwise
                 * just write the chunks the application has set.
                 */
                /* PNG_SET_UNKNOWN_CHUNKS_SUPPORTED */
                let keep: c_int = png_handle_as_unknown(png_ptr, (*up).name.as_ptr());

                if keep != PNG_HANDLE_CHUNK_NEVER
                    && (((*up).name[3] as c_int & 0x20) != 0
                        || keep == PNG_HANDLE_CHUNK_ALWAYS
                        || (keep == PNG_HANDLE_CHUNK_AS_DEFAULT
                            && (*png_ptr).unknown_default == PNG_HANDLE_CHUNK_ALWAYS))
                {
                    /* TODO: review, what is wrong with a zero length unknown chunk? */
                    if (*up).size == 0 {
                        png_warning(png_ptr, cstr(b"Writing zero-length unknown chunk\0"));
                    }

                    png_write_chunk(png_ptr, (*up).name.as_ptr(), (*up).data, (*up).size);
                }
            }
            up = up.add(1);
        }
    }
}

/* Writes all the PNG information.  This is the suggested way to use the
 * library.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_info_before_PLTE(
    png_ptr: png_structrp,
    info_ptr: png_const_inforp,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    if ((*png_ptr).mode & PNG_WROTE_INFO_BEFORE_PLTE) == 0 {
        /* Write PNG signature */
        png_write_sig(png_ptr);

        /* PNG_MNG_FEATURES_SUPPORTED */
        if ((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) != 0 && (*png_ptr).mng_features_permitted != 0
        {
            png_warning(
                png_ptr,
                cstr(b"MNG features are not allowed in a PNG datastream\0"),
            );
            (*png_ptr).mng_features_permitted = 0;
        }

        /* Write IHDR information. */
        png_write_IHDR(
            png_ptr,
            (*info_ptr).width,
            (*info_ptr).height,
            (*info_ptr).bit_depth as c_int,
            (*info_ptr).color_type as c_int,
            (*info_ptr).compression_type as c_int,
            (*info_ptr).filter_type as c_int,
            /* PNG_WRITE_INTERLACING_SUPPORTED */
            (*info_ptr).interlace_type as c_int,
        );

        /* PNG_WRITE_UNKNOWN_CHUNKS_SUPPORTED */
        write_unknown_chunks(png_ptr, info_ptr, PNG_HAVE_IHDR);

        /* PNG_WRITE_sBIT_SUPPORTED */
        if ((*info_ptr).valid & PNG_INFO_sBIT) != 0 {
            png_write_sBIT(
                png_ptr,
                &(*info_ptr).sig_bit,
                (*info_ptr).color_type as c_int,
            );
        }

        /* PNG_WRITE_cLLI_SUPPORTED */
        if ((*info_ptr).valid & PNG_INFO_cLLI) != 0 {
            png_write_cLLI_fixed(png_ptr, (*info_ptr).maxCLL, (*info_ptr).maxFALL);
        }

        /* PNG_WRITE_mDCV_SUPPORTED */
        if ((*info_ptr).valid & PNG_INFO_mDCV) != 0 {
            png_write_mDCV_fixed(
                png_ptr,
                (*info_ptr).mastering_red_x,
                (*info_ptr).mastering_red_y,
                (*info_ptr).mastering_green_x,
                (*info_ptr).mastering_green_y,
                (*info_ptr).mastering_blue_x,
                (*info_ptr).mastering_blue_y,
                (*info_ptr).mastering_white_x,
                (*info_ptr).mastering_white_y,
                (*info_ptr).mastering_maxDL,
                (*info_ptr).mastering_minDL,
            );
        }

        /* PNG_WRITE_cICP_SUPPORTED - Priority 4 */
        if ((*info_ptr).valid & PNG_INFO_cICP) != 0 {
            png_write_cICP(
                png_ptr,
                (*info_ptr).cicp_colour_primaries,
                (*info_ptr).cicp_transfer_function,
                (*info_ptr).cicp_matrix_coefficients,
                (*info_ptr).cicp_video_full_range_flag,
            );
        }

        /* PNG_WRITE_iCCP_SUPPORTED - Priority 3 */
        if ((*info_ptr).valid & PNG_INFO_iCCP) != 0 {
            png_write_iCCP(
                png_ptr,
                (*info_ptr).iccp_name,
                (*info_ptr).iccp_profile,
                (*info_ptr).iccp_proflen,
            );
        }

        /* PNG_WRITE_sRGB_SUPPORTED - Priority 2 */
        if ((*info_ptr).valid & PNG_INFO_sRGB) != 0 {
            png_write_sRGB(png_ptr, (*info_ptr).rendering_intent);
        }

        /* PNG_WRITE_gAMA_SUPPORTED - Priority 1 */
        if ((*info_ptr).valid & PNG_INFO_gAMA) != 0 {
            png_write_gAMA_fixed(png_ptr, (*info_ptr).gamma);
        }

        /* PNG_WRITE_cHRM_SUPPORTED - Also priority 1 */
        if ((*info_ptr).valid & PNG_INFO_cHRM) != 0 {
            png_write_cHRM_fixed(png_ptr, &(*info_ptr).cHRM);
        }

        (*png_ptr).mode |= PNG_WROTE_INFO_BEFORE_PLTE;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_info(png_ptr: png_structrp, info_ptr: png_const_inforp) {
    /* PNG_WRITE_TEXT_SUPPORTED || PNG_WRITE_sPLT_SUPPORTED */
    let mut i: c_int;

    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    png_write_info_before_PLTE(png_ptr, info_ptr);

    if ((*info_ptr).valid & PNG_INFO_PLTE) != 0 {
        png_write_PLTE(
            png_ptr,
            (*info_ptr).palette,
            (*info_ptr).num_palette as png_uint_32,
        );
    } else if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        png_error(
            png_ptr,
            cstr(b"Valid palette required for paletted images\0"),
        );
    }

    /* PNG_WRITE_tRNS_SUPPORTED */
    if ((*info_ptr).valid & PNG_INFO_tRNS) != 0 {
        /* PNG_WRITE_INVERT_ALPHA_SUPPORTED */
        /* Invert the alpha channel (in tRNS) */
        if ((*png_ptr).transformations & PNG_INVERT_ALPHA) != 0
            && (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
        {
            let mut j: c_int;
            let mut jend: c_int;

            jend = (*info_ptr).num_trans as c_int;
            if jend > PNG_MAX_PALETTE_LENGTH {
                jend = PNG_MAX_PALETTE_LENGTH;
            }

            j = 0;
            while j < jend {
                *(*info_ptr).trans_alpha.add(j as usize) =
                    (255 - *(*info_ptr).trans_alpha.add(j as usize) as c_int) as png_byte;
                j += 1;
            }
        }

        png_write_tRNS(
            png_ptr,
            (*info_ptr).trans_alpha,
            &(*info_ptr).trans_color,
            (*info_ptr).num_trans as c_int,
            (*info_ptr).color_type as c_int,
        );
    }

    /* PNG_WRITE_bKGD_SUPPORTED */
    if ((*info_ptr).valid & PNG_INFO_bKGD) != 0 {
        png_write_bKGD(
            png_ptr,
            &(*info_ptr).background,
            (*info_ptr).color_type as c_int,
        );
    }

    /* PNG_WRITE_eXIf_SUPPORTED */
    if ((*info_ptr).valid & PNG_INFO_eXIf) != 0 {
        png_write_eXIf(png_ptr, (*info_ptr).exif, (*info_ptr).num_exif as c_int);
        (*png_ptr).mode |= PNG_WROTE_eXIf;
    }

    /* PNG_WRITE_hIST_SUPPORTED */
    if ((*info_ptr).valid & PNG_INFO_hIST) != 0 {
        png_write_hIST(png_ptr, (*info_ptr).hist, (*info_ptr).num_palette as c_int);
    }

    /* PNG_WRITE_oFFs_SUPPORTED */
    if ((*info_ptr).valid & PNG_INFO_oFFs) != 0 {
        png_write_oFFs(
            png_ptr,
            (*info_ptr).x_offset,
            (*info_ptr).y_offset,
            (*info_ptr).offset_unit_type as c_int,
        );
    }

    /* PNG_WRITE_pCAL_SUPPORTED */
    if ((*info_ptr).valid & PNG_INFO_pCAL) != 0 {
        png_write_pCAL(
            png_ptr,
            (*info_ptr).pcal_purpose,
            (*info_ptr).pcal_X0,
            (*info_ptr).pcal_X1,
            (*info_ptr).pcal_type as c_int,
            (*info_ptr).pcal_nparams as c_int,
            (*info_ptr).pcal_units,
            (*info_ptr).pcal_params,
        );
    }

    /* PNG_WRITE_sCAL_SUPPORTED */
    if ((*info_ptr).valid & PNG_INFO_sCAL) != 0 {
        png_write_sCAL_s(
            png_ptr,
            (*info_ptr).scal_unit as c_int,
            (*info_ptr).scal_s_width,
            (*info_ptr).scal_s_height,
        );
    }

    /* PNG_WRITE_pHYs_SUPPORTED */
    if ((*info_ptr).valid & PNG_INFO_pHYs) != 0 {
        png_write_pHYs(
            png_ptr,
            (*info_ptr).x_pixels_per_unit,
            (*info_ptr).y_pixels_per_unit,
            (*info_ptr).phys_unit_type as c_int,
        );
    }

    /* PNG_WRITE_tIME_SUPPORTED */
    if ((*info_ptr).valid & PNG_INFO_tIME) != 0 {
        png_write_tIME(png_ptr, &(*info_ptr).mod_time);
        (*png_ptr).mode |= PNG_WROTE_tIME;
    }

    /* PNG_WRITE_sPLT_SUPPORTED */
    if ((*info_ptr).valid & PNG_INFO_sPLT) != 0 {
        i = 0;
        while i < (*info_ptr).splt_palettes_num {
            png_write_sPLT(png_ptr, (*info_ptr).splt_palettes.add(i as usize));
            i += 1;
        }
    }

    /* PNG_WRITE_TEXT_SUPPORTED */
    /* Check to see if we need to write text chunks */
    i = 0;
    while i < (*info_ptr).num_text {
        let text_i: png_textp = (*info_ptr).text.add(i as usize);
        /* An internationalized chunk? */
        if (*text_i).compression > 0 {
            /* PNG_WRITE_iTXt_SUPPORTED */
            /* Write international chunk */
            png_write_iTXt(
                png_ptr,
                (*text_i).compression,
                (*text_i).key,
                (*text_i).lang,
                (*text_i).lang_key,
                (*text_i).text,
            );
            /* Mark this chunk as written */
            if (*text_i).compression == PNG_TEXT_COMPRESSION_NONE {
                (*text_i).compression = PNG_TEXT_COMPRESSION_NONE_WR;
            } else {
                (*text_i).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
            }
        }
        /* If we want a compressed text chunk */
        else if (*text_i).compression == PNG_TEXT_COMPRESSION_zTXt {
            /* PNG_WRITE_zTXt_SUPPORTED */
            /* Write compressed chunk */
            png_write_zTXt(
                png_ptr,
                (*text_i).key,
                (*text_i).text,
                (*text_i).compression,
            );
            /* Mark this chunk as written */
            (*text_i).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
        } else if (*text_i).compression == PNG_TEXT_COMPRESSION_NONE {
            /* PNG_WRITE_tEXt_SUPPORTED */
            /* Write uncompressed chunk */
            png_write_tEXt(png_ptr, (*text_i).key, (*text_i).text, 0);
            /* Mark this chunk as written */
            (*text_i).compression = PNG_TEXT_COMPRESSION_NONE_WR;
        }
        i += 1;
    }

    /* PNG_WRITE_UNKNOWN_CHUNKS_SUPPORTED */
    write_unknown_chunks(png_ptr, info_ptr, PNG_HAVE_PLTE);
}

/* Writes the end of the PNG file. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_end(png_ptr: png_structrp, info_ptr: png_inforp) {
    if png_ptr.is_null() {
        return;
    }

    if ((*png_ptr).mode & PNG_HAVE_IDAT) == 0 {
        png_error(png_ptr, cstr(b"No IDATs written into file\0"));
    }

    /* PNG_WRITE_CHECK_FOR_INVALID_INDEX_SUPPORTED */
    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
        && (*png_ptr).num_palette_max >= (*png_ptr).num_palette as c_int
    {
        png_benign_error(
            png_ptr,
            cstr(b"Wrote palette index exceeding num_palette\0"),
        );
    }

    /* See if user wants us to write information chunks */
    if !info_ptr.is_null() {
        /* PNG_WRITE_TEXT_SUPPORTED */
        let mut i: c_int; /* local index variable */

        /* PNG_WRITE_tIME_SUPPORTED */
        /* Check to see if user has supplied a time chunk */
        if ((*info_ptr).valid & PNG_INFO_tIME) != 0 && ((*png_ptr).mode & PNG_WROTE_tIME) == 0 {
            png_write_tIME(png_ptr, &(*info_ptr).mod_time);
        }

        /* PNG_WRITE_TEXT_SUPPORTED */
        /* Loop through comment chunks */
        i = 0;
        while i < (*info_ptr).num_text {
            let text_i: png_textp = (*info_ptr).text.add(i as usize);
            /* An internationalized chunk? */
            if (*text_i).compression > 0 {
                /* PNG_WRITE_iTXt_SUPPORTED */
                /* Write international chunk */
                png_write_iTXt(
                    png_ptr,
                    (*text_i).compression,
                    (*text_i).key,
                    (*text_i).lang,
                    (*text_i).lang_key,
                    (*text_i).text,
                );
                /* Mark this chunk as written */
                if (*text_i).compression == PNG_TEXT_COMPRESSION_NONE {
                    (*text_i).compression = PNG_TEXT_COMPRESSION_NONE_WR;
                } else {
                    (*text_i).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
                }
            } else if (*text_i).compression >= PNG_TEXT_COMPRESSION_zTXt {
                /* PNG_WRITE_zTXt_SUPPORTED */
                /* Write compressed chunk */
                png_write_zTXt(
                    png_ptr,
                    (*text_i).key,
                    (*text_i).text,
                    (*text_i).compression,
                );
                /* Mark this chunk as written */
                (*text_i).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
            } else if (*text_i).compression == PNG_TEXT_COMPRESSION_NONE {
                /* PNG_WRITE_tEXt_SUPPORTED */
                /* Write uncompressed chunk */
                png_write_tEXt(png_ptr, (*text_i).key, (*text_i).text, 0);
                /* Mark this chunk as written */
                (*text_i).compression = PNG_TEXT_COMPRESSION_NONE_WR;
            }
            i += 1;
        }

        /* PNG_WRITE_eXIf_SUPPORTED */
        if ((*info_ptr).valid & PNG_INFO_eXIf) != 0 && ((*png_ptr).mode & PNG_WROTE_eXIf) == 0 {
            png_write_eXIf(png_ptr, (*info_ptr).exif, (*info_ptr).num_exif as c_int);
        }

        /* PNG_WRITE_UNKNOWN_CHUNKS_SUPPORTED */
        write_unknown_chunks(png_ptr, info_ptr, PNG_AFTER_IDAT);
    }

    (*png_ptr).mode |= PNG_AFTER_IDAT;

    /* Write end of PNG file */
    png_write_IEND(png_ptr);

    /* PNG_WRITE_FLUSH_SUPPORTED */
    /* PNG_WRITE_FLUSH_AFTER_IEND_SUPPORTED is NOT defined, so no png_flush here. */
}

/* PNG_CONVERT_tIME_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_convert_from_struct_tm(ptime: png_timep, ttime: *const tm) {
    (*ptime).year = (1900 + (*ttime).tm_year) as png_uint_16;
    (*ptime).month = ((*ttime).tm_mon + 1) as png_byte;
    (*ptime).day = (*ttime).tm_mday as png_byte;
    (*ptime).hour = (*ttime).tm_hour as png_byte;
    (*ptime).minute = (*ttime).tm_min as png_byte;
    (*ptime).second = (*ttime).tm_sec as png_byte;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_convert_from_time_t(ptime: png_timep, ttime: time_t) {
    let tbuf: *mut tm;

    tbuf = gmtime(&ttime);
    if tbuf.is_null() {
        memset(ptime as *mut c_void, 0, core::mem::size_of::<png_time>());
        return;
    }

    png_convert_from_struct_tm(ptime, tbuf);
}

/* Initialize png_ptr structure, and allocate any memory needed */
/* PNG_USER_MEM_SUPPORTED is defined, so png_create_write_struct just forwards
 * to png_create_write_struct_2.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_write_struct(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
) -> png_structp {
    png_create_write_struct_2(
        user_png_ver,
        error_ptr,
        error_fn,
        warn_fn,
        core::ptr::null_mut(),
        None,
        None,
    )
}

/* Alternate initialize png_ptr structure, and allocate any memory needed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_write_struct_2(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
    mem_ptr: png_voidp,
    malloc_fn: png_malloc_ptr,
    free_fn: png_free_ptr,
) -> png_structp {
    let png_ptr: png_structrp = png_create_png_struct(
        user_png_ver,
        error_ptr,
        error_fn,
        warn_fn,
        mem_ptr,
        malloc_fn,
        free_fn,
    );

    if !png_ptr.is_null() {
        /* Set the zlib control values to defaults; they can be overridden by the
         * application after the struct has been created.
         */
        (*png_ptr).zbuffer_size = PNG_ZBUF_SIZE as uInt;

        /* The 'zlib_strategy' setting is irrelevant because png_default_claim in
         * pngwutil.c defaults it according to whether or not filters will be
         * used, and ignores this setting.
         */
        (*png_ptr).zlib_strategy = PNG_Z_DEFAULT_STRATEGY;
        (*png_ptr).zlib_level = PNG_Z_DEFAULT_COMPRESSION;
        (*png_ptr).zlib_mem_level = 8;
        (*png_ptr).zlib_window_bits = 15;
        (*png_ptr).zlib_method = 8;

        /* PNG_WRITE_COMPRESSED_TEXT_SUPPORTED */
        (*png_ptr).zlib_text_strategy = PNG_TEXT_Z_DEFAULT_STRATEGY;
        (*png_ptr).zlib_text_level = PNG_TEXT_Z_DEFAULT_COMPRESSION;
        (*png_ptr).zlib_text_mem_level = 8;
        (*png_ptr).zlib_text_window_bits = 15;
        (*png_ptr).zlib_text_method = 8;

        /* PNG_BENIGN_WRITE_ERRORS_SUPPORTED is NOT defined, so no
         * PNG_FLAG_BENIGN_ERRORS_WARN set here.
         */

        /* App warnings are warnings in release (or release candidate) builds but
         * are errors during development.  PNG_RELEASE_BUILD == 0 here, so this is
         * skipped.
         */

        png_set_write_fn(png_ptr, core::ptr::null_mut(), None, None);
    }

    png_ptr
}

/* Write a few rows of image data. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_rows(
    png_ptr: png_structrp,
    row: png_bytepp,
    num_rows: png_uint_32,
) {
    let mut i: png_uint_32; /* row counter */
    let mut rp: png_bytepp; /* row pointer */

    if png_ptr.is_null() {
        return;
    }

    /* Loop through the rows */
    i = 0;
    rp = row;
    while i < num_rows {
        png_write_row(png_ptr, *rp);
        i += 1;
        rp = rp.add(1);
    }
}

/* Write the image. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_image(png_ptr: png_structrp, image: png_bytepp) {
    let mut i: png_uint_32; /* row index */
    let mut pass: c_int;
    let num_pass: c_int; /* pass variables */
    let mut rp: png_bytepp; /* points to current row */

    if png_ptr.is_null() {
        return;
    }

    /* PNG_WRITE_INTERLACING_SUPPORTED */
    /* Initialize interlace handling.  If image is not interlaced,
     * this will set pass to 1
     */
    num_pass = png_set_interlace_handling(png_ptr);

    /* Loop through passes */
    pass = 0;
    while pass < num_pass {
        /* Loop through image */
        i = 0;
        rp = image;
        while i < (*png_ptr).height {
            png_write_row(png_ptr, *rp);
            i += 1;
            rp = rp.add(1);
        }
        pass += 1;
    }
}

/* PNG_MNG_FEATURES_SUPPORTED */
/* Performs intrapixel differencing  */
pub unsafe extern "C" fn png_do_write_intrapixel(row_info: png_row_infop, row: png_bytep) {
    if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        let bytes_per_pixel: c_int;
        let row_width: png_uint_32 = (*row_info).width;
        if (*row_info).bit_depth == 8 {
            let mut rp: png_bytep;
            let mut i: png_uint_32;

            if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB {
                bytes_per_pixel = 3;
            } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                bytes_per_pixel = 4;
            } else {
                return;
            }

            i = 0;
            rp = row;
            while i < row_width {
                *rp = (*rp as c_int - *rp.add(1) as c_int) as png_byte;
                *rp.add(2) = (*rp.add(2) as c_int - *rp.add(1) as c_int) as png_byte;
                i += 1;
                rp = rp.add(bytes_per_pixel as usize);
            }
        }
        /* PNG_WRITE_16BIT_SUPPORTED */
        else if (*row_info).bit_depth == 16 {
            let mut rp: png_bytep;
            let mut i: png_uint_32;

            if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB {
                bytes_per_pixel = 6;
            } else if (*row_info).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                bytes_per_pixel = 8;
            } else {
                return;
            }

            i = 0;
            rp = row;
            while i < row_width {
                let s0: png_uint_32 =
                    ((*rp as c_int) << 8) as png_uint_32 | *rp.add(1) as png_uint_32;
                let s1: png_uint_32 =
                    ((*rp.add(2) as c_int) << 8) as png_uint_32 | *rp.add(3) as png_uint_32;
                let s2: png_uint_32 =
                    ((*rp.add(4) as c_int) << 8) as png_uint_32 | *rp.add(5) as png_uint_32;
                let red: png_uint_32 = s0.wrapping_sub(s1) & 0xffff;
                let blue: png_uint_32 = s2.wrapping_sub(s1) & 0xffff;
                *rp = (red >> 8) as png_byte;
                *rp.add(1) = red as png_byte;
                *rp.add(4) = (blue >> 8) as png_byte;
                *rp.add(5) = blue as png_byte;
                i += 1;
                rp = rp.add(bytes_per_pixel as usize);
            }
        }
    }
}

/* Called by user to write a row of image data */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_row(png_ptr: png_structrp, row: png_const_bytep) {
    /* 1.5.6: moved from png_struct to be a local structure: */
    let mut row_info: png_row_info = png_row_info::default();

    if png_ptr.is_null() {
        return;
    }

    /* Initialize transformations and other stuff if first time */
    if (*png_ptr).row_number == 0 && (*png_ptr).pass == 0 {
        /* Make sure we wrote the header info */
        if ((*png_ptr).mode & PNG_WROTE_INFO_BEFORE_PLTE) == 0 {
            png_error(
                png_ptr,
                cstr(b"png_write_info was never called before png_write_row\0"),
            );
        }

        /* Check for transforms that have been set but were defined out: all the
         * relevant PNG_WRITE_* macros are defined in this build, so none of the
         * warning blocks are included.
         */

        png_write_start_row(png_ptr);
    }

    /* PNG_WRITE_INTERLACING_SUPPORTED */
    /* If interlaced and not interested in row, return */
    if (*png_ptr).interlaced != 0 && ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
        match (*png_ptr).pass {
            0 => {
                if ((*png_ptr).row_number & 0x07) != 0 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            1 => {
                if ((*png_ptr).row_number & 0x07) != 0 || (*png_ptr).width < 5 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            2 => {
                if ((*png_ptr).row_number & 0x07) != 4 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            3 => {
                if ((*png_ptr).row_number & 0x03) != 0 || (*png_ptr).width < 3 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            4 => {
                if ((*png_ptr).row_number & 0x03) != 2 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            5 => {
                if ((*png_ptr).row_number & 0x01) != 0 || (*png_ptr).width < 2 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            6 => {
                if ((*png_ptr).row_number & 0x01) == 0 {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            _ => { /* error: ignore it */ }
        }
    }

    /* Set up row info for transformations */
    row_info.color_type = (*png_ptr).color_type;
    row_info.width = (*png_ptr).usr_width;
    row_info.channels = (*png_ptr).usr_channels;
    row_info.bit_depth = (*png_ptr).usr_bit_depth;
    row_info.pixel_depth = (row_info.bit_depth as c_int * row_info.channels as c_int) as png_byte;
    row_info.rowbytes = PNG_ROWBYTES(row_info.pixel_depth as usize, row_info.width as usize);

    /* Copy user's row into buffer, leaving room for filter byte. */
    memcpy(
        (*png_ptr).row_buf.add(1) as *mut c_void,
        row as *const c_void,
        row_info.rowbytes,
    );

    /* PNG_WRITE_INTERLACING_SUPPORTED */
    /* Handle interlacing */
    if (*png_ptr).interlaced != 0
        && (*png_ptr).pass < 6
        && ((*png_ptr).transformations & PNG_INTERLACE) != 0
    {
        png_do_write_interlace(
            &mut row_info,
            (*png_ptr).row_buf.add(1),
            (*png_ptr).pass as c_int,
        );
        /* This should always get caught above, but still ... */
        if row_info.width == 0 {
            png_write_finish_row(png_ptr);
            return;
        }
    }

    /* PNG_WRITE_TRANSFORMS_SUPPORTED */
    /* Handle other transformations */
    if (*png_ptr).transformations != 0 {
        png_do_write_transformations(png_ptr, &mut row_info);
    }

    /* At this point the row_info pixel depth must match the 'transformed' depth,
     * which is also the output depth.
     */
    if row_info.pixel_depth != (*png_ptr).pixel_depth
        || row_info.pixel_depth != (*png_ptr).transformed_pixel_depth
    {
        png_error(png_ptr, cstr(b"internal write transform logic error\0"));
    }

    /* PNG_MNG_FEATURES_SUPPORTED */
    if ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
        && ((*png_ptr).filter_type as c_int == PNG_INTRAPIXEL_DIFFERENCING)
    {
        /* Intrapixel differencing */
        png_do_write_intrapixel(&mut row_info, (*png_ptr).row_buf.add(1));
    }

    /* Added at libpng-1.5.10 */
    /* PNG_WRITE_CHECK_FOR_INVALID_INDEX_SUPPORTED */
    /* Check for out-of-range palette index */
    if row_info.color_type as c_int == PNG_COLOR_TYPE_PALETTE && (*png_ptr).num_palette_max >= 0 {
        png_do_check_palette_indexes(png_ptr, &mut row_info);
    }

    /* Find a filter if necessary, filter the row and write it out. */
    png_write_find_filter(png_ptr, &mut row_info);

    if (*png_ptr).write_row_fn.is_some() {
        if let Some(f) = (*png_ptr).write_row_fn {
            f(
                png_ptr as png_structp,
                (*png_ptr).row_number,
                (*png_ptr).pass as c_int,
            );
        }
    }
}

/* PNG_WRITE_FLUSH_SUPPORTED */
/* Set the automatic flush interval or 0 to turn flushing off */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_flush(png_ptr: png_structrp, nrows: c_int) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).flush_dist = if nrows < 0 { 0 } else { nrows as png_uint_32 };
}

/* Flush the current output buffers now */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_flush(png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }

    /* We have already written out all of the data */
    if (*png_ptr).row_number >= (*png_ptr).num_rows {
        return;
    }

    png_compress_IDAT(png_ptr, core::ptr::null(), 0, Z_SYNC_FLUSH);
    (*png_ptr).flush_rows = 0;
    png_flush(png_ptr);
}

/* Free any memory used in png_ptr struct without freeing the struct itself. */
pub unsafe extern "C" fn png_write_destroy(png_ptr: png_structrp) {
    /* Free any memory zlib uses */
    if ((*png_ptr).flags & PNG_FLAG_ZSTREAM_INITIALIZED) != 0 {
        deflateEnd(&mut (*png_ptr).zstream);
    }

    /* Free our memory.  png_free checks NULL for us. */
    png_free_buffer_list(png_ptr, &mut (*png_ptr).zbuffer_list);
    png_free(png_ptr, (*png_ptr).row_buf as png_voidp);
    (*png_ptr).row_buf = core::ptr::null_mut();
    /* PNG_WRITE_FILTER_SUPPORTED */
    png_free(png_ptr, (*png_ptr).prev_row as png_voidp);
    png_free(png_ptr, (*png_ptr).try_row as png_voidp);
    png_free(png_ptr, (*png_ptr).tst_row as png_voidp);
    (*png_ptr).prev_row = core::ptr::null_mut();
    (*png_ptr).try_row = core::ptr::null_mut();
    (*png_ptr).tst_row = core::ptr::null_mut();

    /* PNG_SET_UNKNOWN_CHUNKS_SUPPORTED */
    png_free(png_ptr, (*png_ptr).chunk_list as png_voidp);
    (*png_ptr).chunk_list = core::ptr::null_mut();

    /* PNG_tRNS_SUPPORTED */
    /* Free the independent copy of trans_alpha owned by png_struct. */
    png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
    (*png_ptr).trans_alpha = core::ptr::null_mut();

    /* Free the independent copy of the palette owned by png_struct. */
    png_free(png_ptr, (*png_ptr).palette as png_voidp);
    (*png_ptr).palette = core::ptr::null_mut();

    /* The error handling and memory handling information is left intact at this
     * point: the jmp_buf may still have to be freed.
     */
}

/* Free all memory used by the write. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_write_struct(
    png_ptr_ptr: png_structpp,
    info_ptr_ptr: png_infopp,
) {
    if !png_ptr_ptr.is_null() {
        let png_ptr: png_structrp = *png_ptr_ptr;

        if !png_ptr.is_null() {
            /* added in libpng 1.6.0 */
            png_destroy_info_struct(png_ptr, info_ptr_ptr);

            *png_ptr_ptr = core::ptr::null_mut();
            png_write_destroy(png_ptr);
            png_destroy_png_struct(png_ptr);
        }
    }
}

/* Allow the application to select one or more row filters to use. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filter(png_ptr: png_structrp, method: c_int, mut filters: c_int) {
    let mut method = method;

    if png_ptr.is_null() {
        return;
    }

    /* PNG_MNG_FEATURES_SUPPORTED */
    if ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
        && (method == PNG_INTRAPIXEL_DIFFERENCING)
    {
        method = PNG_FILTER_TYPE_BASE;
    }

    if method == PNG_FILTER_TYPE_BASE {
        match filters & (PNG_ALL_FILTERS | 0x07) {
            /* PNG_WRITE_FILTER_SUPPORTED */
            5 | 6 | 7 => {
                png_app_error(png_ptr, cstr(b"Unknown row filter for method 0\0"));
                /* FALLTHROUGH */
                (*png_ptr).do_filter = PNG_FILTER_NONE as png_byte;
            }
            x if x == PNG_FILTER_VALUE_NONE => {
                (*png_ptr).do_filter = PNG_FILTER_NONE as png_byte;
            }

            /* PNG_WRITE_FILTER_SUPPORTED */
            x if x == PNG_FILTER_VALUE_SUB => {
                (*png_ptr).do_filter = PNG_FILTER_SUB as png_byte;
            }

            x if x == PNG_FILTER_VALUE_UP => {
                (*png_ptr).do_filter = PNG_FILTER_UP as png_byte;
            }

            x if x == PNG_FILTER_VALUE_AVG => {
                (*png_ptr).do_filter = PNG_FILTER_AVG as png_byte;
            }

            x if x == PNG_FILTER_VALUE_PAETH => {
                (*png_ptr).do_filter = PNG_FILTER_PAETH as png_byte;
            }

            _ => {
                (*png_ptr).do_filter = filters as png_byte;
            }
        }

        /* PNG_WRITE_FILTER_SUPPORTED */
        /* If we have allocated the row_buf, this means we have already started
         * with the image and we should have allocated all of the filter buffers
         * that have been selected.
         */
        if !(*png_ptr).row_buf.is_null() {
            let mut num_filters: c_int;
            let buf_size: png_alloc_size_t;

            /* Repeat the checks in png_write_start_row; 1 pixel high or wide
             * images cannot benefit from certain filters.
             */
            if (*png_ptr).height == 1 {
                filters &= !(PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH);
            }

            if (*png_ptr).width == 1 {
                filters &= !(PNG_FILTER_SUB | PNG_FILTER_AVG | PNG_FILTER_PAETH);
            }

            if (filters & (PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH)) != 0
                && (*png_ptr).prev_row.is_null()
            {
                /* This is the error case, however it is benign - the previous row
                 * is not available so the filter can't be used.  Just warn here.
                 */
                png_app_warning(
                    png_ptr,
                    cstr(b"png_set_filter: UP/AVG/PAETH cannot be added after start\0"),
                );
                filters &= !(PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH);
            }

            num_filters = 0;

            if (filters & PNG_FILTER_SUB) != 0 {
                num_filters += 1;
            }

            if (filters & PNG_FILTER_UP) != 0 {
                num_filters += 1;
            }

            if (filters & PNG_FILTER_AVG) != 0 {
                num_filters += 1;
            }

            if (filters & PNG_FILTER_PAETH) != 0 {
                num_filters += 1;
            }

            /* Allocate needed row buffers if they have not already been
             * allocated.
             */
            buf_size = (PNG_ROWBYTES(
                ((*png_ptr).usr_channels as c_int * (*png_ptr).usr_bit_depth as c_int) as usize,
                (*png_ptr).width as usize,
            ) + 1) as png_alloc_size_t;

            if (*png_ptr).try_row.is_null() {
                (*png_ptr).try_row = png_malloc(png_ptr, buf_size) as png_bytep;
            }

            if num_filters > 1 {
                if (*png_ptr).tst_row.is_null() {
                    (*png_ptr).tst_row = png_malloc(png_ptr, buf_size) as png_bytep;
                }
            }
        }
        (*png_ptr).do_filter = filters as png_byte;
    } else {
        png_error(png_ptr, cstr(b"Unknown custom filter method\0"));
    }
}

/* PNG_WRITE_WEIGHTED_FILTER_SUPPORTED (DEPRECATED) */
/* PNG_FLOATING_POINT_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filter_heuristics(
    _png_ptr: png_structrp,
    _heuristic_method: c_int,
    _num_weights: c_int,
    _filter_weights: png_const_doublep,
    _filter_costs: png_const_doublep,
) {
}

/* PNG_FIXED_POINT_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filter_heuristics_fixed(
    _png_ptr: png_structrp,
    _heuristic_method: c_int,
    _num_weights: c_int,
    _filter_weights: png_const_fixed_point_p,
    _filter_costs: png_const_fixed_point_p,
) {
}

/* PNG_WRITE_CUSTOMIZE_COMPRESSION_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_level(png_ptr: png_structrp, level: c_int) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).zlib_level = level;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_mem_level(png_ptr: png_structrp, mem_level: c_int) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).zlib_mem_level = mem_level;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_strategy(png_ptr: png_structrp, strategy: c_int) {
    if png_ptr.is_null() {
        return;
    }

    /* The flag setting here prevents the libpng dynamic selection of strategy. */
    (*png_ptr).flags |= PNG_FLAG_ZLIB_CUSTOM_STRATEGY;
    (*png_ptr).zlib_strategy = strategy;
}

/* If PNG_WRITE_OPTIMIZE_CMF_SUPPORTED is defined, libpng will use a
 * smaller value of window_bits if it can do so safely.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_window_bits(
    png_ptr: png_structrp,
    mut window_bits: c_int,
) {
    if png_ptr.is_null() {
        return;
    }

    if window_bits > 15 {
        png_warning(
            png_ptr,
            cstr(b"Only compression windows <= 32k supported by PNG\0"),
        );
        window_bits = 15;
    } else if window_bits < 8 {
        png_warning(
            png_ptr,
            cstr(b"Only compression windows >= 256 supported by PNG\0"),
        );
        window_bits = 8;
    }

    (*png_ptr).zlib_window_bits = window_bits;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_method(png_ptr: png_structrp, method: c_int) {
    if png_ptr.is_null() {
        return;
    }

    /* This would produce an invalid PNG file if it worked, but it doesn't and
     * deflate will fault it, so it is harmless to just warn here.
     */
    if method != 8 {
        png_warning(
            png_ptr,
            cstr(b"Only compression method 8 is supported by PNG\0"),
        );
    }

    (*png_ptr).zlib_method = method;
}

/* The following were added to libpng-1.5.4 */
/* PNG_WRITE_CUSTOMIZE_ZTXT_COMPRESSION_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_level(png_ptr: png_structrp, level: c_int) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).zlib_text_level = level;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_mem_level(
    png_ptr: png_structrp,
    mem_level: c_int,
) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).zlib_text_mem_level = mem_level;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_strategy(png_ptr: png_structrp, strategy: c_int) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).zlib_text_strategy = strategy;
}

/* If PNG_WRITE_OPTIMIZE_CMF_SUPPORTED is defined, libpng will use a
 * smaller value of window_bits if it can do so safely.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_window_bits(
    png_ptr: png_structrp,
    mut window_bits: c_int,
) {
    if png_ptr.is_null() {
        return;
    }

    if window_bits > 15 {
        png_warning(
            png_ptr,
            cstr(b"Only compression windows <= 32k supported by PNG\0"),
        );
        window_bits = 15;
    } else if window_bits < 8 {
        png_warning(
            png_ptr,
            cstr(b"Only compression windows >= 256 supported by PNG\0"),
        );
        window_bits = 8;
    }

    (*png_ptr).zlib_text_window_bits = window_bits;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_method(png_ptr: png_structrp, method: c_int) {
    if png_ptr.is_null() {
        return;
    }

    if method != 8 {
        png_warning(
            png_ptr,
            cstr(b"Only compression method 8 is supported by PNG\0"),
        );
    }

    (*png_ptr).zlib_text_method = method;
}
/* end of API added to libpng-1.5.4 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_write_status_fn(
    png_ptr: png_structrp,
    write_row_fn: png_write_status_ptr,
) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).write_row_fn = write_row_fn;
}

/* PNG_WRITE_USER_TRANSFORM_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_write_user_transform_fn(
    png_ptr: png_structrp,
    write_user_transform_fn: png_user_transform_ptr,
) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).transformations |= PNG_USER_TRANSFORM;
    (*png_ptr).write_user_transform_fn = write_user_transform_fn;
}

/* PNG_INFO_IMAGE_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_png(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    transforms: c_int,
    _params: png_voidp,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    if ((*info_ptr).valid & PNG_INFO_IDAT) == 0 {
        png_app_error(png_ptr, cstr(b"no rows for png_write_image to write\0"));
        return;
    }

    /* Write the file header information. */
    png_write_info(png_ptr, info_ptr);

    /* ------ these transformations don't touch the info structure ------- */

    /* Invert monochrome pixels */
    if (transforms & PNG_TRANSFORM_INVERT_MONO) != 0 {
        /* PNG_WRITE_INVERT_SUPPORTED */
        png_set_invert_mono(png_ptr);
    }

    /* Shift the pixels up to a legal bit depth and fill in
     * as appropriate to correctly scale the image.
     */
    if (transforms & PNG_TRANSFORM_SHIFT) != 0 {
        /* PNG_WRITE_SHIFT_SUPPORTED */
        if ((*info_ptr).valid & PNG_INFO_sBIT) != 0 {
            png_set_shift(png_ptr, &(*info_ptr).sig_bit);
        }
    }

    /* Pack pixels into bytes */
    if (transforms & PNG_TRANSFORM_PACKING) != 0 {
        /* PNG_WRITE_PACK_SUPPORTED */
        png_set_packing(png_ptr);
    }

    /* Swap location of alpha bytes from ARGB to RGBA */
    if (transforms & PNG_TRANSFORM_SWAP_ALPHA) != 0 {
        /* PNG_WRITE_SWAP_ALPHA_SUPPORTED */
        png_set_swap_alpha(png_ptr);
    }

    /* Remove a filler (X) from XRGB/RGBX/AG/GA into to convert it into
     * RGB, note that the code expects the input color type to be G or RGB; no
     * alpha channel.
     */
    if (transforms & (PNG_TRANSFORM_STRIP_FILLER_AFTER | PNG_TRANSFORM_STRIP_FILLER_BEFORE)) != 0 {
        /* PNG_WRITE_FILLER_SUPPORTED */
        if (transforms & PNG_TRANSFORM_STRIP_FILLER_AFTER) != 0 {
            if (transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE) != 0 {
                png_app_error(
                    png_ptr,
                    cstr(b"PNG_TRANSFORM_STRIP_FILLER: BEFORE+AFTER not supported\0"),
                );
            }

            /* Continue if ignored - this is the pre-1.6.10 behavior */
            png_set_filler(png_ptr, 0, PNG_FILLER_AFTER);
        } else if (transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE) != 0 {
            png_set_filler(png_ptr, 0, PNG_FILLER_BEFORE);
        }
    }

    /* Flip BGR pixels to RGB */
    if (transforms & PNG_TRANSFORM_BGR) != 0 {
        /* PNG_WRITE_BGR_SUPPORTED */
        png_set_bgr(png_ptr);
    }

    /* Swap bytes of 16-bit files to most significant byte first */
    if (transforms & PNG_TRANSFORM_SWAP_ENDIAN) != 0 {
        /* PNG_WRITE_SWAP_SUPPORTED */
        png_set_swap(png_ptr);
    }

    /* Swap bits of 1-bit, 2-bit, 4-bit packed pixel formats */
    if (transforms & PNG_TRANSFORM_PACKSWAP) != 0 {
        /* PNG_WRITE_PACKSWAP_SUPPORTED */
        png_set_packswap(png_ptr);
    }

    /* Invert the alpha channel from opacity to transparency */
    if (transforms & PNG_TRANSFORM_INVERT_ALPHA) != 0 {
        /* PNG_WRITE_INVERT_ALPHA_SUPPORTED */
        png_set_invert_alpha(png_ptr);
    }

    /* ----------------------- end of transformations ------------------- */

    /* Write the bits */
    png_write_image(png_ptr, (*info_ptr).row_pointers);

    /* It is REQUIRED to call this to finish writing the rest of the file */
    png_write_end(png_ptr, info_ptr);
}
