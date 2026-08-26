/* pngwrite.c lines 392..745 */

/* Writes the end of the PNG file.  If you don't want to write comments or
 * time information, you can pass NULL for info.  If you already wrote these
 * in png_write_info(), do not write them again here.  If you have long
 * comments, I suggest writing them here, and compressing them.
 */
/* png_write_end */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_end(png_ptr: png_structrp, info_ptr: png_inforp) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    if ((*png_ptr).mode & PNG_HAVE_IDAT) == 0 {
        png_error(
            png_ptr,
            b"No IDATs written into file\0".as_ptr() as png_const_charp,
        );
    }

    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
        && (*png_ptr).num_palette_max >= (*png_ptr).num_palette as c_int
    {
        png_benign_error(
            png_ptr,
            b"Wrote palette index exceeding num_palette\0".as_ptr() as png_const_charp,
        );
    }

    /* See if user wants us to write information chunks */
    if info_ptr != core::ptr::null_mut() {
        let mut i: c_int; /* local index variable */

        /* Check to see if user has supplied a time chunk */
        if ((*info_ptr).valid & PNG_INFO_tIME) != 0 && ((*png_ptr).mode & PNG_WROTE_tIME) == 0 {
            png_write_tIME(png_ptr, core::ptr::addr_of!((*info_ptr).mod_time));
        }

        /* Loop through comment chunks */
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
                    (*(*info_ptr).text.offset(i as isize)).compression =
                        PNG_TEXT_COMPRESSION_NONE_WR;
                } else {
                    (*(*info_ptr).text.offset(i as isize)).compression =
                        PNG_TEXT_COMPRESSION_zTXt_WR;
                }
            } else if (*(*info_ptr).text.offset(i as isize)).compression
                >= PNG_TEXT_COMPRESSION_zTXt
            {
                /* Write compressed chunk */
                png_write_zTXt(
                    png_ptr,
                    (*(*info_ptr).text.offset(i as isize)).key as png_const_charp,
                    (*(*info_ptr).text.offset(i as isize)).text as png_const_charp,
                    (*(*info_ptr).text.offset(i as isize)).compression,
                );
                /* Mark this chunk as written */
                (*(*info_ptr).text.offset(i as isize)).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
            } else if (*(*info_ptr).text.offset(i as isize)).compression
                == PNG_TEXT_COMPRESSION_NONE
            {
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
    /* PNG_WRITE_FLUSH_AFTER_IEND_SUPPORTED is not defined */
}

/* png_convert_from_struct_tm */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_convert_from_struct_tm(ptime: png_timep, ttime: *const tm) {
    (*ptime).year = (1900 + (*ttime).tm_year) as png_uint_16;
    (*ptime).month = ((*ttime).tm_mon + 1) as png_byte;
    (*ptime).day = (*ttime).tm_mday as png_byte;
    (*ptime).hour = (*ttime).tm_hour as png_byte;
    (*ptime).minute = (*ttime).tm_min as png_byte;
    (*ptime).second = (*ttime).tm_sec as png_byte;
}

/* png_convert_from_time_t */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_convert_from_time_t(ptime: png_timep, ttime: time_t) {
    let tbuf: *mut tm;
    let t: time_t = ttime;

    tbuf = gmtime(&t);
    if tbuf == core::ptr::null_mut() {
        /* TODO: add a safe function which takes a png_ptr argument and raises
         * a png_error if the ttime argument is invalid and the call to gmtime
         * fails as a consequence.
         */
        memset(
            ptime as *mut c_void,
            0,
            core::mem::size_of::<png_time>(),
        );
        return;
    }

    png_convert_from_struct_tm(ptime, tbuf);
}

/* Initialize png_ptr structure, and allocate any memory needed */
/* png_create_write_struct */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_write_struct(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
) -> png_structp {
    return png_create_write_struct_2(
        user_png_ver,
        error_ptr,
        error_fn,
        warn_fn,
        core::ptr::null_mut(),
        None,
        None,
    );
}

/* Alternate initialize png_ptr structure, and allocate any memory needed */
/* png_create_write_struct_2 */
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

    if png_ptr != core::ptr::null_mut() {
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
         *
         * PNG_BENIGN_WRITE_ERRORS_SUPPORTED is not defined.
         */

        /* App warnings are warnings in release (or release candidate) builds but
         * are errors during development.
         *
         * PNG_RELEASE_BUILD is false.
         */

        /* TODO: delay this, it can be done in png_init_io() (if the app doesn't
         * do it itself) avoiding setting the default function if it is not
         * required.
         */
        png_set_write_fn(png_ptr, core::ptr::null_mut(), None, None);
    }

    return png_ptr;
}

/* Write a few rows of image data.  If the image is interlaced,
 * either you will have to write the 7 sub images, or, if you
 * have called png_set_interlace_handling(), you will have to
 * "write" the image seven times.
 */
/* png_write_rows */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_rows(
    png_ptr: png_structrp,
    row: png_bytepp,
    num_rows: png_uint_32,
) {
    let mut i: png_uint_32; /* row counter */
    let mut rp: png_bytepp; /* row pointer */

    if png_ptr == core::ptr::null_mut() {
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

/* Write the image.  You only need to call this function once, even
 * if you are writing an interlaced image.
 */
/* png_write_image */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_image(png_ptr: png_structrp, image: png_bytepp) {
    let mut i: png_uint_32; /* row index */
    let mut pass: c_int;
    let num_pass: c_int; /* pass variables */
    let mut rp: png_bytepp; /* points to current row */

    if png_ptr == core::ptr::null_mut() {
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

/* Performs intrapixel differencing  */
/* png_do_write_intrapixel */
unsafe fn png_do_write_intrapixel(row_info: png_row_infop, row: png_bytep) {
    if ((*row_info).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        let bytes_per_pixel: c_int;
        let row_width: png_uint_32 = (*row_info).width;
        if (*row_info).bit_depth as c_int == 8 {
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
                *rp = ((*rp as c_int) - (*rp.add(1) as c_int)) as png_byte;
                *rp.add(2) = ((*rp.add(2) as c_int) - (*rp.add(1) as c_int)) as png_byte;
                i += 1;
                rp = rp.offset(bytes_per_pixel as isize);
            }
        } else if (*row_info).bit_depth as c_int == 16 {
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
                    (((*rp as c_int) << 8) as png_uint_32) | (*rp.add(1) as png_uint_32);
                let s1: png_uint_32 =
                    (((*rp.add(2) as c_int) << 8) as png_uint_32) | (*rp.add(3) as png_uint_32);
                let s2: png_uint_32 =
                    (((*rp.add(4) as c_int) << 8) as png_uint_32) | (*rp.add(5) as png_uint_32);
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
