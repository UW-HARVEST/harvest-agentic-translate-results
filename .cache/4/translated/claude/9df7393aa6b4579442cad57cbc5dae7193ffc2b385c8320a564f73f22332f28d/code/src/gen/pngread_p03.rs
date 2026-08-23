/* pngread.c lines 674..1103 */

/* Read the end of the PNG file.  Will not read past the end of the
 * file, will verify the end is accurate, and will read any comments
 * or time information at the end of the file, if info is not NULL.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_end(png_ptr: png_structrp, info_ptr: png_inforp) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    /* If png_read_end is called in the middle of reading the rows there may
     * still be pending IDAT data and an owned zstream.  Deal with this here.
     */
    if png_chunk_unknown_handling(png_ptr, png_IDAT) == 0 {
        png_read_finish_IDAT(png_ptr);
    }

    /* Report invalid palette index; added at libpng-1.5.10 */
    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
        && (*png_ptr).num_palette_max >= (*png_ptr).num_palette as c_int
    {
        png_benign_error(
            png_ptr,
            b"Read palette index exceeding num_palette\0".as_ptr() as png_const_charp,
        );
    }

    loop {
        let length: png_uint_32 = png_read_chunk_header(png_ptr);
        let chunk_name: png_uint_32 = (*png_ptr).chunk_name;

        if chunk_name != png_IDAT {
            /* These flags must be set consistently for all non-IDAT chunks,
             * including the unknown chunks.
             */
            (*png_ptr).mode |= PNG_HAVE_CHUNK_AFTER_IDAT | PNG_AFTER_IDAT;
        }

        if chunk_name == png_IEND {
            png_handle_chunk(png_ptr, info_ptr, length);
        } else if chunk_name == png_IHDR {
            png_handle_chunk(png_ptr, info_ptr, length);
        } else if info_ptr == core::ptr::null_mut() {
            png_crc_finish(png_ptr, length);
        } else {
            let keep: c_int = png_chunk_unknown_handling(png_ptr, chunk_name);

            if keep != 0 {
                if chunk_name == png_IDAT {
                    if (length > 0 && ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0)
                        || ((*png_ptr).mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0
                    {
                        png_benign_error(
                            png_ptr,
                            b".Too many IDATs found\0".as_ptr() as png_const_charp,
                        );
                    }
                }
                png_handle_unknown(png_ptr, info_ptr, length, keep);
                if chunk_name == png_PLTE {
                    (*png_ptr).mode |= PNG_HAVE_PLTE;
                }
            } else if chunk_name == png_IDAT {
                /* Zero length IDATs are legal after the last IDAT has been
                 * read, but not after other chunks have been read.  1.6 does not
                 * always read all the deflate data; specifically it cannot be relied
                 * upon to read the Adler32 at the end.  If it doesn't ignore IDAT
                 * chunks which are longer than zero as well:
                 */
                if (length > 0 && ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0)
                    || ((*png_ptr).mode & PNG_HAVE_CHUNK_AFTER_IDAT) != 0
                {
                    png_benign_error(
                        png_ptr,
                        b"..Too many IDATs found\0".as_ptr() as png_const_charp,
                    );
                }

                png_crc_finish(png_ptr, length);
            } else {
                png_handle_chunk(png_ptr, info_ptr, length);
            }
        }

        if ((*png_ptr).mode & PNG_HAVE_IEND) != 0 {
            break;
        }
    }
}

