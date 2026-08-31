//! Translation of pngwrite.c

use crate::*;

/* png_const_fixed_point_p is not provided by the prelude; define it here
 * (the deprecated fixed-point filter-heuristics API needs it).
 */
type png_const_fixed_point_p = *const png_fixed_point;

/* Externally-defined data tables (png.c) and libc functions not provided by
 * the prelude.  These are exported by the reference .so / linked libc, so we
 * only declare them here privately (the guide forbids modifying pngpriv.rs).
 */
unsafe extern "C" {
    static png_sRGB_base: [png_uint_16; 512];
    static png_sRGB_delta: [png_byte; 512];

    fn __errno_location() -> *mut c_int;
    fn strerror(errnum: c_int) -> *mut c_char;
    fn remove(path: *const c_char) -> c_int;
}

/* PNG_sRGB_FROM_LINEAR: given a value 'linear' in the range 0..255*65535
 * calculate the 8-bit sRGB encoded value.  The input has already been
 * multiplied by 255.
 */
#[inline]
unsafe fn PNG_sRGB_FROM_LINEAR(linear: png_uint_32) -> png_byte {
    unsafe {
        (0xff
            & ((png_sRGB_base[(linear >> 15) as usize] as png_uint_32
                + (((linear & 0x7fff) * png_sRGB_delta[(linear >> 15) as usize] as png_uint_32)
                    >> 12))
                >> 8)) as png_byte
    }
}

/* Write out all the unknown chunks for the current given location */
unsafe fn write_unknown_chunks(
    png_ptr: png_structrp,
    info_ptr: png_const_inforp,
    r#where: c_uint,
) {
    unsafe {
        if (*info_ptr).unknown_chunks_num != 0 {
            let mut up: png_const_unknown_chunkp;

            up = (*info_ptr).unknown_chunks;
            while up < (*info_ptr).unknown_chunks.offset((*info_ptr).unknown_chunks_num as isize) {
                if ((*up).location as c_uint & r#where) != 0 {
                    /* If per-chunk unknown chunk handling is enabled use it, otherwise
                     * just write the chunks the application has set.
                     */
                    let keep: c_int = png_handle_as_unknown(png_ptr, (*up).name.as_ptr());

                    /* NOTE: this code is radically different from the read side in the
                     * matter of handling an ancillary unknown chunk.  In the read side
                     * the default behavior is to discard it, in the code below the default
                     * behavior is to write it.  Critical chunks are, however, only
                     * written if explicitly listed or if the default is set to write all
                     * unknown chunks.
                     *
                     * The default handling is also slightly weird - it is not possible to
                     * stop the writing of all unsafe-to-copy chunks!
                     *
                     * TODO: REVIEW: this would seem to be a bug.
                     */
                    if keep != PNG_HANDLE_CHUNK_NEVER
                        && (((*up).name[3] & 0x20) != 0/* safe-to-copy overrides everything */
                            || keep == PNG_HANDLE_CHUNK_ALWAYS
                            || (keep == PNG_HANDLE_CHUNK_AS_DEFAULT
                                && (*png_ptr).unknown_default == PNG_HANDLE_CHUNK_ALWAYS))
                    {
                        /* TODO: review, what is wrong with a zero length unknown chunk? */
                        if (*up).size == 0 {
                            png_warning(png_ptr, c"Writing zero-length unknown chunk".as_ptr());
                        }

                        png_write_chunk(png_ptr, (*up).name.as_ptr(), (*up).data, (*up).size);
                    }
                }

                up = up.offset(1);
            }
        }
    }
}

/* Writes all the PNG information.  This is the suggested way to use the
 * library.  If you have a new chunk to add, make a function to write it,
 * and put it in the correct location here.  If you want the chunk written
 * after the image data, put it in png_write_end().  I strongly encourage
 * you to supply a PNG_INFO_<chunk> flag, and check info_ptr->valid before
 * writing the chunk, as that will keep the code from breaking if you want
 * to just write a plain PNG file.  If you have long comments, I suggest
 * writing them in png_write_end(), and compressing them.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_info_before_PLTE(
    png_ptr: png_structrp,
    info_ptr: png_const_inforp,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        if ((*png_ptr).mode & PNG_WROTE_INFO_BEFORE_PLTE) == 0 {
            /* Write PNG signature */
            png_write_sig(png_ptr);

            if ((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) != 0
                && (*png_ptr).mng_features_permitted != 0
            {
                png_warning(
                    png_ptr,
                    c"MNG features are not allowed in a PNG datastream".as_ptr(),
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
                (*info_ptr).interlace_type as c_int,
            );

            /* The rest of these check to see if the valid field has the appropriate
             * flag set, and if it does, writes the chunk.
             *
             * 1.6.0: COLORSPACE support controls the writing of these chunks too, and
             * the chunks will be written if the WRITE routine is there and
             * information * is available in the COLORSPACE. (See
             * png_colorspace_sync_info in png.c for where the valid flags get set.)
             *
             * Under certain circumstances the colorspace can be invalidated without
             * syncing the info_struct 'valid' flags; this happens if libpng detects
             * an error and calls png_error while the color space is being set, yet
             * the application continues writing the PNG.  So check the 'invalid'
             * flag here too.
             */
            /* Write unknown chunks first; PNG v3 establishes a precedence order
             * for colourspace chunks.  It is certain therefore that new
             * colourspace chunks will have a precedence and very likely it will be
             * higher than all known so far.  Writing the unknown chunks here is
             * most likely to present the chunks in the most convenient order.
             *
             * FUTURE: maybe write chunks in the order the app calls png_set_chnk
             * to give the app control.
             */
            write_unknown_chunks(png_ptr, info_ptr, PNG_HAVE_IHDR);

            /* PNG v3: a streaming app will need to see this before cICP because
             * the information is helpful in handling HLG encoding (which is
             * natively 10 bits but gets expanded to 16 in PNG.)
             *
             * The app shouldn't care about the order ideally, but it might have
             * no choice.  In PNG v3, apps are allowed to reject PNGs where the
             * APNG chunks are out of order so it behooves libpng to be nice here.
             */
            if ((*info_ptr).valid & PNG_INFO_sBIT) != 0 {
                png_write_sBIT(png_ptr, &raw const (*info_ptr).sig_bit, (*info_ptr).color_type as c_int);
            }

            /* PNG v3: the July 2004 version of the TR introduced the concept of colour
             * space priority.  As above it therefore behooves libpng to write the colour
             * space chunks in the priority order so that a streaming app need not buffer
             * them.
             *
             * PNG v3: Chunks mDCV and cLLI provide ancillary information for the
             * interpretation of the colourspace chunks but do not require support for
             * those chunks so are outside the "COLORSPACE" check but before the write of
             * the colourspace chunks themselves.
             */
            if ((*info_ptr).valid & PNG_INFO_cLLI) != 0 {
                png_write_cLLI_fixed(png_ptr, (*info_ptr).maxCLL, (*info_ptr).maxFALL);
            }
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

            /* Priority 4 */
            if ((*info_ptr).valid & PNG_INFO_cICP) != 0 {
                png_write_cICP(
                    png_ptr,
                    (*info_ptr).cicp_colour_primaries,
                    (*info_ptr).cicp_transfer_function,
                    (*info_ptr).cicp_matrix_coefficients,
                    (*info_ptr).cicp_video_full_range_flag,
                );
            }

            /* Priority 3 */
            if ((*info_ptr).valid & PNG_INFO_iCCP) != 0 {
                png_write_iCCP(
                    png_ptr,
                    (*info_ptr).iccp_name,
                    (*info_ptr).iccp_profile,
                    (*info_ptr).iccp_proflen,
                );
            }

            /* Priority 2 */
            if ((*info_ptr).valid & PNG_INFO_sRGB) != 0 {
                png_write_sRGB(png_ptr, (*info_ptr).rendering_intent);
            }

            /* Priority 1 */
            if ((*info_ptr).valid & PNG_INFO_gAMA) != 0 {
                png_write_gAMA_fixed(png_ptr, (*info_ptr).gamma);
            }

            /* Also priority 1 */
            if ((*info_ptr).valid & PNG_INFO_cHRM) != 0 {
                png_write_cHRM_fixed(png_ptr, &raw const (*info_ptr).cHRM);
            }

            (*png_ptr).mode |= PNG_WROTE_INFO_BEFORE_PLTE;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_info(
    png_ptr: png_structrp,
    info_ptr: png_const_inforp,
) {
    unsafe {
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
            png_error(png_ptr, c"Valid palette required for paletted images".as_ptr());
        }

        if ((*info_ptr).valid & PNG_INFO_tRNS) != 0 {
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
                    *(*info_ptr).trans_alpha.offset(j as isize) =
                        (255 - *(*info_ptr).trans_alpha.offset(j as isize) as c_int) as png_byte;
                    j += 1;
                }
            }
            png_write_tRNS(
                png_ptr,
                (*info_ptr).trans_alpha,
                &raw const (*info_ptr).trans_color,
                (*info_ptr).num_trans as c_int,
                (*info_ptr).color_type as c_int,
            );
        }

        if ((*info_ptr).valid & PNG_INFO_bKGD) != 0 {
            png_write_bKGD(png_ptr, &raw const (*info_ptr).background, (*info_ptr).color_type as c_int);
        }

        if ((*info_ptr).valid & PNG_INFO_eXIf) != 0 {
            png_write_eXIf(png_ptr, (*info_ptr).exif, (*info_ptr).num_exif as c_int);
            (*png_ptr).mode |= PNG_WROTE_eXIf;
        }

        if ((*info_ptr).valid & PNG_INFO_hIST) != 0 {
            png_write_hIST(png_ptr, (*info_ptr).hist, (*info_ptr).num_palette as c_int);
        }

        if ((*info_ptr).valid & PNG_INFO_oFFs) != 0 {
            png_write_oFFs(
                png_ptr,
                (*info_ptr).x_offset,
                (*info_ptr).y_offset,
                (*info_ptr).offset_unit_type as c_int,
            );
        }

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

        if ((*info_ptr).valid & PNG_INFO_sCAL) != 0 {
            png_write_sCAL_s(
                png_ptr,
                (*info_ptr).scal_unit as c_int,
                (*info_ptr).scal_s_width,
                (*info_ptr).scal_s_height,
            );
        }

        if ((*info_ptr).valid & PNG_INFO_pHYs) != 0 {
            png_write_pHYs(
                png_ptr,
                (*info_ptr).x_pixels_per_unit,
                (*info_ptr).y_pixels_per_unit,
                (*info_ptr).phys_unit_type as c_int,
            );
        }

        if ((*info_ptr).valid & PNG_INFO_tIME) != 0 {
            png_write_tIME(png_ptr, &raw const (*info_ptr).mod_time);
            (*png_ptr).mode |= PNG_WROTE_tIME;
        }

        if ((*info_ptr).valid & PNG_INFO_sPLT) != 0 {
            i = 0;
            while i < (*info_ptr).splt_palettes_num {
                png_write_sPLT(png_ptr, (*info_ptr).splt_palettes.offset(i as isize));
                i += 1;
            }
        }

        /* Check to see if we need to write text chunks */
        i = 0;
        while i < (*info_ptr).num_text {
            let text = (*info_ptr).text.offset(i as isize);
            /* An internationalized chunk? */
            if (*text).compression > 0 {
                /* Write international chunk */
                png_write_iTXt(
                    png_ptr,
                    (*text).compression,
                    (*text).key,
                    (*text).lang,
                    (*text).lang_key,
                    (*text).text,
                );
                /* Mark this chunk as written */
                if (*text).compression == PNG_TEXT_COMPRESSION_NONE {
                    (*text).compression = PNG_TEXT_COMPRESSION_NONE_WR;
                } else {
                    (*text).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
                }
            }
            /* If we want a compressed text chunk */
            else if (*text).compression == PNG_TEXT_COMPRESSION_zTXt {
                /* Write compressed chunk */
                png_write_zTXt(png_ptr, (*text).key, (*text).text, (*text).compression);
                /* Mark this chunk as written */
                (*text).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
            } else if (*text).compression == PNG_TEXT_COMPRESSION_NONE {
                /* Write uncompressed chunk */
                png_write_tEXt(png_ptr, (*text).key, (*text).text, 0);
                /* Mark this chunk as written */
                (*text).compression = PNG_TEXT_COMPRESSION_NONE_WR;
            }

            i += 1;
        }

        write_unknown_chunks(png_ptr, info_ptr, PNG_HAVE_PLTE);
    }
}

