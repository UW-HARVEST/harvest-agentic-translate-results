/* pngwrite.c lines 1..391 */

/* Write out all the unknown chunks for the current given location */
/* write_unknown_chunks */
unsafe fn write_unknown_chunks(
    png_ptr: png_structrp,
    info_ptr: png_const_inforp,
    where_: c_uint,
) {
    if (*info_ptr).unknown_chunks_num != 0 {
        let mut up: png_const_unknown_chunkp;

        up = (*info_ptr).unknown_chunks as png_const_unknown_chunkp;
        while up
            < (*info_ptr)
                .unknown_chunks
                .offset((*info_ptr).unknown_chunks_num as isize) as png_const_unknown_chunkp
        {
            if ((*up).location as c_uint & where_) != 0 {
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
                    && (((*up).name[3] as c_int & 0x20) != 0 /* safe-to-copy overrides everything */
                        || keep == PNG_HANDLE_CHUNK_ALWAYS
                        || (keep == PNG_HANDLE_CHUNK_AS_DEFAULT
                            && (*png_ptr).unknown_default == PNG_HANDLE_CHUNK_ALWAYS))
                {
                    /* TODO: review, what is wrong with a zero length unknown chunk? */
                    if (*up).size == 0 {
                        png_warning(
                            png_ptr,
                            b"Writing zero-length unknown chunk\0".as_ptr() as png_const_charp,
                        );
                    }

                    png_write_chunk(
                        png_ptr,
                        (*up).name.as_ptr(),
                        (*up).data as png_const_bytep,
                        (*up).size,
                    );
                }
            }
            up = up.offset(1);
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
/* png_write_info_before_PLTE */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_info_before_PLTE(
    png_ptr: png_structrp,
    info_ptr: png_const_inforp,
) {
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
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
                b"MNG features are not allowed in a PNG datastream\0".as_ptr() as png_const_charp,
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
            png_write_sBIT(
                png_ptr,
                core::ptr::addr_of!((*info_ptr).sig_bit),
                (*info_ptr).color_type as c_int,
            );
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
                (*info_ptr).iccp_name as png_const_charp,
                (*info_ptr).iccp_profile as png_const_bytep,
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
            png_write_cHRM_fixed(png_ptr, core::ptr::addr_of!((*info_ptr).cHRM));
        }

        (*png_ptr).mode |= PNG_WROTE_INFO_BEFORE_PLTE;
    }
}

/* png_write_info */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_info(png_ptr: png_structrp, info_ptr: png_const_inforp) {
    let mut i: c_int;

    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    png_write_info_before_PLTE(png_ptr, info_ptr);

    if ((*info_ptr).valid & PNG_INFO_PLTE) != 0 {
        png_write_PLTE(
            png_ptr,
            (*info_ptr).palette as png_const_colorp,
            (*info_ptr).num_palette as png_uint_32,
        );
    } else if (*info_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        png_error(
            png_ptr,
            b"Valid palette required for paletted images\0".as_ptr() as png_const_charp,
        );
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
            (*info_ptr).trans_alpha as png_const_bytep,
            core::ptr::addr_of!((*info_ptr).trans_color),
            (*info_ptr).num_trans as c_int,
            (*info_ptr).color_type as c_int,
        );
    }

    if ((*info_ptr).valid & PNG_INFO_bKGD) != 0 {
        png_write_bKGD(
            png_ptr,
            core::ptr::addr_of!((*info_ptr).background),
            (*info_ptr).color_type as c_int,
        );
    }

    if ((*info_ptr).valid & PNG_INFO_eXIf) != 0 {
        png_write_eXIf(png_ptr, (*info_ptr).exif, (*info_ptr).num_exif as c_int);
        (*png_ptr).mode |= PNG_WROTE_eXIf;
    }

    if ((*info_ptr).valid & PNG_INFO_hIST) != 0 {
        png_write_hIST(
            png_ptr,
            (*info_ptr).hist as png_const_uint_16p,
            (*info_ptr).num_palette as c_int,
        );
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
            (*info_ptr).pcal_units as png_const_charp,
            (*info_ptr).pcal_params,
        );
    }

    if ((*info_ptr).valid & PNG_INFO_sCAL) != 0 {
        png_write_sCAL_s(
            png_ptr,
            (*info_ptr).scal_unit as c_int,
            (*info_ptr).scal_s_width as png_const_charp,
            (*info_ptr).scal_s_height as png_const_charp,
        );
    }
    /* sCAL */

    if ((*info_ptr).valid & PNG_INFO_pHYs) != 0 {
        png_write_pHYs(
            png_ptr,
            (*info_ptr).x_pixels_per_unit,
            (*info_ptr).y_pixels_per_unit,
            (*info_ptr).phys_unit_type as c_int,
        );
    }
    /* pHYs */

    if ((*info_ptr).valid & PNG_INFO_tIME) != 0 {
        png_write_tIME(png_ptr, core::ptr::addr_of!((*info_ptr).mod_time));
        (*png_ptr).mode |= PNG_WROTE_tIME;
    }
    /* tIME */

    if ((*info_ptr).valid & PNG_INFO_sPLT) != 0 {
        i = 0;
        while i < (*info_ptr).splt_palettes_num {
            png_write_sPLT(
                png_ptr,
                (*info_ptr).splt_palettes.offset(i as isize) as png_const_sPLT_tp,
            );
            i += 1;
        }
    }
    /* sPLT */

    /* Check to see if we need to write text chunks */
    i = 0;
    while i < (*info_ptr).num_text {
        /* An internationalized chunk? */
        if (*(*info_ptr).text.offset(i as isize)).compression > 0 {
            /* Write international chunk */
            png_write_iTXt(
                png_ptr,
                (*(*info_ptr).text.offset(i as isize)).compression,
                (*(*info_ptr).text.offset(i as isize)).key as png_const_charp,
                (*(*info_ptr).text.offset(i as isize)).lang as png_const_charp,
                (*(*info_ptr).text.offset(i as isize)).lang_key as png_const_charp,
                (*(*info_ptr).text.offset(i as isize)).text as png_const_charp,
            );
            /* Mark this chunk as written */
            if (*(*info_ptr).text.offset(i as isize)).compression == PNG_TEXT_COMPRESSION_NONE {
                (*(*info_ptr).text.offset(i as isize)).compression = PNG_TEXT_COMPRESSION_NONE_WR;
            } else {
                (*(*info_ptr).text.offset(i as isize)).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
            }
        }
        /* If we want a compressed text chunk */
        else if (*(*info_ptr).text.offset(i as isize)).compression == PNG_TEXT_COMPRESSION_zTXt {
            /* Write compressed chunk */
            png_write_zTXt(
                png_ptr,
                (*(*info_ptr).text.offset(i as isize)).key as png_const_charp,
                (*(*info_ptr).text.offset(i as isize)).text as png_const_charp,
                (*(*info_ptr).text.offset(i as isize)).compression,
            );
            /* Mark this chunk as written */
            (*(*info_ptr).text.offset(i as isize)).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
        } else if (*(*info_ptr).text.offset(i as isize)).compression == PNG_TEXT_COMPRESSION_NONE {
            /* Write uncompressed chunk */
            png_write_tEXt(
                png_ptr,
                (*(*info_ptr).text.offset(i as isize)).key as png_const_charp,
                (*(*info_ptr).text.offset(i as isize)).text as png_const_charp,
                0,
            );
            /* Mark this chunk as written */
            (*(*info_ptr).text.offset(i as isize)).compression = PNG_TEXT_COMPRESSION_NONE_WR;
        }
        i += 1;
    }
    /* tEXt */

    write_unknown_chunks(png_ptr, info_ptr, PNG_HAVE_PLTE);
}
