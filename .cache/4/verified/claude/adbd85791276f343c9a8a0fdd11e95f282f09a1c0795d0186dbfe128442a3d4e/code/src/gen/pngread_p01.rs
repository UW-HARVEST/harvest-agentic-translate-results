/* pngread.c lines 1..287 */

/* Create a PNG structure for reading, and allocate any memory needed. */
/* png_create_read_struct */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_read_struct(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
) -> png_structp {
    png_create_read_struct_2(
        user_png_ver,
        error_ptr,
        error_fn,
        warn_fn,
        core::ptr::null_mut(),
        None,
        None,
    )
}

/* Alternate create PNG structure for reading, and allocate any memory
 * needed.
 */
/* png_create_read_struct_2 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_read_struct_2(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
    mem_ptr: png_voidp,
    malloc_fn: png_malloc_ptr,
    free_fn: png_free_ptr,
) -> png_structp {
    let png_ptr: png_structp = png_create_png_struct(
        user_png_ver,
        error_ptr,
        error_fn,
        warn_fn,
        mem_ptr,
        malloc_fn,
        free_fn,
    );

    if png_ptr != core::ptr::null_mut() {
        (*png_ptr).mode = PNG_IS_READ_STRUCT;

        /* Added in libpng-1.6.0; this can be used to detect a read structure if
         * required (it will be zero in a write structure.)
         */
        (*png_ptr).IDAT_read_size = PNG_IDAT_READ_SIZE as uInt;

        (*png_ptr).flags |= PNG_FLAG_BENIGN_ERRORS_WARN;

        /* In stable builds only warn if an application error can be completely
         * handled.
         */
        /* PNG_RELEASE_BUILD is false, so PNG_FLAG_APP_WARNINGS_WARN is not set */

        /* TODO: delay this, it can be done in png_init_io (if the app doesn't
         * do it itself) avoiding setting the default function if it is not
         * required.
         */
        png_set_read_fn(png_ptr, core::ptr::null_mut(), None);
    }

    png_ptr
}

/* Read the information before the actual image data.  This has been
 * changed in v0.90 to allow reading a file that already has the magic
 * bytes read from the stream.  You can tell libpng how many bytes have
 * been read from the beginning of the stream (up to the maximum of 8)
 * via png_set_sig_bytes(), and we will only check the remaining bytes
 * here.  The application can then have access to the signature bytes we
 * read if it is determined that this isn't a valid PNG file.
 */
/* png_read_info */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_info(png_ptr: png_structrp, info_ptr: png_inforp) {
    let mut keep: c_int;

    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    /* Read and check the PNG file signature. */
    png_read_sig(png_ptr, info_ptr);

    loop {
        let length: png_uint_32 = png_read_chunk_header(png_ptr);
        let chunk_name: png_uint_32 = (*png_ptr).chunk_name;

        /* IDAT logic needs to happen here to simplify getting the two flags
         * right.
         */
        if chunk_name == png_IDAT {
            if ((*png_ptr).mode & PNG_HAVE_IHDR) == 0 {
                png_chunk_error(
                    png_ptr,
                    b"Missing IHDR before IDAT\0".as_ptr() as png_const_charp,
                );
            } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
                && ((*png_ptr).mode & PNG_HAVE_PLTE) == 0
            {
                png_chunk_error(
                    png_ptr,
                    b"Missing PLTE before IDAT\0".as_ptr() as png_const_charp,
                );
            } else if ((*png_ptr).mode & PNG_AFTER_IDAT) != 0 {
                png_chunk_benign_error(
                    png_ptr,
                    b"Too many IDATs found\0".as_ptr() as png_const_charp,
                );
            }

            (*png_ptr).mode |= PNG_HAVE_IDAT;
        } else if ((*png_ptr).mode & PNG_HAVE_IDAT) != 0 {
            (*png_ptr).mode |= PNG_HAVE_CHUNK_AFTER_IDAT;
            (*png_ptr).mode |= PNG_AFTER_IDAT;
        }

        if chunk_name == png_IHDR {
            png_handle_chunk(png_ptr, info_ptr, length);
        } else if chunk_name == png_IEND {
            png_handle_chunk(png_ptr, info_ptr, length);
        } else {
            keep = png_chunk_unknown_handling(png_ptr, chunk_name);

            if keep != 0 {
                png_handle_unknown(png_ptr, info_ptr, length, keep);

                if chunk_name == png_PLTE {
                    (*png_ptr).mode |= PNG_HAVE_PLTE;
                } else if chunk_name == png_IDAT {
                    (*png_ptr).idat_size = 0; /* It has been consumed */
                    break;
                }
            } else if chunk_name == png_IDAT {
                (*png_ptr).idat_size = length;
                break;
            } else {
                png_handle_chunk(png_ptr, info_ptr, length);
            }
        }
    }
}

/* Optional call to update the users info_ptr structure */
/* png_read_update_info */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_update_info(png_ptr: png_structrp, info_ptr: png_inforp) {
    if png_ptr != core::ptr::null_mut() {
        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
            png_read_start_row(png_ptr);

            png_read_transform_info(png_ptr, info_ptr);
        }
        /* New in 1.6.0 this avoids the bug of doing the initializations twice */
        else {
            png_app_error(
                png_ptr,
                b"png_read_update_info/png_start_read_image: duplicate call\0".as_ptr()
                    as png_const_charp,
            );
        }
    }
}

/* Initialize palette, background, etc, after transformations
 * are set, but before any reading takes place.  This allows
 * the user to obtain a gamma-corrected palette, for example.
 * If the user doesn't call this, we will do it ourselves.
 */
/* png_start_read_image */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_start_read_image(png_ptr: png_structrp) {
    if png_ptr != core::ptr::null_mut() {
        if ((*png_ptr).flags & PNG_FLAG_ROW_INIT) == 0 {
            png_read_start_row(png_ptr);
        }
        /* New in 1.6.0 this avoids the bug of doing the initializations twice */
        else {
            png_app_error(
                png_ptr,
                b"png_start_read_image/png_read_update_info: duplicate call\0".as_ptr()
                    as png_const_charp,
            );
        }
    }
}

/* Undoes intrapixel differencing,
 * NOTE: this is apparently only supported in the 'sequential' reader.
 */
/* png_do_read_intrapixel (static) */
unsafe fn png_do_read_intrapixel(row_info: png_row_infop, row: png_bytep) {
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
                *rp = ((256 + *rp as c_int + *rp.add(1) as c_int) & 0xff) as png_byte;
                *rp.add(2) =
                    ((256 + *rp.add(2) as c_int + *rp.add(1) as c_int) & 0xff) as png_byte;

                i += 1;
                rp = rp.add(bytes_per_pixel as usize);
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
                let red: png_uint_32 = s0.wrapping_add(s1).wrapping_add(65536) & 0xffff;
                let blue: png_uint_32 = s2.wrapping_add(s1).wrapping_add(65536) & 0xffff;
                *rp = ((red >> 8) & 0xff) as png_byte;
                *rp.add(1) = (red & 0xff) as png_byte;
                *rp.add(4) = ((blue >> 8) & 0xff) as png_byte;
                *rp.add(5) = (blue & 0xff) as png_byte;

                i += 1;
                rp = rp.add(bytes_per_pixel as usize);
            }
        }
    }
}