/* Writes the end of the PNG file.  If you don't want to write comments or
 * time information, you can pass NULL for info.  If you already wrote these
 * in png_write_info(), do not write them again here.  If you have long
 * comments, I suggest writing them here, and compressing them.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_end(png_ptr: png_structrp, info_ptr: png_inforp) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        if ((*png_ptr).mode & PNG_HAVE_IDAT) == 0 {
            png_error(png_ptr, c"No IDATs written into file".as_ptr());
        }

        if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
            && (*png_ptr).num_palette_max >= (*png_ptr).num_palette as c_int
        {
            png_benign_error(png_ptr, c"Wrote palette index exceeding num_palette".as_ptr());
        }

        /* See if user wants us to write information chunks */
        if !info_ptr.is_null() {
            let mut i: c_int; /* local index variable */

            /* Check to see if user has supplied a time chunk */
            if ((*info_ptr).valid & PNG_INFO_tIME) != 0 && ((*png_ptr).mode & PNG_WROTE_tIME) == 0 {
                png_write_tIME(png_ptr, &raw const (*info_ptr).mod_time);
            }

            /* Loop through comment chunks */
            i = 0;
            while i < (*info_ptr).num_text {
                let text = (*info_ptr).text.offset(i as isize);
                /* An internationalized chunk? */
                if (*text).compression > 0 {
                    /* Write international chunk */
                    png_write_iTXt(
                        png_ptr,
                        (*text).compression,
                        (*text).key,
                        (*text).lang,
                        (*text).lang_key,
                        (*text).text,
                    );
                    /* Mark this chunk as written */
                    if (*text).compression == PNG_TEXT_COMPRESSION_NONE {
                        (*text).compression = PNG_TEXT_COMPRESSION_NONE_WR;
                    } else {
                        (*text).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
                    }
                } else if (*text).compression >= PNG_TEXT_COMPRESSION_zTXt {
                    /* Write compressed chunk */
                    png_write_zTXt(png_ptr, (*text).key, (*text).text, (*text).compression);
                    /* Mark this chunk as written */
                    (*text).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
                } else if (*text).compression == PNG_TEXT_COMPRESSION_NONE {
                    /* Write uncompressed chunk */
                    png_write_tEXt(png_ptr, (*text).key, (*text).text, 0);
                    /* Mark this chunk as written */
                    (*text).compression = PNG_TEXT_COMPRESSION_NONE_WR;
                }

                i += 1;
            }

            if ((*info_ptr).valid & PNG_INFO_eXIf) != 0 && ((*png_ptr).mode & PNG_WROTE_eXIf) == 0 {
                png_write_eXIf(png_ptr, (*info_ptr).exif, (*info_ptr).num_exif as c_int);
            }

            write_unknown_chunks(png_ptr, info_ptr, PNG_AFTER_IDAT);
        }

        (*png_ptr).mode |= PNG_AFTER_IDAT;

        /* Write end of PNG file */
        png_write_IEND(png_ptr);

        /* This flush, added in libpng-1.0.8, removed from libpng-1.0.9beta03,
         * and restored again in libpng-1.2.30, may cause some applications that
         * do not set png_ptr->output_flush_fn to crash.  If your application
         * experiences a problem, please try building libpng with
         * PNG_WRITE_FLUSH_AFTER_IEND_SUPPORTED defined, and report the event to
         * png-mng-implement at lists.sf.net .
         */
        /* PNG_WRITE_FLUSH_AFTER_IEND_SUPPORTED is not defined in this build. */
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_convert_from_struct_tm(ptime: png_timep, ttime: *const tm) {
    unsafe {
        (*ptime).year = (1900 + (*ttime).tm_year) as png_uint_16;
        (*ptime).month = ((*ttime).tm_mon + 1) as png_byte;
        (*ptime).day = (*ttime).tm_mday as png_byte;
        (*ptime).hour = (*ttime).tm_hour as png_byte;
        (*ptime).minute = (*ttime).tm_min as png_byte;
        (*ptime).second = (*ttime).tm_sec as png_byte;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_convert_from_time_t(ptime: png_timep, ttime: i64) {
    unsafe {
        let tbuf: *mut tm = gmtime(&ttime);
        if tbuf.is_null() {
            /* TODO: add a safe function which takes a png_ptr argument and raises
             * a png_error if the ttime argument is invalid and the call to gmtime
             * fails as a consequence.
             */
            memset(ptime as *mut c_void, 0, core::mem::size_of::<png_time>());
            return;
        }

        png_convert_from_struct_tm(ptime, tbuf);
    }
}

/* Initialize png_ptr structure, and allocate any memory needed */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_create_write_struct(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
) -> png_structp {
    unsafe {
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
}

/* Alternate initialize png_ptr structure, and allocate any memory needed */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_create_write_struct_2(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
    mem_ptr: png_voidp,
    malloc_fn: png_malloc_ptr,
    free_fn: png_free_ptr,
) -> png_structp {
    unsafe {
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

            (*png_ptr).zlib_text_strategy = PNG_TEXT_Z_DEFAULT_STRATEGY;
            (*png_ptr).zlib_text_level = PNG_TEXT_Z_DEFAULT_COMPRESSION;
            (*png_ptr).zlib_text_mem_level = 8;
            (*png_ptr).zlib_text_window_bits = 15;
            (*png_ptr).zlib_text_method = 8;

            /* This is a highly dubious configuration option; by default it is off,
             * but it may be appropriate for private builds that are testing
             * extensions not conformant to the current specification, or of
             * applications that must not fail to write at all costs!
             */
            /* PNG_BENIGN_WRITE_ERRORS_SUPPORTED is off in this build. */

            /* App warnings are warnings in release (or release candidate) builds but
             * are errors during development.
             */
            /* PNG_RELEASE_BUILD is 0 in this build. */

            /* TODO: delay this, it can be done in png_init_io() (if the app doesn't
             * do it itself) avoiding setting the default function if it is not
             * required.
             */
            png_set_write_fn(png_ptr, core::ptr::null_mut(), None, None);
        }

        png_ptr
    }
}

/* Write a few rows of image data.  If the image is interlaced,
 * either you will have to write the 7 sub images, or, if you
 * have called png_set_interlace_handling(), you will have to
 * "write" the image seven times.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_rows(
    png_ptr: png_structrp,
    row: png_bytepp,
    num_rows: png_uint_32,
) {
    unsafe {
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
            rp = rp.offset(1);
        }
    }
}

/* Write the image.  You only need to call this function once, even
 * if you are writing an interlaced image.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_image(png_ptr: png_structrp, image: png_bytepp) {
    unsafe {
        let mut i: png_uint_32; /* row index */
        let mut pass: c_int;
        let num_pass: c_int; /* pass variables */
        let mut rp: png_bytepp; /* points to current row */

        if png_ptr.is_null() {
            return;
        }

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
                rp = rp.offset(1);
            }
            pass += 1;
        }
    }
}

