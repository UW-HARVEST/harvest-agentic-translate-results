/* pngread.c lines 1104..1527 */

/* Do all the *safe* initialization - 'safe' means that png_error won't be
 * called, so setting up the jmp_buf is not required.  This means that anything
 * called from here must *not* call png_malloc - it has to call png_malloc_warn
 * instead so that control is returned safely back to this routine.
 */
unsafe fn png_image_read_init(image: png_imagep) -> c_int {
    if (*image).opaque == core::ptr::null_mut() {
        let mut png_ptr: png_structp = png_create_read_struct(
            PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
            image as png_voidp,
            Some(png_safe_error),
            Some(png_safe_warning),
        );

        /* And set the rest of the structure to NULL to ensure that the various
         * fields are consistent.
         */
        memset(image as *mut c_void, 0, core::mem::size_of::<png_image>());
        (*image).version = PNG_IMAGE_VERSION;

        if png_ptr != core::ptr::null_mut() {
            let mut info_ptr: png_infop = png_create_info_struct(png_ptr);

            if info_ptr != core::ptr::null_mut() {
                let control: png_controlp = png_malloc_warn(
                    png_ptr,
                    core::mem::size_of::<png_control>() as png_alloc_size_t,
                ) as png_controlp;

                if control != core::ptr::null_mut() {
                    memset(
                        control as *mut c_void,
                        0,
                        core::mem::size_of::<png_control>(),
                    );

                    (*control).png_ptr = png_ptr;
                    (*control).info_ptr = info_ptr;
                    (*control).set_for_write(0);

                    (*image).opaque = control;
                    return 1;
                }

                /* Error clean up */
                png_destroy_info_struct(png_ptr, &mut info_ptr);
            }

            png_destroy_read_struct(
                &mut png_ptr,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            );
        }

        return png_image_error(
            image,
            b"png_image_read: out of memory\0".as_ptr() as png_const_charp,
        );
    }

    png_image_error(
        image,
        b"png_image_read: opaque pointer not NULL\0".as_ptr() as png_const_charp,
    )
}

/* Utility to find the base format of a PNG file from a png_struct. */
unsafe fn png_image_format(png_ptr: png_structrp) -> png_uint_32 {
    let mut format: png_uint_32 = 0;

    if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        format |= PNG_FORMAT_FLAG_COLOR;
    }

    if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0 {
        format |= PNG_FORMAT_FLAG_ALPHA;
    }
    /* Use png_ptr here, not info_ptr, because by examination png_handle_tRNS
     * sets the png_struct fields; that's all we are interested in here.  The
     * precise interaction with an app call to png_set_tRNS and PNG file reading
     * is unclear.
     */
    else if (*png_ptr).num_trans > 0 {
        format |= PNG_FORMAT_FLAG_ALPHA;
    }

    if (*png_ptr).bit_depth as c_int == 16 {
        format |= PNG_FORMAT_FLAG_LINEAR;
    }

    if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_PALETTE) != 0 {
        format |= PNG_FORMAT_FLAG_COLORMAP;
    }

    format
}

static sRGB_xy: png_xy = png_xy {
    /* From ITU-R BT.709-3 */
    /* color      x       y */
    /* red   */
    redx: 64000,
    redy: 33000,
    /* green */
    greenx: 30000,
    greeny: 60000,
    /* blue  */
    bluex: 15000,
    bluey: 6000,
    /* white */
    whitex: 31270,
    whitey: 32900,
};

unsafe fn chromaticities_match_sRGB(xy: *const png_xy) -> c_int {
    if PNG_OUT_OF_RANGE((*xy).whitex, sRGB_xy.whitex, sRGB_TOLERANCE)
        || PNG_OUT_OF_RANGE((*xy).whitey, sRGB_xy.whitey, sRGB_TOLERANCE)
        || PNG_OUT_OF_RANGE((*xy).redx, sRGB_xy.redx, sRGB_TOLERANCE)
        || PNG_OUT_OF_RANGE((*xy).redy, sRGB_xy.redy, sRGB_TOLERANCE)
        || PNG_OUT_OF_RANGE((*xy).greenx, sRGB_xy.greenx, sRGB_TOLERANCE)
        || PNG_OUT_OF_RANGE((*xy).greeny, sRGB_xy.greeny, sRGB_TOLERANCE)
        || PNG_OUT_OF_RANGE((*xy).bluex, sRGB_xy.bluex, sRGB_TOLERANCE)
        || PNG_OUT_OF_RANGE((*xy).bluey, sRGB_xy.bluey, sRGB_TOLERANCE)
    {
        return 0;
    }
    1
}

/* Is the given gamma significantly different from sRGB?  The test is the same
 * one used in pngrtran.c when deciding whether to do gamma correction.  The
 * arithmetic optimizes the division by using the fact that the inverse of the
 * file sRGB gamma is 2.2
 */