/* Free all memory used in the read struct */
unsafe fn png_read_destroy(png_ptr: png_structrp) {
    png_destroy_gamma_table(png_ptr);

    png_free(png_ptr, (*png_ptr).big_row_buf as png_voidp);
    (*png_ptr).big_row_buf = core::ptr::null_mut();
    png_free(png_ptr, (*png_ptr).big_prev_row as png_voidp);
    (*png_ptr).big_prev_row = core::ptr::null_mut();
    png_free(png_ptr, (*png_ptr).read_buffer as png_voidp);
    (*png_ptr).read_buffer = core::ptr::null_mut();

    png_free(png_ptr, (*png_ptr).palette_lookup as png_voidp);
    (*png_ptr).palette_lookup = core::ptr::null_mut();
    png_free(png_ptr, (*png_ptr).quantize_index as png_voidp);
    (*png_ptr).quantize_index = core::ptr::null_mut();

    /* png_ptr->palette is always independently allocated (not aliased
     * with info_ptr->palette), so free it unconditionally.
     */
    png_free(png_ptr, (*png_ptr).palette as png_voidp);
    (*png_ptr).palette = core::ptr::null_mut();

    /* png_ptr->trans_alpha is always independently allocated (not aliased
     * with info_ptr->trans_alpha), so free it unconditionally.
     */
    png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
    (*png_ptr).trans_alpha = core::ptr::null_mut();

    inflateEnd(core::ptr::addr_of_mut!((*png_ptr).zstream));

    png_free(png_ptr, (*png_ptr).save_buffer as png_voidp);
    (*png_ptr).save_buffer = core::ptr::null_mut();

    png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
    (*png_ptr).unknown_chunk.data = core::ptr::null_mut();

    png_free(png_ptr, (*png_ptr).chunk_list as png_voidp);
    (*png_ptr).chunk_list = core::ptr::null_mut();

    /* PNG_READ_EXPAND_SUPPORTED && defined(PNG_ARM_NEON_IMPLEMENTATION) */
    png_free(png_ptr, (*png_ptr).riffled_palette as png_voidp);
    (*png_ptr).riffled_palette = core::ptr::null_mut();

    /* NOTE: the 'setjmp' buffer may still be allocated and the memory and error
     * callbacks are still set at this point.  They are required to complete the
     * destruction of the png_struct itself.
     */
}