/* Performs intrapixel differencing  */
unsafe fn png_do_write_intrapixel(row_info: png_row_infop, row: png_bytep) {
    unsafe {
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
                    rp = rp.offset(bytes_per_pixel as isize);
                }
            } else if (*row_info).bit_depth == 16 {
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
                    let s0: png_uint_32 = ((*rp as png_uint_32) << 8) | *rp.add(1) as png_uint_32;
                    let s1: png_uint_32 =
                        ((*rp.add(2) as png_uint_32) << 8) | *rp.add(3) as png_uint_32;
                    let s2: png_uint_32 =
                        ((*rp.add(4) as png_uint_32) << 8) | *rp.add(5) as png_uint_32;
                    let red: png_uint_32 = s0.wrapping_sub(s1) & 0xffff;
                    let blue: png_uint_32 = s2.wrapping_sub(s1) & 0xffff;
                    *rp = (red >> 8) as png_byte;
                    *rp.add(1) = red as png_byte;
                    *rp.add(4) = (blue >> 8) as png_byte;
                    *rp.add(5) = blue as png_byte;
                    i += 1;
                    rp = rp.offset(bytes_per_pixel as isize);
                }
            }
        }
    }
}

/* Called by user to write a row of image data */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_row(png_ptr: png_structrp, row: png_const_bytep) {
    unsafe {
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
                    c"png_write_info was never called before png_write_row".as_ptr(),
                );
            }

            /* Check for transforms that have been set but were defined out:
             * all WRITE transforms are supported in this build, so none of the
             * "not defined" warnings survive the preprocessor.
             */

            png_write_start_row(png_ptr);
        }

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

                _ => {
                    /* error: ignore it */
                }
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

        /* Handle interlacing */
        if (*png_ptr).interlaced != 0
            && ((*png_ptr).pass as c_int) < 6
            && ((*png_ptr).transformations & PNG_INTERLACE) != 0
        {
            png_do_write_interlace(&mut row_info, (*png_ptr).row_buf.add(1), (*png_ptr).pass as c_int);
            /* This should always get caught above, but still ... */
            if row_info.width == 0 {
                png_write_finish_row(png_ptr);
                return;
            }
        }

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
            png_error(png_ptr, c"internal write transform logic error".as_ptr());
        }

        /* Write filter_method 64 (intrapixel differencing) only if
         * 1. Libpng was compiled with PNG_MNG_FEATURES_SUPPORTED and
         * 2. Libpng did not write a PNG signature (this filter_method is only
         *    used in PNG datastreams that are embedded in MNG datastreams) and
         * 3. The application called png_permit_mng_features with a mask that
         *    included PNG_FLAG_MNG_FILTER_64 and
         * 4. The filter_method is 64 and
         * 5. The color_type is RGB or RGBA
         */
        if ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
            && ((*png_ptr).filter_type as c_int == PNG_INTRAPIXEL_DIFFERENCING)
        {
            /* Intrapixel differencing */
            png_do_write_intrapixel(&mut row_info, (*png_ptr).row_buf.add(1));
        }

        /* Added at libpng-1.5.10 */
        /* Check for out-of-range palette index */
        if row_info.color_type as c_int == PNG_COLOR_TYPE_PALETTE && (*png_ptr).num_palette_max >= 0 {
            png_do_check_palette_indexes(png_ptr, &mut row_info);
        }

        /* Find a filter if necessary, filter the row and write it out. */
        png_write_find_filter(png_ptr, &mut row_info);

        if (*png_ptr).write_row_fn.is_some() {
            ((*png_ptr).write_row_fn.unwrap())(png_ptr, (*png_ptr).row_number, (*png_ptr).pass as c_int);
        }
    }
}

/* Set the automatic flush interval or 0 to turn flushing off */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_flush(png_ptr: png_structrp, nrows: c_int) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        (*png_ptr).flush_dist = if nrows < 0 { 0 } else { nrows as png_uint_32 };
    }
}

/* Flush the current output buffers now */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_flush(png_ptr: png_structrp) {
    unsafe {
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
}

/* Free any memory used in png_ptr struct without freeing the struct itself. */
unsafe fn png_write_destroy(png_ptr: png_structrp) {
    unsafe {
        /* Free any memory zlib uses */
        if ((*png_ptr).flags & PNG_FLAG_ZSTREAM_INITIALIZED) != 0 {
            deflateEnd(&raw mut (*png_ptr).zstream);
        }

        /* Free our memory.  png_free checks NULL for us. */
        png_free_buffer_list(png_ptr, &raw mut (*png_ptr).zbuffer_list);
        png_free(png_ptr, (*png_ptr).row_buf as png_voidp);
        (*png_ptr).row_buf = core::ptr::null_mut();
        png_free(png_ptr, (*png_ptr).prev_row as png_voidp);
        png_free(png_ptr, (*png_ptr).try_row as png_voidp);
        png_free(png_ptr, (*png_ptr).tst_row as png_voidp);
        (*png_ptr).prev_row = core::ptr::null_mut();
        (*png_ptr).try_row = core::ptr::null_mut();
        (*png_ptr).tst_row = core::ptr::null_mut();

        png_free(png_ptr, (*png_ptr).chunk_list as png_voidp);
        (*png_ptr).chunk_list = core::ptr::null_mut();

        /* Free the independent copy of trans_alpha owned by png_struct. */
        png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
        (*png_ptr).trans_alpha = core::ptr::null_mut();

        /* Free the independent copy of the palette owned by png_struct. */
        png_free(png_ptr, (*png_ptr).palette as png_voidp);
        (*png_ptr).palette = core::ptr::null_mut();

        /* The error handling and memory handling information is left intact at this
         * point: the jmp_buf may still have to be freed.  See png_destroy_png_struct
         * for how this happens.
         */
    }
}

/* Free all memory used by the write.
 * In libpng 1.6.0 this API changed quietly to no longer accept a NULL value for
 * *png_ptr_ptr.  Prior to 1.6.0 it would accept such a value and it would free
 * the passed in info_structs but it would quietly fail to free any of the data
 * inside them.  In 1.6.0 it quietly does nothing (it has to be quiet because it
 * has no png_ptr.)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_destroy_write_struct(
    png_ptr_ptr: png_structpp,
    info_ptr_ptr: png_infopp,
) {
    unsafe {
        if !png_ptr_ptr.is_null() {
            let png_ptr: png_structrp = *png_ptr_ptr;

            if !png_ptr.is_null()
            /* added in libpng 1.6.0 */
            {
                png_destroy_info_struct(png_ptr, info_ptr_ptr);

                *png_ptr_ptr = core::ptr::null_mut();
                png_write_destroy(png_ptr);
                png_destroy_png_struct(png_ptr);
            }
        }
    }
}