unsafe fn png_gamma_not_sRGB(g: png_fixed_point) -> c_int {
    /* 1.6.47: use the same sanity checks as used in pngrtran.c */
    if g < PNG_LIB_GAMMA_MIN || g > PNG_LIB_GAMMA_MAX {
        return 0; /* Includes the uninitialized value 0 */
    }

    png_gamma_significant((g * 11 + 2) / 5 /* i.e. *2.2, rounded */)
}

/* Do the main body of a 'png_image_begin_read' function; read the PNG file
 * header and fill in all the information.  This is executed in a safe context,
 * unlike the init routine above.
 */
unsafe fn png_image_is_not_sRGB(png_ptr: png_const_structrp) -> c_int {
    /* Does the colorspace **not** match sRGB?  The flag is only set if the
     * answer can be determined reliably.
     *
     * png_struct::chromaticities always exists since the simplified API
     * requires rgb-to-gray.  The mDCV, cICP and cHRM chunks may all set it to
     * a non-sRGB value, so it needs to be checked but **only** if one of
     * those chunks occurred in the file.
     */
    /* Highest priority: check to be safe. */
    if png_file_has_chunk(png_ptr, PNG_INDEX_cICP)
        || png_file_has_chunk(png_ptr, PNG_INDEX_mDCV)
    {
        return (chromaticities_match_sRGB(core::ptr::addr_of!((*png_ptr).chromaticities)) == 0)
            as c_int;
    }

    /* If the image is marked as sRGB then it is... */
    if png_file_has_chunk(png_ptr, PNG_INDEX_sRGB) {
        return 0;
    }

    /* Last stop: cHRM, must check: */
    if png_file_has_chunk(png_ptr, PNG_INDEX_cHRM) {
        return (chromaticities_match_sRGB(core::ptr::addr_of!((*png_ptr).chromaticities)) == 0)
            as c_int;
    }

    /* Else default to sRGB */
    0
}