/* Free all memory used by the read */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_read_struct(
    png_ptr_ptr: png_structpp,
    info_ptr_ptr: png_infopp,
    end_info_ptr_ptr: png_infopp,
) {
    let mut png_ptr: png_structrp = core::ptr::null_mut();

    if png_ptr_ptr != core::ptr::null_mut() {
        png_ptr = *png_ptr_ptr;
    }

    if png_ptr == core::ptr::null_mut() {
        return;
    }

    /* libpng 1.6.0: use the API to destroy info structs to ensure consistent
     * behavior.  Prior to 1.6.0 libpng did extra 'info' destruction in this API.
     * The extra was, apparently, unnecessary yet this hides memory leak bugs.
     */
    png_destroy_info_struct(png_ptr, end_info_ptr_ptr);
    png_destroy_info_struct(png_ptr, info_ptr_ptr);

    *png_ptr_ptr = core::ptr::null_mut();
    png_read_destroy(png_ptr);
    png_destroy_png_struct(png_ptr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_read_status_fn(
    png_ptr: png_structrp,
    read_row_fn: png_read_status_ptr,
) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    (*png_ptr).read_row_fn = read_row_fn;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_png(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    transforms: c_int,
    _params: png_voidp,
) {
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    /* png_read_info() gives us all of the information from the
     * PNG file before the first IDAT (image data chunk).
     */
    png_read_info(png_ptr, info_ptr);
    if (*info_ptr).height as usize
        > PNG_UINT_32_MAX as usize / core::mem::size_of::<png_bytep>()
    {
        png_error(
            png_ptr,
            b"Image is too high to process with png_read_png()\0".as_ptr() as png_const_charp,
        );
    }

    /* -------------- image transformations start here ------------------- */
    /* libpng 1.6.10: add code to cause a png_app_error if a selected TRANSFORM
     * is not implemented.  This will only happen in de-configured (non-default)
     * libpng builds.  The results can be unexpected - png_read_png may return
     * short or mal-formed rows because the transform is skipped.
     */

    /* Tell libpng to strip 16-bit/color files down to 8 bits per color.
     */
    if (transforms & PNG_TRANSFORM_SCALE_16) != 0 {
        /* Added at libpng-1.5.4. "strip_16" produces the same result that it
         * did in earlier versions, while "scale_16" is now more accurate.
         */
        png_set_scale_16(png_ptr);
    }

    /* If both SCALE and STRIP are required pngrtran will effectively cancel the
     * latter by doing SCALE first.  This is ok and allows apps not to check for
     * which is supported to get the right answer.
     */
    if (transforms & PNG_TRANSFORM_STRIP_16) != 0 {
        png_set_strip_16(png_ptr);
    }

    /* Strip alpha bytes from the input data without combining with
     * the background (not recommended).
     */
    if (transforms & PNG_TRANSFORM_STRIP_ALPHA) != 0 {
        png_set_strip_alpha(png_ptr);
    }

    /* Extract multiple pixels with bit depths of 1, 2, or 4 from a single
     * byte into separate bytes (useful for paletted and grayscale images).
     */
    if (transforms & PNG_TRANSFORM_PACKING) != 0 {
        png_set_packing(png_ptr);
    }

    /* Change the order of packed pixels to least significant bit first
     * (not useful if you are using png_set_packing).
     */
    if (transforms & PNG_TRANSFORM_PACKSWAP) != 0 {
        png_set_packswap(png_ptr);
    }

    /* Expand paletted colors into true RGB triplets
     * Expand grayscale images to full 8 bits from 1, 2, or 4 bits/pixel
     * Expand paletted or RGB images with transparency to full alpha
     * channels so the data will be available as RGBA quartets.
     */
    if (transforms & PNG_TRANSFORM_EXPAND) != 0 {
        png_set_expand(png_ptr);
    }

    /* We don't handle background color or gamma transformation or quantizing.
     */

    /* Invert monochrome files to have 0 as white and 1 as black
     */
    if (transforms & PNG_TRANSFORM_INVERT_MONO) != 0 {
        png_set_invert_mono(png_ptr);
    }

    /* If you want to shift the pixel values from the range [0,255] or
     * [0,65535] to the original [0,7] or [0,31], or whatever range the
     * colors were originally in:
     */
    if (transforms & PNG_TRANSFORM_SHIFT) != 0 {
        if ((*info_ptr).valid & PNG_INFO_sBIT) != 0 {
            png_set_shift(png_ptr, core::ptr::addr_of!((*info_ptr).sig_bit));
        }
    }

    /* Flip the RGB pixels to BGR (or RGBA to BGRA) */
    if (transforms & PNG_TRANSFORM_BGR) != 0 {
        png_set_bgr(png_ptr);
    }

    /* Swap the RGBA or GA data to ARGB or AG (or BGRA to ABGR) */
    if (transforms & PNG_TRANSFORM_SWAP_ALPHA) != 0 {
        png_set_swap_alpha(png_ptr);
    }

    /* Swap bytes of 16-bit files to least significant byte first */
    if (transforms & PNG_TRANSFORM_SWAP_ENDIAN) != 0 {
        png_set_swap(png_ptr);
    }

    /* Added at libpng-1.2.41 */
    /* Invert the alpha channel from opacity to transparency */
    if (transforms & PNG_TRANSFORM_INVERT_ALPHA) != 0 {
        png_set_invert_alpha(png_ptr);
    }

    /* Added at libpng-1.2.41 */
    /* Expand grayscale image to RGB */
    if (transforms & PNG_TRANSFORM_GRAY_TO_RGB) != 0 {
        png_set_gray_to_rgb(png_ptr);
    }

    /* Added at libpng-1.5.4 */
    if (transforms & PNG_TRANSFORM_EXPAND_16) != 0 {
        png_set_expand_16(png_ptr);
    }

    /* We don't handle adding filler bytes */

    /* We use png_read_image and rely on that for interlace handling, but we also
     * call png_read_update_info therefore must turn on interlace handling now:
     */
    png_set_interlace_handling(png_ptr);

    /* Optional call to gamma correct and add the background to the palette
     * and update info structure.  REQUIRED if you are expecting libpng to
     * update the palette for you (i.e., you selected such a transform above).
     */
    png_read_update_info(png_ptr, info_ptr);

    /* -------------- image transformations end here ------------------- */

    png_free_data(png_ptr, info_ptr, PNG_FREE_ROWS, 0);
    if (*info_ptr).row_pointers == core::ptr::null_mut() {
        let mut iptr: png_uint_32;

        (*info_ptr).row_pointers = png_malloc(
            png_ptr,
            ((*info_ptr).height as usize * core::mem::size_of::<png_bytep>()) as png_alloc_size_t,
        ) as png_bytepp;

        iptr = 0;
        while iptr < (*info_ptr).height {
            *(*info_ptr).row_pointers.add(iptr as usize) = core::ptr::null_mut();
            iptr = iptr.wrapping_add(1);
        }

        (*info_ptr).free_me |= PNG_FREE_ROWS;

        iptr = 0;
        while iptr < (*info_ptr).height {
            *(*info_ptr).row_pointers.add(iptr as usize) =
                png_malloc(png_ptr, (*info_ptr).rowbytes as png_alloc_size_t) as png_bytep;
            iptr = iptr.wrapping_add(1);
        }
    }

    png_read_image(png_ptr, (*info_ptr).row_pointers);
    (*info_ptr).valid |= PNG_INFO_IDAT;

    /* Read rest of file, and get additional chunks in info_ptr - REQUIRED */
    png_read_end(png_ptr, info_ptr);
}