/* Allow the application to select one or more row filters to use. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_filter(
    png_ptr: png_structrp,
    mut method: c_int,
    mut filters: c_int,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        if ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
            && (method == PNG_INTRAPIXEL_DIFFERENCING)
        {
            method = PNG_FILTER_TYPE_BASE;
        }

        if method == PNG_FILTER_TYPE_BASE {
            match filters & (PNG_ALL_FILTERS | 0x07) {
                5 | 6 | 7 => {
                    png_app_error(png_ptr, c"Unknown row filter for method 0".as_ptr());
                    /* FALLTHROUGH */
                    (*png_ptr).do_filter = PNG_FILTER_NONE as png_byte;
                }
                PNG_FILTER_VALUE_NONE => {
                    (*png_ptr).do_filter = PNG_FILTER_NONE as png_byte;
                }

                PNG_FILTER_VALUE_SUB => {
                    (*png_ptr).do_filter = PNG_FILTER_SUB as png_byte;
                }

                PNG_FILTER_VALUE_UP => {
                    (*png_ptr).do_filter = PNG_FILTER_UP as png_byte;
                }

                PNG_FILTER_VALUE_AVG => {
                    (*png_ptr).do_filter = PNG_FILTER_AVG as png_byte;
                }

                PNG_FILTER_VALUE_PAETH => {
                    (*png_ptr).do_filter = PNG_FILTER_PAETH as png_byte;
                }

                _ => {
                    (*png_ptr).do_filter = filters as png_byte;
                }
            }

            /* If we have allocated the row_buf, this means we have already started
             * with the image and we should have allocated all of the filter buffers
             * that have been selected.  If prev_row isn't already allocated, then
             * it is too late to start using the filters that need it, since we
             * will be missing the data in the previous row.  If an application
             * wants to start and stop using particular filters during compression,
             * it should start out with all of the filters, and then remove them
             * or add them back after the start of compression.
             *
             * NOTE: this is a nasty constraint on the code, because it means that the
             * prev_row buffer must be maintained even if there are currently no
             * 'prev_row' requiring filters active.
             */
            if !(*png_ptr).row_buf.is_null() {
                let mut num_filters: c_int;
                let buf_size: png_alloc_size_t;

                /* Repeat the checks in png_write_start_row; 1 pixel high or wide
                 * images cannot benefit from certain filters.  If this isn't done here
                 * the check below will fire on 1 pixel high images.
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
                        c"png_set_filter: UP/AVG/PAETH cannot be added after start".as_ptr(),
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
                buf_size = PNG_ROWBYTES(
                    ((*png_ptr).usr_channels as c_int * (*png_ptr).usr_bit_depth as c_int) as usize,
                    (*png_ptr).width as usize,
                ) + 1;

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
            png_error(png_ptr, c"Unknown custom filter method".as_ptr());
        }
    }
}

/* DEPRECATED */
/* Provide floating and fixed point APIs */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_filter_heuristics(
    png_ptr: png_structrp,
    heuristic_method: c_int,
    num_weights: c_int,
    filter_weights: png_const_doublep,
    filter_costs: png_const_doublep,
) {
    PNG_UNUSED(png_ptr);
    PNG_UNUSED(heuristic_method);
    PNG_UNUSED(num_weights);
    PNG_UNUSED(filter_weights);
    PNG_UNUSED(filter_costs);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_filter_heuristics_fixed(
    png_ptr: png_structrp,
    heuristic_method: c_int,
    num_weights: c_int,
    filter_weights: png_const_fixed_point_p,
    filter_costs: png_const_fixed_point_p,
) {
    PNG_UNUSED(png_ptr);
    PNG_UNUSED(heuristic_method);
    PNG_UNUSED(num_weights);
    PNG_UNUSED(filter_weights);
    PNG_UNUSED(filter_costs);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_compression_level(png_ptr: png_structrp, level: c_int) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        (*png_ptr).zlib_level = level;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_compression_mem_level(
    png_ptr: png_structrp,
    mem_level: c_int,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        (*png_ptr).zlib_mem_level = mem_level;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_compression_strategy(
    png_ptr: png_structrp,
    strategy: c_int,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        /* The flag setting here prevents the libpng dynamic selection of strategy. */
        (*png_ptr).flags |= PNG_FLAG_ZLIB_CUSTOM_STRATEGY;
        (*png_ptr).zlib_strategy = strategy;
    }
}

/* If PNG_WRITE_OPTIMIZE_CMF_SUPPORTED is defined, libpng will use a
 * smaller value of window_bits if it can do so safely.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_compression_window_bits(
    png_ptr: png_structrp,
    mut window_bits: c_int,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        /* Prior to 1.6.0 this would warn but then set the window_bits value. This
         * meant that negative window bits values could be selected that would cause
         * libpng to write a non-standard PNG file with raw deflate or gzip
         * compressed IDAT or ancillary chunks.  Such files can be read and there is
         * no warning on read, so this seems like a very bad idea.
         */
        if window_bits > 15 {
            png_warning(png_ptr, c"Only compression windows <= 32k supported by PNG".as_ptr());
            window_bits = 15;
        } else if window_bits < 8 {
            png_warning(png_ptr, c"Only compression windows >= 256 supported by PNG".as_ptr());
            window_bits = 8;
        }

        (*png_ptr).zlib_window_bits = window_bits;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_compression_method(png_ptr: png_structrp, method: c_int) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        /* This would produce an invalid PNG file if it worked, but it doesn't and
         * deflate will fault it, so it is harmless to just warn here.
         */
        if method != 8 {
            png_warning(png_ptr, c"Only compression method 8 is supported by PNG".as_ptr());
        }

        (*png_ptr).zlib_method = method;
    }
}

/* The following were added to libpng-1.5.4 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_text_compression_level(
    png_ptr: png_structrp,
    level: c_int,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        (*png_ptr).zlib_text_level = level;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_text_compression_mem_level(
    png_ptr: png_structrp,
    mem_level: c_int,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        (*png_ptr).zlib_text_mem_level = mem_level;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_text_compression_strategy(
    png_ptr: png_structrp,
    strategy: c_int,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        (*png_ptr).zlib_text_strategy = strategy;
    }
}

/* If PNG_WRITE_OPTIMIZE_CMF_SUPPORTED is defined, libpng will use a
 * smaller value of window_bits if it can do so safely.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_text_compression_window_bits(
    png_ptr: png_structrp,
    mut window_bits: c_int,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        if window_bits > 15 {
            png_warning(png_ptr, c"Only compression windows <= 32k supported by PNG".as_ptr());
            window_bits = 15;
        } else if window_bits < 8 {
            png_warning(png_ptr, c"Only compression windows >= 256 supported by PNG".as_ptr());
            window_bits = 8;
        }

        (*png_ptr).zlib_text_window_bits = window_bits;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_text_compression_method(
    png_ptr: png_structrp,
    method: c_int,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        if method != 8 {
            png_warning(png_ptr, c"Only compression method 8 is supported by PNG".as_ptr());
        }

        (*png_ptr).zlib_text_method = method;
    }
}
/* end of API added to libpng-1.5.4 */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_write_status_fn(
    png_ptr: png_structrp,
    write_row_fn: png_write_status_ptr,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        (*png_ptr).write_row_fn = write_row_fn;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_write_user_transform_fn(
    png_ptr: png_structrp,
    write_user_transform_fn: png_user_transform_ptr,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        (*png_ptr).transformations |= PNG_USER_TRANSFORM;
        (*png_ptr).write_user_transform_fn = write_user_transform_fn;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_png(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    transforms: c_int,
    params: png_voidp,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        if ((*info_ptr).valid & PNG_INFO_IDAT) == 0 {
            png_app_error(png_ptr, c"no rows for png_write_image to write".as_ptr());
            return;
        }

        /* Write the file header information. */
        png_write_info(png_ptr, info_ptr);

        /* ------ these transformations don't touch the info structure ------- */

        /* Invert monochrome pixels */
        if (transforms & PNG_TRANSFORM_INVERT_MONO) != 0 {
            png_set_invert_mono(png_ptr);
        }

        /* Shift the pixels up to a legal bit depth and fill in
         * as appropriate to correctly scale the image.
         */
        if (transforms & PNG_TRANSFORM_SHIFT) != 0 {
            if ((*info_ptr).valid & PNG_INFO_sBIT) != 0 {
                png_set_shift(png_ptr, &raw const (*info_ptr).sig_bit);
            }
        }

        /* Pack pixels into bytes */
        if (transforms & PNG_TRANSFORM_PACKING) != 0 {
            png_set_packing(png_ptr);
        }

        /* Swap location of alpha bytes from ARGB to RGBA */
        if (transforms & PNG_TRANSFORM_SWAP_ALPHA) != 0 {
            png_set_swap_alpha(png_ptr);
        }

        /* Remove a filler (X) from XRGB/RGBX/AG/GA into to convert it into
         * RGB, note that the code expects the input color type to be G or RGB; no
         * alpha channel.
         */
        if (transforms & (PNG_TRANSFORM_STRIP_FILLER_AFTER | PNG_TRANSFORM_STRIP_FILLER_BEFORE)) != 0
        {
            if (transforms & PNG_TRANSFORM_STRIP_FILLER_AFTER) != 0 {
                if (transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE) != 0 {
                    png_app_error(
                        png_ptr,
                        c"PNG_TRANSFORM_STRIP_FILLER: BEFORE+AFTER not supported".as_ptr(),
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
            png_set_bgr(png_ptr);
        }

        /* Swap bytes of 16-bit files to most significant byte first */
        if (transforms & PNG_TRANSFORM_SWAP_ENDIAN) != 0 {
            png_set_swap(png_ptr);
        }

        /* Swap bits of 1-bit, 2-bit, 4-bit packed pixel formats */
        if (transforms & PNG_TRANSFORM_PACKSWAP) != 0 {
            png_set_packswap(png_ptr);
        }

        /* Invert the alpha channel from opacity to transparency */
        if (transforms & PNG_TRANSFORM_INVERT_ALPHA) != 0 {
            png_set_invert_alpha(png_ptr);
        }

        /* ----------------------- end of transformations ------------------- */

        /* Write the bits */
        png_write_image(png_ptr, (*info_ptr).row_pointers);

        /* It is REQUIRED to call this to finish writing the rest of the file */
        png_write_end(png_ptr, info_ptr);

        PNG_UNUSED(params);
    }
}

/* Initialize the write structure - general purpose utility. */
unsafe fn png_image_write_init(image: png_imagep) -> c_int {
    unsafe {
        let mut png_ptr: png_structp = png_create_write_struct(
            PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
            image as png_voidp,
            Some(png_safe_error),
            Some(png_safe_warning),
        );

        if !png_ptr.is_null() {
            let mut info_ptr: png_infop = png_create_info_struct(png_ptr);

            if !info_ptr.is_null() {
                let control: png_controlp =
                    png_malloc_warn(png_ptr, core::mem::size_of::<png_control>()) as png_controlp;

                if !control.is_null() {
                    memset(control as *mut c_void, 0, core::mem::size_of::<png_control>());

                    (*control).png_ptr = png_ptr;
                    (*control).info_ptr = info_ptr;
                    (*control).set_for_write(1);

                    (*image).opaque = control;
                    return 1;
                }

                /* Error clean up */
                png_destroy_info_struct(png_ptr, &raw mut info_ptr);
            }

            png_destroy_write_struct(&raw mut png_ptr, core::ptr::null_mut());
        }

        png_image_error(image, c"png_image_write_: out of memory".as_ptr())
    }
}

/* Arguments to png_image_write_main: */
#[repr(C)]
struct png_image_write_control {
    /* Arguments */
    image: png_imagep,
    buffer: png_const_voidp,
    row_stride: png_int_32,
    colormap: png_const_voidp,
    convert_to_8bit: c_int,

    /* Instance variables */
    first_row: png_const_voidp,
    local_row: png_voidp,
    row_step: isize, /* ptrdiff_t */

    /* Byte count for memory writing */
    memory: png_bytep,
    memory_bytes: png_alloc_size_t, /* not used for STDIO */
    output_bytes: png_alloc_size_t, /* running total */
}

/* Write png_uint_16 input to a 16-bit PNG; the png_ptr has already been set to
 * do any necessary byte swapping.  The component order is defined by the
 * png_image format value.
 */
unsafe extern "C-unwind" fn png_write_image_16bit(argument: png_voidp) -> c_int {
    unsafe {
        let display: *mut png_image_write_control = argument as *mut png_image_write_control;
        let image: png_imagep = (*display).image;
        let png_ptr: png_structrp = (*(*image).opaque).png_ptr;

        let mut input_row: png_const_uint_16p = (*display).first_row as png_const_uint_16p;
        let mut output_row: png_uint_16p = (*display).local_row as png_uint_16p;
        let row_end: png_uint_16p;
        let channels: c_uint = if ((*image).format & PNG_FORMAT_FLAG_COLOR) != 0 {
            3
        } else {
            1
        };
        let mut aindex: c_int = 0;
        let mut y: png_uint_32 = (*image).height;

        if ((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0 {
            if ((*image).format & PNG_FORMAT_FLAG_AFIRST) != 0 {
                aindex = -1;
                input_row = input_row.add(1); /* To point to the first component */
                output_row = output_row.add(1);
            } else {
                aindex = channels as c_int;
            }
        } else {
            png_error(png_ptr, c"png_write_image: internal call error".as_ptr());
        }

        /* Work out the output row end and count over this, note that the increment
         * above to 'row' means that row_end can actually be beyond the end of the
         * row; this is correct.
         */
        row_end = output_row.add(((*image).width * (channels + 1)) as usize);

        while y > 0 {
            let mut in_ptr: png_const_uint_16p = input_row;
            let mut out_ptr: png_uint_16p = output_row;

            while out_ptr < row_end {
                let alpha: png_uint_16 = *in_ptr.offset(aindex as isize);
                let mut reciprocal: png_uint_32 = 0;
                let mut c: c_int;

                *out_ptr.offset(aindex as isize) = alpha;

                /* Calculate a reciprocal.  The correct calculation is simply
                 * component/alpha*65535 << 15. (I.e. 15 bits of precision); this
                 * allows correct rounding by adding .5 before the shift.  'reciprocal'
                 * is only initialized when required.
                 */
                if alpha > 0 && alpha < 65535 {
                    reciprocal = ((0xffffu32 << 15) + (alpha as png_uint_32 >> 1)) / alpha as png_uint_32;
                }

                c = channels as c_int;
                loop
                /* always at least one channel */
                {
                    let mut component: png_uint_16 = *in_ptr;
                    in_ptr = in_ptr.add(1);

                    /* The following gives 65535 for an alpha of 0, which is fine,
                     * otherwise if 0/0 is represented as some other value there is more
                     * likely to be a discontinuity which will probably damage
                     * compression when moving from a fully transparent area to a
                     * nearly transparent one.  (The assumption here is that opaque
                     * areas tend not to be 0 intensity.)
                     */
                    if component >= alpha {
                        component = 65535;
                    }
                    /* component<alpha, so component/alpha is less than one and
                     * component*reciprocal is less than 2^31.
                     */
                    else if component > 0 && alpha < 65535 {
                        let mut calc: png_uint_32 = component as png_uint_32 * reciprocal;
                        calc += 16384; /* round to nearest */
                        component = (calc >> 15) as png_uint_16;
                    }

                    *out_ptr = component;
                    out_ptr = out_ptr.add(1);

                    c -= 1;
                    if c <= 0 {
                        break;
                    }
                }

                /* Skip to next component (skip the intervening alpha channel) */
                in_ptr = in_ptr.add(1);
                out_ptr = out_ptr.add(1);
            }

            png_write_row(png_ptr, (*display).local_row as png_const_bytep);
            input_row = input_row.offset(((*display).row_step / 2) as isize);

            y -= 1;
        }

        1
    }
}

/* Given 16-bit input (1 to 4 channels) write 8-bit output.  If an alpha channel
 * is present it must be removed from the components, the components are then
 * written in sRGB encoding.  No components are added or removed.
 *
 * Calculate an alpha reciprocal to reverse pre-multiplication.  As above the
 * calculation can be done to 15 bits of accuracy; however, the output needs to
 * be scaled in the range 0..255*65535, so include that scaling here.
 */
#[inline]
fn UNP_RECIPROCAL(alpha: png_uint_32) -> png_uint_32 {
    (((0xffffu32 * 0xff) << 7) + (alpha >> 1)) / alpha
}

unsafe fn png_unpremultiply(
    mut component: png_uint_32,
    alpha: png_uint_32,
    reciprocal: png_uint_32, /*from the above macro*/
) -> png_byte {
    unsafe {
        /* The following gives 1.0 for an alpha of 0, which is fine, otherwise if 0/0
         * is represented as some other value there is more likely to be a
         * discontinuity which will probably damage compression when moving from a
         * fully transparent area to a nearly transparent one.  (The assumption here
         * is that opaque areas tend not to be 0 intensity.)
         *
         * There is a rounding problem here; if alpha is less than 128 it will end up
         * as 0 when scaled to 8 bits.  To avoid introducing spurious colors into the
         * output change for this too.
         */
        if component >= alpha || alpha < 128 {
            255
        }
        /* component<alpha, so component/alpha is less than one and
         * component*reciprocal is less than 2^31.
         */
        else if component > 0 {
            /* The test is that alpha/257 (rounded) is less than 255, the first value
             * that becomes 255 is 65407.
             * NOTE: this must agree with the PNG_DIV257 macro (which must, therefore,
             * be exact!)  [Could also test reciprocal != 0]
             */
            if alpha < 65407 {
                component *= reciprocal;
                component += 64; /* round to nearest */
                component >>= 7;
            } else {
                component *= 255;
            }

            /* Convert the component to sRGB. */
            PNG_sRGB_FROM_LINEAR(component)
        } else {
            0
        }
    }
}

unsafe extern "C-unwind" fn png_write_image_8bit(argument: png_voidp) -> c_int {
    unsafe {
        let display: *mut png_image_write_control = argument as *mut png_image_write_control;
        let image: png_imagep = (*display).image;
        let png_ptr: png_structrp = (*(*image).opaque).png_ptr;

        let mut input_row: png_const_uint_16p = (*display).first_row as png_const_uint_16p;
        let mut output_row: png_bytep = (*display).local_row as png_bytep;
        let mut y: png_uint_32 = (*image).height;
        let channels: c_uint = if ((*image).format & PNG_FORMAT_FLAG_COLOR) != 0 {
            3
        } else {
            1
        };

        if ((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0 {
            let row_end: png_bytep;
            let aindex: c_int;

            if ((*image).format & PNG_FORMAT_FLAG_AFIRST) != 0 {
                aindex = -1;
                input_row = input_row.add(1); /* To point to the first component */
                output_row = output_row.add(1);
            } else {
                aindex = channels as c_int;
            }

            /* Use row_end in place of a loop counter: */
            row_end = output_row.add(((*image).width * (channels + 1)) as usize);

            while y > 0 {
                let mut in_ptr: png_const_uint_16p = input_row;
                let mut out_ptr: png_bytep = output_row;

                while out_ptr < row_end {
                    let alpha: png_uint_16 = *in_ptr.offset(aindex as isize);
                    let alphabyte: png_byte = PNG_DIV257(alpha as png_uint_32) as png_byte;
                    let mut reciprocal: png_uint_32 = 0;
                    let mut c: c_int;

                    /* Scale and write the alpha channel. */
                    *out_ptr.offset(aindex as isize) = alphabyte;

                    if alphabyte > 0 && alphabyte < 255 {
                        reciprocal = UNP_RECIPROCAL(alpha as png_uint_32);
                    }

                    c = channels as c_int;
                    loop
                    /* always at least one channel */
                    {
                        *out_ptr = png_unpremultiply(*in_ptr as png_uint_32, alpha as png_uint_32, reciprocal);
                        in_ptr = in_ptr.add(1);
                        out_ptr = out_ptr.add(1);
                        c -= 1;
                        if c <= 0 {
                            break;
                        }
                    }

                    /* Skip to next component (skip the intervening alpha channel) */
                    in_ptr = in_ptr.add(1);
                    out_ptr = out_ptr.add(1);
                } /* while out_ptr < row_end */

                png_write_row(png_ptr, (*display).local_row as png_const_bytep);
                input_row = input_row.offset(((*display).row_step / 2) as isize);

                y -= 1;
            } /* while y */
        } else {
            /* No alpha channel, so the row_end really is the end of the row and it
             * is sufficient to loop over the components one by one.
             */
            let row_end: png_bytep = output_row.add(((*image).width * channels) as usize);

            while y > 0 {
                let mut in_ptr: png_const_uint_16p = input_row;
                let mut out_ptr: png_bytep = output_row;

                while out_ptr < row_end {
                    let mut component: png_uint_32 = *in_ptr as png_uint_32;
                    in_ptr = in_ptr.add(1);

                    component *= 255;
                    *out_ptr = PNG_sRGB_FROM_LINEAR(component);
                    out_ptr = out_ptr.add(1);
                }

                png_write_row(png_ptr, output_row);
                input_row = input_row.offset(((*display).row_step / 2) as isize);

                y -= 1;
            }
        }

        1
    }
}

unsafe fn png_image_set_PLTE(display: *mut png_image_write_control) {
    unsafe {
        let image: png_imagep = (*display).image;
        let cmap: *const c_void = (*display).colormap;
        let entries: c_int = if (*image).colormap_entries > 256 {
            256
        } else {
            (*image).colormap_entries as c_int
        };

        /* NOTE: the caller must check for cmap != NULL and entries != 0 */
        let format: png_uint_32 = (*image).format;
        let channels: c_uint = PNG_IMAGE_SAMPLE_CHANNELS(format);

        let afirst: c_int = ((format & PNG_FORMAT_FLAG_AFIRST) != 0
            && (format & PNG_FORMAT_FLAG_ALPHA) != 0) as c_int;

        let bgr: c_int = if (format & PNG_FORMAT_FLAG_BGR) != 0 {
            2
        } else {
            0
        };

        let mut i: c_int;
        let mut num_trans: c_int;
        let mut palette: [png_color; 256] = [png_color::default(); 256];
        let mut tRNS: [png_byte; 256] = [0; 256];

        memset(tRNS.as_mut_ptr() as *mut c_void, 255, core::mem::size_of_val(&tRNS));
        memset(palette.as_mut_ptr() as *mut c_void, 0, core::mem::size_of_val(&palette));

        i = 0;
        num_trans = 0;
        while i < entries {
            /* This gets automatically converted to sRGB with reversal of the
             * pre-multiplication if the color-map has an alpha channel.
             */
            if (format & PNG_FORMAT_FLAG_LINEAR) != 0 {
                let mut entry: png_const_uint_16p = cmap as png_const_uint_16p;

                entry = entry.add((i as c_uint * channels) as usize);

                if (channels & 1) != 0
                /* no alpha */
                {
                    if channels >= 3
                    /* RGB */
                    {
                        palette[i as usize].blue =
                            PNG_sRGB_FROM_LINEAR(255 * *entry.offset((2 ^ bgr) as isize) as png_uint_32);
                        palette[i as usize].green =
                            PNG_sRGB_FROM_LINEAR(255 * *entry.offset(1) as png_uint_32);
                        palette[i as usize].red =
                            PNG_sRGB_FROM_LINEAR(255 * *entry.offset(bgr as isize) as png_uint_32);
                    } else
                    /* Gray */
                    {
                        let v = PNG_sRGB_FROM_LINEAR(255 * *entry as png_uint_32);
                        palette[i as usize].green = v;
                        palette[i as usize].red = v;
                        palette[i as usize].blue = v;
                    }
                } else
                /* alpha */
                {
                    let alpha: png_uint_16 =
                        *entry.offset(if afirst != 0 { 0 } else { channels as isize - 1 });
                    let alphabyte: png_byte = PNG_DIV257(alpha as png_uint_32) as png_byte;
                    let mut reciprocal: png_uint_32 = 0;

                    /* Calculate a reciprocal, as in the png_write_image_8bit code above
                     * this is designed to produce a value scaled to 255*65535 when
                     * divided by 128 (i.e. asr 7).
                     */
                    if alphabyte > 0 && alphabyte < 255 {
                        reciprocal =
                            (((0xffffu32 * 0xff) << 7) + (alpha as png_uint_32 >> 1)) / alpha as png_uint_32;
                    }

                    tRNS[i as usize] = alphabyte;
                    if alphabyte < 255 {
                        num_trans = i + 1;
                    }

                    if channels >= 3
                    /* RGB */
                    {
                        palette[i as usize].blue = png_unpremultiply(
                            *entry.offset((afirst + (2 ^ bgr)) as isize) as png_uint_32,
                            alpha as png_uint_32,
                            reciprocal,
                        );
                        palette[i as usize].green = png_unpremultiply(
                            *entry.offset((afirst + 1) as isize) as png_uint_32,
                            alpha as png_uint_32,
                            reciprocal,
                        );
                        palette[i as usize].red = png_unpremultiply(
                            *entry.offset((afirst + bgr) as isize) as png_uint_32,
                            alpha as png_uint_32,
                            reciprocal,
                        );
                    } else
                    /* gray */
                    {
                        let v = png_unpremultiply(
                            *entry.offset(afirst as isize) as png_uint_32,
                            alpha as png_uint_32,
                            reciprocal,
                        );
                        palette[i as usize].green = v;
                        palette[i as usize].red = v;
                        palette[i as usize].blue = v;
                    }
                }
            } else
            /* Color-map has sRGB values */
            {
                let mut entry: png_const_bytep = cmap as png_const_bytep;

                entry = entry.add((i as c_uint * channels) as usize);

                match channels {
                    4 => {
                        tRNS[i as usize] = *entry.offset(if afirst != 0 { 0 } else { 3 });
                        if tRNS[i as usize] < 255 {
                            num_trans = i + 1;
                        }
                        /* FALLTHROUGH */
                        palette[i as usize].blue = *entry.offset((afirst + (2 ^ bgr)) as isize);
                        palette[i as usize].green = *entry.offset((afirst + 1) as isize);
                        palette[i as usize].red = *entry.offset((afirst + bgr) as isize);
                    }
                    3 => {
                        palette[i as usize].blue = *entry.offset((afirst + (2 ^ bgr)) as isize);
                        palette[i as usize].green = *entry.offset((afirst + 1) as isize);
                        palette[i as usize].red = *entry.offset((afirst + bgr) as isize);
                    }

                    2 => {
                        tRNS[i as usize] = *entry.offset((1 ^ afirst) as isize);
                        if tRNS[i as usize] < 255 {
                            num_trans = i + 1;
                        }
                        /* FALLTHROUGH */
                        let v = *entry.offset(afirst as isize);
                        palette[i as usize].green = v;
                        palette[i as usize].red = v;
                        palette[i as usize].blue = v;
                    }
                    1 => {
                        let v = *entry.offset(afirst as isize);
                        palette[i as usize].green = v;
                        palette[i as usize].red = v;
                        palette[i as usize].blue = v;
                    }

                    _ => {}
                }
            }

            i += 1;
        }

        png_set_PLTE(
            (*(*image).opaque).png_ptr,
            (*(*image).opaque).info_ptr,
            palette.as_ptr(),
            entries,
        );

        if num_trans > 0 {
            png_set_tRNS(
                (*(*image).opaque).png_ptr,
                (*(*image).opaque).info_ptr,
                tRNS.as_ptr(),
                num_trans,
                core::ptr::null(),
            );
        }

        (*image).colormap_entries = entries as png_uint_32;
    }
}

unsafe extern "C-unwind" fn png_image_write_main(argument: png_voidp) -> c_int {
    unsafe {
        let display: *mut png_image_write_control = argument as *mut png_image_write_control;
        let image: png_imagep = (*display).image;
        let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
        let info_ptr: png_inforp = (*(*image).opaque).info_ptr;
        let mut format: png_uint_32 = (*image).format;

        /* The following four ints are actually booleans */
        let colormap: c_int = (format & PNG_FORMAT_FLAG_COLORMAP) as c_int;
        let linear: c_int = (colormap == 0 && (format & PNG_FORMAT_FLAG_LINEAR) != 0) as c_int; /* input */
        let alpha: c_int = (colormap == 0 && (format & PNG_FORMAT_FLAG_ALPHA) != 0) as c_int;
        let write_16bit: c_int = (linear != 0 && ((*display).convert_to_8bit == 0)) as c_int;

        /* Make sure we error out on any bad situation */
        png_set_benign_errors(png_ptr, 0 /*error*/);

        /* Default the 'row_stride' parameter if required, also check the row stride
         * and total image size to ensure that they are within the system limits.
         */
        {
            let channels: c_uint = PNG_IMAGE_PIXEL_CHANNELS((*image).format);

            if (*image).width <= 0x7fffffffu32 / channels
            /* no overflow */
            {
                let check: png_uint_32;
                let png_row_stride: png_uint_32 = (*image).width * channels;

                if (*display).row_stride == 0 {
                    (*display).row_stride = png_row_stride as png_int_32; /*SAFE*/
                }

                if (*display).row_stride < 0 {
                    check = (0 as png_uint_32).wrapping_sub((*display).row_stride as png_uint_32);
                } else {
                    check = (*display).row_stride as png_uint_32;
                }

                if check >= png_row_stride {
                    /* Now check for overflow of the image buffer calculation; this
                     * limits the whole image size to 32 bits for API compatibility with
                     * the current, 32-bit, PNG_IMAGE_BUFFER_SIZE macro.
                     */
                    if (*image).height > 0xffffffffu32 / png_row_stride {
                        png_error((*(*image).opaque).png_ptr, c"memory image too large".as_ptr());
                    }
                } else {
                    png_error((*(*image).opaque).png_ptr, c"supplied row stride too small".as_ptr());
                }
            } else {
                png_error((*(*image).opaque).png_ptr, c"image row stride too large".as_ptr());
            }
        }

        /* Set the required transforms then write the rows in the correct order. */
        if (format & PNG_FORMAT_FLAG_COLORMAP) != 0 {
            if !(*display).colormap.is_null() && (*image).colormap_entries > 0 {
                let entries: png_uint_32 = (*image).colormap_entries;

                png_set_IHDR(
                    png_ptr,
                    info_ptr,
                    (*image).width,
                    (*image).height,
                    if entries > 16 {
                        8
                    } else if entries > 4 {
                        4
                    } else if entries > 2 {
                        2
                    } else {
                        1
                    },
                    PNG_COLOR_TYPE_PALETTE,
                    PNG_INTERLACE_NONE,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );

                png_image_set_PLTE(display);
            } else {
                png_error(
                    (*(*image).opaque).png_ptr,
                    c"no color-map for color-mapped image".as_ptr(),
                );
            }
        } else {
            png_set_IHDR(
                png_ptr,
                info_ptr,
                (*image).width,
                (*image).height,
                if write_16bit != 0 { 16 } else { 8 },
                (if (format & PNG_FORMAT_FLAG_COLOR) != 0 {
                    PNG_COLOR_MASK_COLOR
                } else {
                    0
                }) + (if (format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                    PNG_COLOR_MASK_ALPHA
                } else {
                    0
                }),
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
        }

        /* Counter-intuitively the data transformations must be called *after*
         * png_write_info, not before as in the read code, but the 'set' functions
         * must still be called before.  Just set the color space information, never
         * write an interlaced image.
         */

        if write_16bit != 0 {
            /* The gamma here is 1.0 (linear) and the cHRM chunk matches sRGB. */
            png_set_gAMA_fixed(png_ptr, info_ptr, PNG_GAMMA_LINEAR);

            if ((*image).flags & PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB) == 0 {
                png_set_cHRM_fixed(
                    png_ptr,
                    info_ptr,
                    /* color      x       y */
                    /* white */ 31270,
                    32900,
                    /* red   */ 64000,
                    33000,
                    /* green */ 30000,
                    60000,
                    /* blue  */ 15000,
                    6000,
                );
            }
        } else if ((*image).flags & PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB) == 0 {
            png_set_sRGB(png_ptr, info_ptr, PNG_sRGB_INTENT_PERCEPTUAL);
        }
        /* Else writing an 8-bit file and the *colors* aren't sRGB, but the 8-bit
         * space must still be gamma encoded.
         */
        else {
            png_set_gAMA_fixed(png_ptr, info_ptr, PNG_GAMMA_sRGB_INVERSE);
        }

        /* Write the file header. */
        png_write_info(png_ptr, info_ptr);

        /* Now set up the data transformations (*after* the header is written),
         * remove the handled transformations from the 'format' flags for checking.
         *
         * First check for a little endian system if writing 16-bit files.
         */
        if write_16bit != 0 {
            let le: png_uint_16 = 0x0001;

            if (*(&le as *const png_uint_16 as png_const_bytep) & le as png_byte) != 0 {
                png_set_swap(png_ptr);
            }
        }

        if (format & PNG_FORMAT_FLAG_BGR) != 0 {
            if colormap == 0 && (format & PNG_FORMAT_FLAG_COLOR) != 0 {
                png_set_bgr(png_ptr);
            }
            format &= !PNG_FORMAT_FLAG_BGR;
        }

        if (format & PNG_FORMAT_FLAG_AFIRST) != 0 {
            if colormap == 0 && (format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                png_set_swap_alpha(png_ptr);
            }
            format &= !PNG_FORMAT_FLAG_AFIRST;
        }

        /* If there are 16 or fewer color-map entries we wrote a lower bit depth
         * above, but the application data is still byte packed.
         */
        if colormap != 0 && (*image).colormap_entries <= 16 {
            png_set_packing(png_ptr);
        }

        /* That should have handled all (both) the transforms. */
        if (format
            & !(PNG_FORMAT_FLAG_COLOR
                | PNG_FORMAT_FLAG_LINEAR
                | PNG_FORMAT_FLAG_ALPHA
                | PNG_FORMAT_FLAG_COLORMAP))
            != 0
        {
            png_error(png_ptr, c"png_write_image: unsupported transformation".as_ptr());
        }

        {
            let mut row: png_const_bytep = (*display).buffer as png_const_bytep;
            let mut row_step: isize = (*display).row_stride as isize;

            if linear != 0 {
                row_step *= 2;
            }

            if row_step < 0 {
                row = row.offset(((*image).height - 1) as isize * (-row_step));
            }

            (*display).first_row = row as png_const_voidp;
            (*display).row_step = row_step;
        }

        /* Apply 'fast' options if the flag is set. */
        if ((*image).flags & PNG_IMAGE_FLAG_FAST) != 0 {
            png_set_filter(png_ptr, PNG_FILTER_TYPE_BASE, PNG_NO_FILTERS);
            /* NOTE: determined by experiment using pngstest, this reflects some
             * balance between the time to write the image once and the time to read
             * it about 50 times.  The speed-up in pngstest was about 10-20% of the
             * total (user) time on a heavily loaded system.
             */
            png_set_compression_level(png_ptr, 3);
        }

        /* Check for the cases that currently require a pre-transform on the row
         * before it is written.  This only applies when the input is 16-bit and
         * either there is an alpha channel or it is converted to 8-bit.
         */
        if linear != 0 && (alpha != 0 || (*display).convert_to_8bit != 0) {
            let row: png_bytep =
                png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr)) as png_bytep;
            let result: c_int;

            (*display).local_row = row as png_voidp;
            if write_16bit != 0 {
                result = png_safe_execute(image, Some(png_write_image_16bit), display as png_voidp);
            } else {
                result = png_safe_execute(image, Some(png_write_image_8bit), display as png_voidp);
            }
            (*display).local_row = core::ptr::null_mut();

            png_free(png_ptr, row as png_voidp);

            /* Skip the 'write_end' on error: */
            if result == 0 {
                return 0;
            }
        }
        /* Otherwise this is the case where the input is in a format currently
         * supported by the rest of the libpng write code; call it directly.
         */
        else {
            let mut row: png_const_bytep = (*display).first_row as png_const_bytep;
            let row_step: isize = (*display).row_step;
            let mut y: png_uint_32 = (*image).height;

            while y > 0 {
                png_write_row(png_ptr, row);
                row = row.offset(row_step);
                y -= 1;
            }
        }

        png_write_end(png_ptr, info_ptr);
        1
    }
}

unsafe extern "C-unwind" fn image_memory_write(
    png_ptr: png_structp,
    data: png_bytep,
    size: usize,
) {
    unsafe {
        let display: *mut png_image_write_control =
            (*png_ptr).io_ptr as *mut png_image_write_control /*backdoor: png_get_io_ptr(png_ptr)*/;
        let ob: png_alloc_size_t = (*display).output_bytes;

        /* Check for overflow; this should never happen: */
        if size <= (usize::MAX) - ob {
            /* I don't think libpng ever does this, but just in case: */
            if size > 0 {
                if (*display).memory_bytes >= ob + size
                /* writing */
                {
                    memcpy(
                        (*display).memory.add(ob) as *mut c_void,
                        data as *const c_void,
                        size,
                    );
                }

                /* Always update the size: */
                (*display).output_bytes = ob + size;
            }
        } else {
            png_error(png_ptr, c"png_image_write_to_memory: PNG too big".as_ptr());
        }
    }
}

unsafe extern "C-unwind" fn image_memory_flush(png_ptr: png_structp) {
    PNG_UNUSED(png_ptr);
}

unsafe extern "C-unwind" fn png_image_write_memory(argument: png_voidp) -> c_int {
    unsafe {
        let display: *mut png_image_write_control = argument as *mut png_image_write_control;

        /* The rest of the memory-specific init and write_main in an error protected
         * environment.  This case needs to use callbacks for the write operations
         * since libpng has no built in support for writing to memory.
         */
        png_set_write_fn(
            (*(*(*display).image).opaque).png_ptr,
            display as png_voidp, /*io_ptr*/
            Some(image_memory_write),
            Some(image_memory_flush),
        );

        png_image_write_main(display as png_voidp)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_image_write_to_memory(
    image: png_imagep,
    memory: *mut c_void,
    memory_bytes: *mut png_alloc_size_t,
    convert_to_8bit: c_int,
    buffer: *const c_void,
    row_stride: png_int_32,
    colormap: *const c_void,
) -> c_int {
    unsafe {
        /* Write the image to the given buffer, or count the bytes if it is NULL */
        if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
            if !memory_bytes.is_null() && !buffer.is_null() {
                /* This is to give the caller an easier error detection in the NULL
                 * case and guard against uninitialized variable problems:
                 */
                if memory.is_null() {
                    *memory_bytes = 0;
                }

                if png_image_write_init(image) != 0 {
                    let mut display: png_image_write_control = core::mem::zeroed();
                    let mut result: c_int;

                    memset(
                        &raw mut display as *mut c_void,
                        0,
                        core::mem::size_of::<png_image_write_control>(),
                    );
                    display.image = image;
                    display.buffer = buffer;
                    display.row_stride = row_stride;
                    display.colormap = colormap;
                    display.convert_to_8bit = convert_to_8bit;
                    display.memory = memory as png_bytep;
                    display.memory_bytes = *memory_bytes;
                    display.output_bytes = 0;

                    result = png_safe_execute(
                        image,
                        Some(png_image_write_memory),
                        &raw mut display as png_voidp,
                    );
                    png_image_free(image);

                    /* write_memory returns true even if we ran out of buffer. */
                    if result != 0 {
                        /* On out-of-buffer this function returns '0' but still updates
                         * memory_bytes:
                         */
                        if !memory.is_null() && display.output_bytes > *memory_bytes {
                            result = 0;
                        }

                        *memory_bytes = display.output_bytes;
                    }

                    result
                } else {
                    0
                }
            } else {
                png_image_error(image, c"png_image_write_to_memory: invalid argument".as_ptr())
            }
        } else if !image.is_null() {
            png_image_error(
                image,
                c"png_image_write_to_memory: incorrect PNG_IMAGE_VERSION".as_ptr(),
            )
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_image_write_to_stdio(
    image: png_imagep,
    file: *mut FILE,
    convert_to_8bit: c_int,
    buffer: *const c_void,
    row_stride: png_int_32,
    colormap: *const c_void,
) -> c_int {
    unsafe {
        /* Write the image to the given FILE object. */
        if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
            if !file.is_null() && !buffer.is_null() {
                if png_image_write_init(image) != 0 {
                    let mut display: png_image_write_control = core::mem::zeroed();
                    let result: c_int;

                    /* This is slightly evil, but png_init_io doesn't do anything other
                     * than this and we haven't changed the standard IO functions so
                     * this saves a 'safe' function.
                     */
                    (*(*(*image).opaque).png_ptr).io_ptr = file as png_voidp;

                    memset(
                        &raw mut display as *mut c_void,
                        0,
                        core::mem::size_of::<png_image_write_control>(),
                    );
                    display.image = image;
                    display.buffer = buffer;
                    display.row_stride = row_stride;
                    display.colormap = colormap;
                    display.convert_to_8bit = convert_to_8bit;

                    result = png_safe_execute(
                        image,
                        Some(png_image_write_main),
                        &raw mut display as png_voidp,
                    );
                    png_image_free(image);
                    result
                } else {
                    0
                }
            } else {
                png_image_error(image, c"png_image_write_to_stdio: invalid argument".as_ptr())
            }
        } else if !image.is_null() {
            png_image_error(
                image,
                c"png_image_write_to_stdio: incorrect PNG_IMAGE_VERSION".as_ptr(),
            )
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_image_write_to_file(
    image: png_imagep,
    file_name: *const c_char,
    convert_to_8bit: c_int,
    buffer: *const c_void,
    row_stride: png_int_32,
    colormap: *const c_void,
) -> c_int {
    unsafe {
        /* Write the image to the named file. */
        if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
            if !file_name.is_null() && !buffer.is_null() {
                let fp: *mut FILE = fopen(file_name, c"wb".as_ptr());

                if !fp.is_null() {
                    if png_image_write_to_stdio(image, fp, convert_to_8bit, buffer, row_stride, colormap)
                        != 0
                    {
                        let error: c_int; /* from fflush/fclose */

                        /* Make sure the file is flushed correctly. */
                        if fflush(fp) == 0 && ferror(fp) == 0 {
                            if fclose(fp) == 0 {
                                return 1;
                            }

                            error = *__errno_location(); /* from fclose */
                        } else {
                            error = *__errno_location(); /* from fflush or ferror */
                            fclose(fp);
                        }

                        remove(file_name);
                        /* The image has already been cleaned up; this is just used to
                         * set the error (because the original write succeeded).
                         */
                        return png_image_error(image, strerror(error));
                    } else {
                        /* Clean up: just the opened file. */
                        fclose(fp);
                        remove(file_name);
                        0
                    }
                } else {
                    png_image_error(image, strerror(*__errno_location()))
                }
            } else {
                png_image_error(image, c"png_image_write_to_file: invalid argument".as_ptr())
            }
        } else if !image.is_null() {
            png_image_error(
                image,
                c"png_image_write_to_file: incorrect PNG_IMAGE_VERSION".as_ptr(),
            )
        } else {
            0
        }
    }
}