unsafe extern "C" fn png_image_read_header(argument: png_voidp) -> c_int {
    let image: png_imagep = argument as png_imagep;
    let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
    let info_ptr: png_inforp = (*(*image).opaque).info_ptr;

    png_set_benign_errors(png_ptr, 1 /*warn*/);
    png_read_info(png_ptr, info_ptr);

    /* Do this the fast way; just read directly out of png_struct. */
    (*image).width = (*png_ptr).width;
    (*image).height = (*png_ptr).height;

    {
        let format: png_uint_32 = png_image_format(png_ptr);

        (*image).format = format;

        /* Greyscale images don't (typically) have colour space information and
         * using it is pretty much impossible, so use sRGB for grayscale (it
         * doesn't matter r==g==b so the transform is irrelevant.)
         */
        if (format & PNG_FORMAT_FLAG_COLOR) != 0 && png_image_is_not_sRGB(png_ptr) != 0 {
            (*image).flags |= PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB;
        }
    }

    /* We need the maximum number of entries regardless of the format the
     * application sets here.
     */
    {
        let mut cmap_entries: png_uint_32;

        match (*png_ptr).color_type as c_int {
            PNG_COLOR_TYPE_GRAY => {
                cmap_entries = 1u32 << (*png_ptr).bit_depth as c_int;
            }

            PNG_COLOR_TYPE_PALETTE => {
                cmap_entries = (*png_ptr).num_palette as png_uint_32;
            }

            _ => {
                cmap_entries = 256;
            }
        }

        if cmap_entries > 256 {
            cmap_entries = 256;
        }

        (*image).colormap_entries = cmap_entries;
    }

    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_begin_read_from_stdio(
    image: png_imagep,
    file: *mut FILE,
) -> c_int {
    if image != core::ptr::null_mut() && (*image).version == PNG_IMAGE_VERSION {
        if file != core::ptr::null_mut() {
            if png_image_read_init(image) != 0 {
                /* This is slightly evil, but png_init_io doesn't do anything other
                 * than this and we haven't changed the standard IO functions so
                 * this saves a 'safe' function.
                 */
                (*(*(*image).opaque).png_ptr).io_ptr = file as png_voidp;
                return png_safe_execute(
                    image,
                    Some(png_image_read_header),
                    image as png_voidp,
                );
            }
        } else {
            return png_image_error(
                image,
                b"png_image_begin_read_from_stdio: invalid argument\0".as_ptr()
                    as png_const_charp,
            );
        }
    } else if image != core::ptr::null_mut() {
        return png_image_error(
            image,
            b"png_image_begin_read_from_stdio: incorrect PNG_IMAGE_VERSION\0".as_ptr()
                as png_const_charp,
        );
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_begin_read_from_file(
    image: png_imagep,
    file_name: *const c_char,
) -> c_int {
    if image != core::ptr::null_mut() && (*image).version == PNG_IMAGE_VERSION {
        if file_name != core::ptr::null() {
            let fp: *mut FILE = fopen(file_name, b"rb\0".as_ptr() as *const c_char);

            if fp != core::ptr::null_mut() {
                if png_image_read_init(image) != 0 {
                    (*(*(*image).opaque).png_ptr).io_ptr = fp as png_voidp;
                    (*(*image).opaque).set_owned_file(1);
                    return png_safe_execute(
                        image,
                        Some(png_image_read_header),
                        image as png_voidp,
                    );
                }

                /* Clean up: just the opened file. */
                fclose(fp);
            } else {
                return png_image_error(
                    image,
                    strerror(*__errno_location()) as png_const_charp,
                );
            }
        } else {
            return png_image_error(
                image,
                b"png_image_begin_read_from_file: invalid argument\0".as_ptr()
                    as png_const_charp,
            );
        }
    } else if image != core::ptr::null_mut() {
        return png_image_error(
            image,
            b"png_image_begin_read_from_file: incorrect PNG_IMAGE_VERSION\0".as_ptr()
                as png_const_charp,
        );
    }

    0
}

unsafe extern "C" fn png_image_memory_read(png_ptr: png_structp, out: png_bytep, need: usize) {
    if png_ptr != core::ptr::null_mut() {
        let image: png_imagep = (*png_ptr).io_ptr as png_imagep;
        if image != core::ptr::null_mut() {
            let cp: png_controlp = (*image).opaque;
            if cp != core::ptr::null_mut() {
                let memory: png_const_bytep = (*cp).memory;
                let size: usize = (*cp).size;

                if memory != core::ptr::null() && size >= need {
                    memcpy(out as *mut c_void, memory as *const c_void, need);
                    (*cp).memory = memory.add(need);
                    (*cp).size = size - need;
                    return;
                }

                png_error(
                    png_ptr,
                    b"read beyond end of data\0".as_ptr() as png_const_charp,
                );
            }
        }

        png_error(
            png_ptr,
            b"invalid memory read\0".as_ptr() as png_const_charp,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_begin_read_from_memory(
    image: png_imagep,
    memory: png_const_voidp,
    size: usize,
) -> c_int {
    if image != core::ptr::null_mut() && (*image).version == PNG_IMAGE_VERSION {
        if memory != core::ptr::null() && size > 0 {
            if png_image_read_init(image) != 0 {
                /* Now set the IO functions to read from the memory buffer and
                 * store it into io_ptr.  Again do this in-place to avoid calling a
                 * libpng function that requires error handling.
                 */
                (*(*image).opaque).memory = memory as png_const_bytep;
                (*(*image).opaque).size = size;
                (*(*(*image).opaque).png_ptr).io_ptr = image as png_voidp;
                (*(*(*image).opaque).png_ptr).read_data_fn = Some(png_image_memory_read);

                return png_safe_execute(
                    image,
                    Some(png_image_read_header),
                    image as png_voidp,
                );
            }
        } else {
            return png_image_error(
                image,
                b"png_image_begin_read_from_memory: invalid argument\0".as_ptr()
                    as png_const_charp,
            );
        }
    } else if image != core::ptr::null_mut() {
        return png_image_error(
            image,
            b"png_image_begin_read_from_memory: incorrect PNG_IMAGE_VERSION\0".as_ptr()
                as png_const_charp,
        );
    }

    0
}

/* Utility function to skip chunks that are not used by the simplified image
 * read functions and an appropriate macro to call it.
 */
static chunks_to_process: [png_byte; 35] = [
    98, 75, 71, 68, b'\0',   /* bKGD */
    99, 72, 82, 77, b'\0',   /* cHRM */
    99, 73, 67, 80, b'\0',   /* cICP */
    103, 65, 77, 65, b'\0',  /* gAMA */
    109, 68, 67, 86, b'\0',  /* mDCV */
    115, 66, 73, 84, b'\0',  /* sBIT */
    115, 82, 71, 66, b'\0',  /* sRGB */
];

unsafe fn png_image_skip_unused_chunks(png_ptr: png_structrp) {
    /* Prepare the reader to ignore all recognized chunks whose data will not
     * be used, i.e., all chunks recognized by libpng except for those
     * involved in basic image reading:
     *
     *    IHDR, PLTE, IDAT, IEND
     *
     * Or image data handling:
     *
     *    tRNS, bKGD, gAMA, cHRM, sRGB, [iCCP] and sBIT.
     *
     * This provides a small performance improvement and eliminates any
     * potential vulnerability to security problems in the unused chunks.
     *
     * At present the iCCP chunk data isn't used, so iCCP chunk can be ignored
     * too.  This allows the simplified API to be compiled without iCCP support.
     */
    {
        /* Ignore unknown chunks and all other chunks except for the
         * IHDR, PLTE, tRNS, IDAT, and IEND chunks.
         */
        png_set_keep_unknown_chunks(
            png_ptr,
            PNG_HANDLE_CHUNK_NEVER,
            core::ptr::null(),
            -1,
        );

        /* But do not ignore image data handling chunks */
        png_set_keep_unknown_chunks(
            png_ptr,
            PNG_HANDLE_CHUNK_AS_DEFAULT,
            chunks_to_process.as_ptr(),
            (core::mem::size_of::<[png_byte; 35]>() as c_int) / 5, /*SAFE*/
        );
    }
}
