//! pngread.c lines 1122-1954: the simplified read API initialization, header
//! reading, the IO setup entry points and the color-map construction helpers.
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

extern "C" {
    fn __errno_location() -> *mut c_int;
    fn strerror(errnum: c_int) -> *mut c_char;
}

/// `PNG_IMAGE_SAMPLE_CHANNELS(fmt)` from png.h
#[inline]
fn PNG_IMAGE_SAMPLE_CHANNELS(fmt: png_uint_32) -> png_uint_32 {
    (fmt & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1
}

/* Do all the *safe* initialization - 'safe' means that png_error won't be
 * called, so setting up the jmp_buf is not required.  This means that anything
 * called from here must *not* call png_malloc - it has to call png_malloc_warn
 * instead so that control is returned safely back to this routine.
 */
pub unsafe fn png_image_read_init(image: png_imagep) -> c_int {
    if (*image).opaque.is_null() {
        let mut png_ptr: png_structp = png_create_read_struct(
            PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
            image as png_voidp,
            Some(png_safe_error),
            Some(png_safe_warning),
        );

        /* And set the rest of the structure to NULL to ensure that the various
         * fields are consistent.
         */
        memset(image as *mut u8, 0, core::mem::size_of::<png_image>());
        (*image).version = PNG_IMAGE_VERSION;

        if !png_ptr.is_null() {
            let mut info_ptr: png_infop = png_create_info_struct(png_ptr);

            if !info_ptr.is_null() {
                let control: png_controlp =
                    png_malloc_warn(png_ptr, core::mem::size_of::<png_control>()) as png_controlp;

                if !control.is_null() {
                    memset(control as *mut u8, 0, core::mem::size_of::<png_control>());

                    (*control).png_ptr = png_ptr;
                    (*control).info_ptr = info_ptr;
                    (*control).set_for_write(false);

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

        return png_image_error(image, c"png_image_read: out of memory".as_ptr());
    }

    png_image_error(image, c"png_image_read: opaque pointer not NULL".as_ptr())
}

/* Utility to find the base format of a PNG file from a png_struct. */
pub unsafe fn png_image_format(png_ptr: png_structrp) -> png_uint_32 {
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

    if (*png_ptr).bit_depth == 16 {
        format |= PNG_FORMAT_FLAG_LINEAR;
    }

    if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_PALETTE) != 0 {
        format |= PNG_FORMAT_FLAG_COLORMAP;
    }

    format
}

pub unsafe fn chromaticities_match_sRGB(xy: *const png_xy) -> c_int {
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
pub unsafe fn png_gamma_not_sRGB(g: png_fixed_point) -> c_int {
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
pub unsafe fn png_image_is_not_sRGB(png_ptr: png_const_structrp) -> c_int {
    /* Does the colorspace **not** match sRGB?  The flag is only set if the
     * answer can be determined reliably.
     *
     * png_struct::chromaticities always exists since the simplified API
     * requires rgb-to-gray.  The mDCV, cICP and cHRM chunks may all set it to
     * a non-sRGB value, so it needs to be checked but **only** if one of
     * those chunks occurred in the file.
     */
    /* Highest priority: check to be safe. */
    if ((*png_ptr).chunks & png_chunk_flag_from_index(PNG_INDEX_cICP)) != 0
        || ((*png_ptr).chunks & png_chunk_flag_from_index(PNG_INDEX_mDCV)) != 0
    {
        return (chromaticities_match_sRGB(&(*png_ptr).chromaticities) == 0) as c_int;
    }

    /* If the image is marked as sRGB then it is... */
    if ((*png_ptr).chunks & png_chunk_flag_from_index(PNG_INDEX_sRGB)) != 0 {
        return 0;
    }

    /* Last stop: cHRM, must check: */
    if ((*png_ptr).chunks & png_chunk_flag_from_index(PNG_INDEX_cHRM)) != 0 {
        return (chromaticities_match_sRGB(&(*png_ptr).chromaticities) == 0) as c_int;
    }

    /* Else default to sRGB */
    0
}

pub unsafe extern "C-unwind" fn png_image_read_header(argument: png_voidp) -> c_int {
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
                cmap_entries = 1u32 << (*png_ptr).bit_depth;
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
pub unsafe extern "C-unwind" fn png_image_begin_read_from_stdio(
    image: png_imagep,
    file: *mut c_void,
) -> c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
        if !file.is_null() {
            if png_image_read_init(image) != 0 {
                /* This is slightly evil, but png_init_io doesn't do anything other
                 * than this and we haven't changed the standard IO functions so
                 * this saves a 'safe' function.
                 */
                (*(*(*image).opaque).png_ptr).io_ptr = file;
                return png_safe_execute(
                    image,
                    Some(png_image_read_header),
                    image as png_voidp,
                );
            }
        } else {
            return png_image_error(
                image,
                c"png_image_begin_read_from_stdio: invalid argument".as_ptr(),
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            c"png_image_begin_read_from_stdio: incorrect PNG_IMAGE_VERSION".as_ptr(),
        );
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_image_begin_read_from_file(
    image: png_imagep,
    file_name: *const c_char,
) -> c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
        if !file_name.is_null() {
            let fp: *mut c_void = crate::cabi::fopen(file_name, c"rb".as_ptr());

            if !fp.is_null() {
                if png_image_read_init(image) != 0 {
                    (*(*(*image).opaque).png_ptr).io_ptr = fp;
                    (*(*image).opaque).set_owned_file(true);
                    return png_safe_execute(
                        image,
                        Some(png_image_read_header),
                        image as png_voidp,
                    );
                }

                /* Clean up: just the opened file. */
                crate::cabi::fclose(fp);
            } else {
                return png_image_error(image, strerror(*__errno_location()));
            }
        } else {
            return png_image_error(
                image,
                c"png_image_begin_read_from_file: invalid argument".as_ptr(),
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            c"png_image_begin_read_from_file: incorrect PNG_IMAGE_VERSION".as_ptr(),
        );
    }

    0
}

pub unsafe extern "C-unwind" fn png_image_memory_read(
    png_ptr: png_structp,
    out: png_bytep,
    need: usize,
) {
    if !png_ptr.is_null() {
        let image: png_imagep = (*png_ptr).io_ptr as png_imagep;
        if !image.is_null() {
            let cp: png_controlp = (*image).opaque;
            if !cp.is_null() {
                let memory: png_const_bytep = (*cp).memory;
                let size: usize = (*cp).size;

                if !memory.is_null() && size >= need {
                    memcpy(out as *mut u8, memory as *const u8, need);
                    (*cp).memory = memory.add(need);
                    (*cp).size = size - need;
                    return;
                }

                png_error(png_ptr, c"read beyond end of data".as_ptr());
            }
        }

        png_error(png_ptr, c"invalid memory read".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_image_begin_read_from_memory(
    image: png_imagep,
    memory: png_const_voidp,
    size: usize,
) -> c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
        if !memory.is_null() && size > 0 {
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
                c"png_image_begin_read_from_memory: invalid argument".as_ptr(),
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            c"png_image_begin_read_from_memory: incorrect PNG_IMAGE_VERSION".as_ptr(),
        );
    }

    0
}

/* Utility function to skip chunks that are not used by the simplified image
 * read functions and an appropriate macro to call it.
 */
pub unsafe fn png_image_skip_unused_chunks(png_ptr: png_structrp) {
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
        static chunks_to_process: [png_byte; 35] = [
            98, 75, 71, 68, b'\0', /* bKGD */
            99, 72, 82, 77, b'\0', /* cHRM */
            99, 73, 67, 80, b'\0', /* cICP */
            103, 65, 77, 65, b'\0', /* gAMA */
            109, 68, 67, 86, b'\0', /* mDCV */
            115, 66, 73, 84, b'\0', /* sBIT */
            115, 82, 71, 66, b'\0', /* sRGB */
        ];

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
            (35 / 5) as c_int, /*SAFE*/
        );
    }
}

/* Utility functions to make particular color-maps */
pub unsafe fn set_file_encoding(display: *mut png_image_read_control) {
    let png_ptr: png_structrp = (*(*(*display).image).opaque).png_ptr;
    let g: png_fixed_point = png_resolve_file_gamma(png_ptr);

    /* PNGv3: the result may be 0 however the 'default_gamma' should have been
     * set before this is called so zero is an error:
     */
    if g == 0 {
        png_error(png_ptr, c"internal: default gamma not set".as_ptr());
    }

    if png_gamma_significant(g) != 0 {
        if png_gamma_not_sRGB(g) != 0 {
            (*display).file_encoding = P_FILE;
            (*display).gamma_to_linear = png_reciprocal(g);
        } else {
            (*display).file_encoding = P_sRGB;
        }
    } else {
        (*display).file_encoding = P_LINEAR8;
    }
}

pub unsafe fn decode_gamma(
    display: *mut png_image_read_control,
    value: png_uint_32,
    encoding: c_int,
) -> c_uint {
    let mut value = value;
    let mut encoding = encoding;

    if encoding == P_FILE
    /* double check */
    {
        encoding = (*display).file_encoding;
    }

    if encoding == P_NOTSET
    /* must be the file encoding */
    {
        set_file_encoding(display);
        encoding = (*display).file_encoding;
    }

    if encoding == P_FILE {
        value = png_gamma_16bit_correct(value * 257, (*display).gamma_to_linear) as png_uint_32;
    } else if encoding == P_sRGB {
        value = png_sRGB_table[value as usize] as png_uint_32;
    } else if encoding == P_LINEAR {
        /* nothing to do */
    } else if encoding == P_LINEAR8 {
        value *= 257;
    } else {
        png_error(
            (*(*(*display).image).opaque).png_ptr,
            c"unexpected encoding (internal error)".as_ptr(),
        );
    }

    value
}

pub unsafe fn png_colormap_compose(
    display: *mut png_image_read_control,
    foreground: png_uint_32,
    foreground_encoding: c_int,
    alpha: png_uint_32,
    background: png_uint_32,
    encoding: c_int,
) -> png_uint_32 {
    /* The file value is composed on the background, the background has the given
     * encoding and so does the result, the file is encoded with P_FILE and the
     * file and alpha are 8-bit values.  The (output) encoding will always be
     * P_LINEAR or P_sRGB.
     */
    let mut f: png_uint_32 = decode_gamma(display, foreground, foreground_encoding);
    let b: png_uint_32 = decode_gamma(display, background, encoding);

    /* The alpha is always an 8-bit value (it comes from the palette), the value
     * scaled by 255 is what PNG_sRGB_FROM_LINEAR requires.
     */
    f = f * alpha + b * (255 - alpha);

    if encoding == P_LINEAR {
        /* Scale to 65535; divide by 255, approximately (in fact this is extremely
         * accurate, it divides by 255.00000005937181414556, with no overflow.)
         */
        f *= 257; /* Now scaled by 65535 */
        f += f >> 16;
        f = (f + 32768) >> 16;
    } else
    /* P_sRGB */
    {
        f = PNG_sRGB_FROM_LINEAR(f) as png_uint_32;
    }

    f
}

/* NOTE: P_LINEAR values to this routine must be 16-bit, but P_FILE values must
 * be 8-bit.
 */
pub unsafe fn png_create_colormap_entry(
    display: *mut png_image_read_control,
    ip: png_uint_32,
    red: png_uint_32,
    green: png_uint_32,
    blue: png_uint_32,
    alpha: png_uint_32,
    encoding: c_int,
) {
    let mut red = red;
    let mut green = green;
    let mut blue = blue;
    let mut alpha = alpha;
    let mut encoding = encoding;

    let image: png_imagep = (*display).image;
    let output_encoding: c_int = if ((*image).format & PNG_FORMAT_FLAG_LINEAR) != 0 {
        P_LINEAR
    } else {
        P_sRGB
    };
    let convert_to_Y: c_int = (((*image).format & PNG_FORMAT_FLAG_COLOR) == 0
        && (red != green || green != blue)) as c_int;

    if ip > 255 {
        png_error(
            (*(*image).opaque).png_ptr,
            c"color-map index out of range".as_ptr(),
        );
    }

    /* Update the cache with whether the file gamma is significantly different
     * from sRGB.
     */
    if encoding == P_FILE {
        if (*display).file_encoding == P_NOTSET {
            set_file_encoding(display);
        }

        /* Note that the cached value may be P_FILE too, but if it is then the
         * gamma_to_linear member has been set.
         */
        encoding = (*display).file_encoding;
    }

    if encoding == P_FILE {
        let g: png_fixed_point = (*display).gamma_to_linear;

        red = png_gamma_16bit_correct(red * 257, g) as png_uint_32;
        green = png_gamma_16bit_correct(green * 257, g) as png_uint_32;
        blue = png_gamma_16bit_correct(blue * 257, g) as png_uint_32;

        if convert_to_Y != 0 || output_encoding == P_LINEAR {
            alpha *= 257;
            encoding = P_LINEAR;
        } else {
            red = PNG_sRGB_FROM_LINEAR(red * 255) as png_uint_32;
            green = PNG_sRGB_FROM_LINEAR(green * 255) as png_uint_32;
            blue = PNG_sRGB_FROM_LINEAR(blue * 255) as png_uint_32;
            encoding = P_sRGB;
        }
    } else if encoding == P_LINEAR8 {
        /* This encoding occurs quite frequently in test cases because PngSuite
         * includes a gAMA 1.0 chunk with most images.
         */
        red *= 257;
        green *= 257;
        blue *= 257;
        alpha *= 257;
        encoding = P_LINEAR;
    } else if encoding == P_sRGB && (convert_to_Y != 0 || output_encoding == P_LINEAR) {
        /* The values are 8-bit sRGB values, but must be converted to 16-bit
         * linear.
         */
        red = png_sRGB_table[red as usize] as png_uint_32;
        green = png_sRGB_table[green as usize] as png_uint_32;
        blue = png_sRGB_table[blue as usize] as png_uint_32;
        alpha *= 257;
        encoding = P_LINEAR;
    }

    /* This is set if the color isn't gray but the output is. */
    if encoding == P_LINEAR {
        if convert_to_Y != 0 {
            /* NOTE: these values are copied from png_do_rgb_to_gray */
            let mut y: png_uint_32 =
                6968u32 * red + 23434u32 * green + 2366u32 * blue;

            if output_encoding == P_LINEAR {
                y = (y + 16384) >> 15;
            } else {
                /* y is scaled by 32768, we need it scaled by 255: */
                y = (y + 128) >> 8;
                y *= 255;
                y = PNG_sRGB_FROM_LINEAR((y + 64) >> 7) as png_uint_32;
                alpha = PNG_DIV257(alpha);
                encoding = P_sRGB;
            }

            green = y;
            red = green;
            blue = red;
        } else if output_encoding == P_sRGB {
            red = PNG_sRGB_FROM_LINEAR(red * 255) as png_uint_32;
            green = PNG_sRGB_FROM_LINEAR(green * 255) as png_uint_32;
            blue = PNG_sRGB_FROM_LINEAR(blue * 255) as png_uint_32;
            alpha = PNG_DIV257(alpha);
            encoding = P_sRGB;
        }
    }

    if encoding != output_encoding {
        png_error(
            (*(*image).opaque).png_ptr,
            c"bad encoding (internal error)".as_ptr(),
        );
    }

    /* Store the value. */
    {
        let afirst: c_int = (((*image).format & PNG_FORMAT_FLAG_AFIRST) != 0
            && ((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0) as c_int;
        let bgr: c_int = if ((*image).format & PNG_FORMAT_FLAG_BGR) != 0 {
            2
        } else {
            0
        };

        if output_encoding == P_LINEAR {
            let mut entry: png_uint_16p = (*display).colormap as png_uint_16p;

            entry = entry.add((ip * PNG_IMAGE_SAMPLE_CHANNELS((*image).format)) as usize);

            /* The linear 16-bit values must be pre-multiplied by the alpha channel
             * value, if less than 65535 (this is, effectively, composite on black
             * if the alpha channel is removed.)
             */
            match PNG_IMAGE_SAMPLE_CHANNELS((*image).format) {
                4 => {
                    *entry.add(if afirst != 0 { 0 } else { 3 }) = alpha as png_uint_16;
                    /* FALLTHROUGH */

                    /* case 3: */
                    if alpha < 65535 {
                        if alpha > 0 {
                            blue = (blue * alpha + 32767u32) / 65535u32;
                            green = (green * alpha + 32767u32) / 65535u32;
                            red = (red * alpha + 32767u32) / 65535u32;
                        } else {
                            blue = 0;
                            green = 0;
                            red = 0;
                        }
                    }
                    *entry.add((afirst + (2 ^ bgr)) as usize) = blue as png_uint_16;
                    *entry.add((afirst + 1) as usize) = green as png_uint_16;
                    *entry.add((afirst + bgr) as usize) = red as png_uint_16;
                }

                3 => {
                    if alpha < 65535 {
                        if alpha > 0 {
                            blue = (blue * alpha + 32767u32) / 65535u32;
                            green = (green * alpha + 32767u32) / 65535u32;
                            red = (red * alpha + 32767u32) / 65535u32;
                        } else {
                            blue = 0;
                            green = 0;
                            red = 0;
                        }
                    }
                    *entry.add((afirst + (2 ^ bgr)) as usize) = blue as png_uint_16;
                    *entry.add((afirst + 1) as usize) = green as png_uint_16;
                    *entry.add((afirst + bgr) as usize) = red as png_uint_16;
                }

                2 => {
                    *entry.add((1 ^ afirst) as usize) = alpha as png_uint_16;
                    /* FALLTHROUGH */

                    /* case 1: */
                    if alpha < 65535 {
                        if alpha > 0 {
                            green = (green * alpha + 32767u32) / 65535u32;
                        } else {
                            green = 0;
                        }
                    }
                    *entry.add(afirst as usize) = green as png_uint_16;
                }

                1 => {
                    if alpha < 65535 {
                        if alpha > 0 {
                            green = (green * alpha + 32767u32) / 65535u32;
                        } else {
                            green = 0;
                        }
                    }
                    *entry.add(afirst as usize) = green as png_uint_16;
                }

                _ => {}
            }
        } else
        /* output encoding is P_sRGB */
        {
            let mut entry: png_bytep = (*display).colormap as png_bytep;

            entry = entry.add((ip * PNG_IMAGE_SAMPLE_CHANNELS((*image).format)) as usize);

            match PNG_IMAGE_SAMPLE_CHANNELS((*image).format) {
                4 => {
                    *entry.add(if afirst != 0 { 0 } else { 3 }) = alpha as png_byte;
                    /* FALLTHROUGH */
                    /* case 3: */
                    *entry.add((afirst + (2 ^ bgr)) as usize) = blue as png_byte;
                    *entry.add((afirst + 1) as usize) = green as png_byte;
                    *entry.add((afirst + bgr) as usize) = red as png_byte;
                }
                3 => {
                    *entry.add((afirst + (2 ^ bgr)) as usize) = blue as png_byte;
                    *entry.add((afirst + 1) as usize) = green as png_byte;
                    *entry.add((afirst + bgr) as usize) = red as png_byte;
                }

                2 => {
                    *entry.add((1 ^ afirst) as usize) = alpha as png_byte;
                    /* FALLTHROUGH */
                    /* case 1: */
                    *entry.add(afirst as usize) = green as png_byte;
                }
                1 => {
                    *entry.add(afirst as usize) = green as png_byte;
                }

                _ => {}
            }
        }
    }
}

pub unsafe fn make_gray_file_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i: c_uint;

    i = 0;
    while i < 256 {
        png_create_colormap_entry(display, i, i, i, i, 255, P_FILE);
        i += 1;
    }

    i as c_int
}

pub unsafe fn make_gray_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i: c_uint;

    i = 0;
    while i < 256 {
        png_create_colormap_entry(display, i, i, i, i, 255, P_sRGB);
        i += 1;
    }

    i as c_int
}

pub unsafe fn make_ga_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i: c_uint;
    let mut a: c_uint;

    /* Alpha is retained, the output will be a color-map with entries
     * selected by six levels of alpha.  One transparent entry, 6 gray
     * levels for all the intermediate alpha values, leaving 230 entries
     * for the opaque grays.  The color-map entries are the six values
     * [0..5]*51, the GA processing uses PNG_DIV51(value) to find the
     * relevant entry.
     *
     * if (alpha > 229) // opaque
     * {
     *    // The 231 entries are selected to make the math below work:
     *    base = 0;
     *    entry = (231 * gray + 128) >> 8;
     * }
     * else if (alpha < 26) // transparent
     * {
     *    base = 231;
     *    entry = 0;
     * }
     * else // partially opaque
     * {
     *    base = 226 + 6 * PNG_DIV51(alpha);
     *    entry = PNG_DIV51(gray);
     * }
     */
    i = 0;
    while i < 231 {
        let gray: c_uint = (i * 256 + 115) / 231;
        png_create_colormap_entry(display, i, gray, gray, gray, 255, P_sRGB);
        i += 1;
    }

    /* 255 is used here for the component values for consistency with the code
     * that undoes premultiplication in pngwrite.c.
     */
    png_create_colormap_entry(display, i, 255, 255, 255, 0, P_sRGB);
    i += 1;

    a = 1;
    while a < 5 {
        let mut g: c_uint;

        g = 0;
        while g < 6 {
            png_create_colormap_entry(display, i, g * 51, g * 51, g * 51, a * 51, P_sRGB);
            i += 1;
            g += 1;
        }
        a += 1;
    }

    i as c_int
}

pub unsafe fn make_rgb_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i: c_uint;
    let mut r: c_uint;

    /* Build a 6x6x6 opaque RGB cube */
    i = 0;
    r = 0;
    while r < 6 {
        let mut g: c_uint;

        g = 0;
        while g < 6 {
            let mut b: c_uint;

            b = 0;
            while b < 6 {
                png_create_colormap_entry(display, i, r * 51, g * 51, b * 51, 255, P_sRGB);
                i += 1;
                b += 1;
            }
            g += 1;
        }
        r += 1;
    }

    i as c_int
}
