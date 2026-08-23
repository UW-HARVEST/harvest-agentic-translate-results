/* pngwrite.c lines 1852..2462 */

/* png_image_set_PLTE */
unsafe fn png_image_set_PLTE(display: *mut png_image_write_control) {
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

    let afirst: c_int = ((format & PNG_FORMAT_FLAG_AFIRST) != 0 && (format & PNG_FORMAT_FLAG_ALPHA) != 0)
        as c_int;

    let bgr: c_int = if (format & PNG_FORMAT_FLAG_BGR) != 0 { 2 } else { 0 };

    let mut i: c_int;
    let mut num_trans: c_int;
    let mut palette: [png_color; 256] = [Default::default(); 256];
    let mut tRNS: [png_byte; 256] = [0; 256];

    memset(
        tRNS.as_mut_ptr() as *mut c_void,
        255,
        core::mem::size_of::<[png_byte; 256]>(),
    );
    memset(
        palette.as_mut_ptr() as *mut c_void,
        0,
        core::mem::size_of::<[png_color; 256]>(),
    );

    num_trans = 0;
    i = 0;
    while i < entries {
        /* This gets automatically converted to sRGB with reversal of the
         * pre-multiplication if the color-map has an alpha channel.
         */
        if (format & PNG_FORMAT_FLAG_LINEAR) != 0 {
            let mut entry: png_const_uint_16p = cmap as png_const_uint_16p;

            entry = entry.add(((i as c_uint).wrapping_mul(channels)) as usize);

            if (channels & 1) != 0
            /* no alpha */
            {
                if channels >= 3
                /* RGB */
                {
                    palette[i as usize].blue = PNG_sRGB_FROM_LINEAR(
                        255 * (*entry.offset((2 ^ bgr) as isize) as png_uint_32),
                    ) as png_byte;
                    palette[i as usize].green =
                        PNG_sRGB_FROM_LINEAR(255 * (*entry.add(1) as png_uint_32)) as png_byte;
                    palette[i as usize].red = PNG_sRGB_FROM_LINEAR(
                        255 * (*entry.offset(bgr as isize) as png_uint_32),
                    ) as png_byte;
                }
                /* Gray */
                else {
                    let v: png_byte =
                        PNG_sRGB_FROM_LINEAR(255 * (*entry as png_uint_32)) as png_byte;
                    palette[i as usize].green = v;
                    palette[i as usize].red = v;
                    palette[i as usize].blue = v;
                }
            }
            /* alpha */
            else {
                let alpha: png_uint_16 = *entry.offset(if afirst != 0 {
                    0
                } else {
                    channels.wrapping_sub(1) as isize
                });
                let alphabyte: png_byte = PNG_DIV257(alpha as png_uint_32) as png_byte;
                let mut reciprocal: png_uint_32 = 0;

                /* Calculate a reciprocal, as in the png_write_image_8bit code above
                 * this is designed to produce a value scaled to 255*65535 when
                 * divided by 128 (i.e. asr 7).
                 */
                if alphabyte > 0 && alphabyte < 255 {
                    reciprocal = ((((0xffff as png_uint_32) * 0xff) << 7)
                        + ((alpha as png_uint_32) >> 1))
                        / (alpha as png_uint_32);
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
                }
                /* gray */
                else {
                    let v: png_byte = png_unpremultiply(
                        *entry.offset(afirst as isize) as png_uint_32,
                        alpha as png_uint_32,
                        reciprocal,
                    );
                    palette[i as usize].green = v;
                    palette[i as usize].red = v;
                    palette[i as usize].blue = v;
                }
            }
        }
        /* Color-map has sRGB values */
        else {
            let mut entry: png_const_bytep = cmap as png_const_bytep;

            entry = entry.add(((i as c_uint).wrapping_mul(channels)) as usize);

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
                    let v: png_byte = *entry.offset(afirst as isize);
                    palette[i as usize].green = v;
                    palette[i as usize].red = v;
                    palette[i as usize].blue = v;
                }

                1 => {
                    let v: png_byte = *entry.offset(afirst as isize);
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
        palette.as_ptr() as png_const_colorp,
        entries,
    );

    if num_trans > 0 {
        png_set_tRNS(
            (*(*image).opaque).png_ptr,
            (*(*image).opaque).info_ptr,
            tRNS.as_ptr() as png_const_bytep,
            num_trans,
            core::ptr::null_mut(),
        );
    }

    (*image).colormap_entries = entries as png_uint_32;
}

/* png_image_write_main */
unsafe extern "C" fn png_image_write_main(argument: png_voidp) -> c_int {
    let display: *mut png_image_write_control = argument as *mut png_image_write_control;
    let image: png_imagep = (*display).image;
    let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
    let info_ptr: png_inforp = (*(*image).opaque).info_ptr;
    let mut format: png_uint_32 = (*image).format;

    /* The following four ints are actually booleans */
    let colormap: c_int = (format & PNG_FORMAT_FLAG_COLORMAP) as c_int;
    let linear: c_int = (colormap == 0 && (format & PNG_FORMAT_FLAG_LINEAR) != 0) as c_int; /* input */
    let alpha: c_int = (colormap == 0 && (format & PNG_FORMAT_FLAG_ALPHA) != 0) as c_int;
    let write_16bit: c_int = (linear != 0 && (*display).convert_to_8bit == 0) as c_int;

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
            let png_row_stride: png_uint_32 = (*image).width.wrapping_mul(channels);

            if (*display).row_stride == 0 {
                (*display).row_stride = /*SAFE*/ png_row_stride as png_int_32;
            }

            if (*display).row_stride < 0 {
                check = ((*display).row_stride as png_uint_32).wrapping_neg();
            } else {
                check = (*display).row_stride as png_uint_32;
            }

            if check >= png_row_stride {
                /* Now check for overflow of the image buffer calculation; this
                 * limits the whole image size to 32 bits for API compatibility with
                 * the current, 32-bit, PNG_IMAGE_BUFFER_SIZE macro.
                 */
                if (*image).height > 0xffffffffu32 / png_row_stride {
                    png_error(
                        (*(*image).opaque).png_ptr,
                        b"memory image too large\0".as_ptr() as png_const_charp,
                    );
                }
            } else {
                png_error(
                    (*(*image).opaque).png_ptr,
                    b"supplied row stride too small\0".as_ptr() as png_const_charp,
                );
            }
        } else {
            png_error(
                (*(*image).opaque).png_ptr,
                b"image row stride too large\0".as_ptr() as png_const_charp,
            );
        }
    }

    /* Set the required transforms then write the rows in the correct order. */
    if (format & PNG_FORMAT_FLAG_COLORMAP) != 0 {
        if (*display).colormap != core::ptr::null() && (*image).colormap_entries > 0 {
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
                b"no color-map for color-mapped image\0".as_ptr() as png_const_charp,
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
                /* white */ 31270, 32900,
                /* red   */ 64000, 33000,
                /* green */ 30000, 60000,
                /* blue  */ 15000, 6000,
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

        if *(core::ptr::addr_of!(le) as png_const_bytep) != 0 {
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
        & !((PNG_FORMAT_FLAG_COLOR
            | PNG_FORMAT_FLAG_LINEAR
            | PNG_FORMAT_FLAG_ALPHA
            | PNG_FORMAT_FLAG_COLORMAP) as png_uint_32))
        != 0
    {
        png_error(
            png_ptr,
            b"png_write_image: unsupported transformation\0".as_ptr() as png_const_charp,
        );
    }

    {
        let mut row: png_const_bytep = (*display).buffer as png_const_bytep;
        let mut row_step: isize = (*display).row_stride as isize;

        if linear != 0 {
            row_step *= 2;
        }

        if row_step < 0 {
            row = row.offset(
                ((*image).height.wrapping_sub(1) as isize).wrapping_mul(row_step.wrapping_neg()),
            );
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
        let row: png_bytep = png_malloc(png_ptr, png_get_rowbytes(png_ptr, info_ptr)) as png_bytep;
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

/* image_memory_write */
unsafe extern "C" fn image_memory_write(png_ptr: png_structp, data: png_bytep, size: usize) {
    let display: *mut png_image_write_control =
        (*png_ptr).io_ptr /*backdoor: png_get_io_ptr(png_ptr)*/ as *mut png_image_write_control;
    let ob: png_alloc_size_t = (*display).output_bytes;

    /* Check for overflow; this should never happen: */
    if size <= ((0 as png_alloc_size_t).wrapping_sub(1)).wrapping_sub(ob) {
        /* I don't think libpng ever does this, but just in case: */
        if size > 0 {
            if (*display).memory_bytes >= ob.wrapping_add(size)
            /* writing */
            {
                memcpy(
                    (*display).memory.add(ob) as *mut c_void,
                    data as *const c_void,
                    size,
                );
            }

            /* Always update the size: */
            (*display).output_bytes = ob.wrapping_add(size);
        }
    } else {
        png_error(
            png_ptr,
            b"png_image_write_to_memory: PNG too big\0".as_ptr() as png_const_charp,
        );
    }
}

/* image_memory_flush */
unsafe extern "C" fn image_memory_flush(png_ptr: png_structp) {}

/* png_image_write_memory */
unsafe extern "C" fn png_image_write_memory(argument: png_voidp) -> c_int {
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

/* png_image_write_to_memory */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_write_to_memory(
    image: png_imagep,
    memory: *mut c_void,
    memory_bytes: *mut png_alloc_size_t,
    convert_to_8bit: c_int,
    buffer: *const c_void,
    row_stride: png_int_32,
    colormap: *const c_void,
) -> c_int {
    /* Write the image to the given buffer, or count the bytes if it is NULL */
    if image != core::ptr::null_mut() && (*image).version == PNG_IMAGE_VERSION {
        if memory_bytes != core::ptr::null_mut() && buffer != core::ptr::null() {
            /* This is to give the caller an easier error detection in the NULL
             * case and guard against uninitialized variable problems:
             */
            if memory == core::ptr::null_mut() {
                *memory_bytes = 0;
            }

            if png_image_write_init(image) != 0 {
                let mut display: png_image_write_control = core::mem::zeroed();
                let mut result: c_int;

                memset(
                    core::ptr::addr_of_mut!(display) as *mut c_void,
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
                    core::ptr::addr_of_mut!(display) as png_voidp,
                );
                png_image_free(image);

                /* write_memory returns true even if we ran out of buffer. */
                if result != 0 {
                    /* On out-of-buffer this function returns '0' but still updates
                     * memory_bytes:
                     */
                    if memory != core::ptr::null_mut() && display.output_bytes > *memory_bytes {
                        result = 0;
                    }

                    *memory_bytes = display.output_bytes;
                }

                return result;
            } else {
                return 0;
            }
        } else {
            return png_image_error(
                image,
                b"png_image_write_to_memory: invalid argument\0".as_ptr() as png_const_charp,
            );
        }
    } else if image != core::ptr::null_mut() {
        return png_image_error(
            image,
            b"png_image_write_to_memory: incorrect PNG_IMAGE_VERSION\0".as_ptr()
                as png_const_charp,
        );
    } else {
        return 0;
    }
}

/* png_image_write_to_stdio */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_write_to_stdio(
    image: png_imagep,
    file: *mut FILE,
    convert_to_8bit: c_int,
    buffer: *const c_void,
    row_stride: png_int_32,
    colormap: *const c_void,
) -> c_int {
    /* Write the image to the given FILE object. */
    if image != core::ptr::null_mut() && (*image).version == PNG_IMAGE_VERSION {
        if file != core::ptr::null_mut() && buffer != core::ptr::null() {
            if png_image_write_init(image) != 0 {
                let mut display: png_image_write_control = core::mem::zeroed();
                let result: c_int;

                /* This is slightly evil, but png_init_io doesn't do anything other
                 * than this and we haven't changed the standard IO functions so
                 * this saves a 'safe' function.
                 */
                (*(*(*image).opaque).png_ptr).io_ptr = file as png_voidp;

                memset(
                    core::ptr::addr_of_mut!(display) as *mut c_void,
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
                    core::ptr::addr_of_mut!(display) as png_voidp,
                );
                png_image_free(image);
                return result;
            } else {
                return 0;
            }
        } else {
            return png_image_error(
                image,
                b"png_image_write_to_stdio: invalid argument\0".as_ptr() as png_const_charp,
            );
        }
    } else if image != core::ptr::null_mut() {
        return png_image_error(
            image,
            b"png_image_write_to_stdio: incorrect PNG_IMAGE_VERSION\0".as_ptr() as png_const_charp,
        );
    } else {
        return 0;
    }
}

/* remove(), strerror() and __errno_location() come from src/ffi.rs. */

/* png_image_write_to_file */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_write_to_file(
    image: png_imagep,
    file_name: *const c_char,
    convert_to_8bit: c_int,
    buffer: *const c_void,
    row_stride: png_int_32,
    colormap: *const c_void,
) -> c_int {
    /* Write the image to the named file. */
    if image != core::ptr::null_mut() && (*image).version == PNG_IMAGE_VERSION {
        if file_name != core::ptr::null() && buffer != core::ptr::null() {
            let fp: *mut FILE = fopen(file_name, b"wb\0".as_ptr() as *const c_char);

            if fp != core::ptr::null_mut() {
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
                    return 0;
                }
            } else {
                return png_image_error(image, strerror(*__errno_location()));
            }
        } else {
            return png_image_error(
                image,
                b"png_image_write_to_file: invalid argument\0".as_ptr() as png_const_charp,
            );
        }
    } else if image != core::ptr::null_mut() {
        return png_image_error(
            image,
            b"png_image_write_to_file: incorrect PNG_IMAGE_VERSION\0".as_ptr() as png_const_charp,
        );
    } else {
        return 0;
    }
}
