//! Translation of c_src/src/pngread.c lines 1128..2812
#![allow(non_snake_case)]
#![allow(unused_assignments)]
use crate::prelude::*;

/* --------------------------------------------------------------- */
/* Locally-defined helpers (private macros from png.h/png.c used    */
/* only in this file; they are not exported through the prelude).   */
/* --------------------------------------------------------------- */

/// `PNG_IMAGE_SAMPLE_CHANNELS(fmt)`
#[inline]
fn PNG_IMAGE_SAMPLE_CHANNELS(fmt: png_uint_32) -> c_uint {
    ((fmt & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1) as c_uint
}

/// `PNG_IMAGE_SAMPLE_COMPONENT_SIZE(fmt)`
#[inline]
fn PNG_IMAGE_SAMPLE_COMPONENT_SIZE(fmt: png_uint_32) -> c_uint {
    (((fmt & PNG_FORMAT_FLAG_LINEAR) >> 2) + 1) as c_uint
}

/// `PNG_IMAGE_SAMPLE_SIZE(fmt)`
#[inline]
fn PNG_IMAGE_SAMPLE_SIZE(fmt: png_uint_32) -> c_uint {
    PNG_IMAGE_SAMPLE_CHANNELS(fmt) * PNG_IMAGE_SAMPLE_COMPONENT_SIZE(fmt)
}

/// `PNG_BACKGROUND_GAMMA_SCREEN` (png.h)
const PNG_BACKGROUND_GAMMA_SCREEN: c_int = 1;

/// `PNG_RGB_INDEX(r,g,b)` from pngread.c
#[inline]
fn PNG_RGB_INDEX(r: png_uint_32, g: png_uint_32, b: png_uint_32) -> png_byte {
    (6u32.wrapping_mul(
        6u32.wrapping_mul(PNG_DIV51(r)).wrapping_add(PNG_DIV51(g)),
    )
    .wrapping_add(PNG_DIV51(b))) as png_byte
}

const PNG_GRAY_COLORMAP_ENTRIES: c_uint = 256;
const PNG_GA_COLORMAP_ENTRIES: c_uint = 256;
const PNG_RGB_COLORMAP_ENTRIES: c_uint = 216;

/* --------------------------------------------------------------- */
/* png_image_read_init (static)                                     */
/* --------------------------------------------------------------- */

/* png_safe_error is `-> !` in the Rust translation, but png_create_read_struct
 * takes a png_error_ptr (`-> ()`).  In C the never-returning attribute is not
 * part of the pointer type, so a thin `-> ()` shim reproduces the C call.
 */
unsafe extern "C" fn png_image_read_safe_error_shim(
    png_ptr: png_structp,
    error_message: png_const_charp,
) {
    png_safe_error(png_ptr, error_message)
}

pub unsafe extern "C" fn png_image_read_init(image: png_imagep) -> c_int {
    if (*image).opaque.is_null() {
        let mut png_ptr: png_structp = png_create_read_struct(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            image as png_voidp,
            Some(png_image_read_safe_error_shim),
            Some(png_safe_warning),
        );

        /* And set the rest of the structure to NULL to ensure that the various
         * fields are consistent.
         */
        memset(image as png_voidp, 0, core::mem::size_of::<png_image>());
        (*image).version = PNG_IMAGE_VERSION;

        if !png_ptr.is_null() {
            let info_ptr: png_infop = png_create_info_struct(png_ptr);

            if !info_ptr.is_null() {
                let control: png_controlp = png_malloc_warn(
                    png_ptr,
                    core::mem::size_of::<png_control>(),
                ) as png_controlp;

                if !control.is_null() {
                    memset(control as png_voidp, 0, core::mem::size_of::<png_control>());

                    (*control).png_ptr = png_ptr;
                    (*control).info_ptr = info_ptr;
                    (*control).for_write = 0;

                    (*image).opaque = control;
                    return 1;
                }

                /* Error clean up */
                let mut info_ptr_local = info_ptr;
                png_destroy_info_struct(png_ptr, &mut info_ptr_local);
            }

            png_destroy_read_struct(&mut png_ptr, core::ptr::null_mut(), core::ptr::null_mut());
        }

        return png_image_error(image, cstr(b"png_image_read: out of memory\0"));
    }

    png_image_error(image, cstr(b"png_image_read: opaque pointer not NULL\0"))
}

/* --------------------------------------------------------------- */
/* png_image_format (static)                                        */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn png_image_format(png_ptr: png_structrp) -> png_uint_32 {
    let mut format: png_uint_32 = 0;

    if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        format |= PNG_FORMAT_FLAG_COLOR;
    }

    if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0 {
        format |= PNG_FORMAT_FLAG_ALPHA;
    }
    /* Use png_ptr here, not info_ptr, because by examination png_handle_tRNS
     * sets the png_struct fields; that's all we are interested in here.
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

/* --------------------------------------------------------------- */
/* chromaticities_match_sRGB (static)                               */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn chromaticities_match_sRGB(xy: *const png_xy) -> c_int {
    const sRGB_TOLERANCE: png_fixed_point = 1000;
    /* From ITU-R BT.709-3 */
    let sRGB_xy: png_xy = png_xy {
        redx: 64000,
        redy: 33000,
        greenx: 30000,
        greeny: 60000,
        bluex: 15000,
        bluey: 6000,
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

/* --------------------------------------------------------------- */
/* png_gamma_not_sRGB (static)                                      */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn png_gamma_not_sRGB(g: png_fixed_point) -> c_int {
    /* 1.6.47: use the same sanity checks as used in pngrtran.c */
    if g < PNG_LIB_GAMMA_MIN || g > PNG_LIB_GAMMA_MAX {
        return 0; /* Includes the uninitialized value 0 */
    }

    png_gamma_significant((g * 11 + 2) / 5 /* i.e. *2.2, rounded */)
}

/* --------------------------------------------------------------- */
/* png_image_is_not_sRGB (static)                                   */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn png_image_is_not_sRGB(png_ptr: png_const_structrp) -> c_int {
    /* Highest priority: check to be safe. */
    if png_file_has_chunk(png_ptr, PNG_INDEX_cICP) || png_file_has_chunk(png_ptr, PNG_INDEX_mDCV) {
        return (chromaticities_match_sRGB(&(*png_ptr).chromaticities) == 0) as c_int;
    }

    /* If the image is marked as sRGB then it is... */
    if png_file_has_chunk(png_ptr, PNG_INDEX_sRGB) {
        return 0;
    }

    /* Last stop: cHRM, must check: */
    if png_file_has_chunk(png_ptr, PNG_INDEX_cHRM) {
        return (chromaticities_match_sRGB(&(*png_ptr).chromaticities) == 0) as c_int;
    }

    /* Else default to sRGB */
    0
}

/* --------------------------------------------------------------- */
/* png_image_read_header (static)                                   */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn png_image_read_header(argument: png_voidp) -> c_int {
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

        /* Greyscale images don't (typically) have colour space information */
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

/* --------------------------------------------------------------- */
/* png_image_begin_read_from_stdio (exported)                       */
/* --------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_begin_read_from_stdio(
    image: png_imagep,
    file: *mut FILE,
) -> c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
        if !file.is_null() {
            if png_image_read_init(image) != 0 {
                /* This is slightly evil, but png_init_io doesn't do anything
                 * other than this and we haven't changed the standard IO
                 * functions so this saves a 'safe' function.
                 */
                (*(*(*image).opaque).png_ptr).io_ptr = file as png_voidp;
                return png_safe_execute(image, Some(png_image_read_header), image as png_voidp);
            }
        } else {
            return png_image_error(
                image,
                cstr(b"png_image_begin_read_from_stdio: invalid argument\0"),
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            cstr(b"png_image_begin_read_from_stdio: incorrect PNG_IMAGE_VERSION\0"),
        );
    }

    0
}

/* --------------------------------------------------------------- */
/* png_image_begin_read_from_file (exported)                        */
/* --------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_begin_read_from_file(
    image: png_imagep,
    file_name: *const c_char,
) -> c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
        if !file_name.is_null() {
            let fp: *mut FILE = fopen(file_name, cstr(b"rb\0"));

            if !fp.is_null() {
                if png_image_read_init(image) != 0 {
                    (*(*(*image).opaque).png_ptr).io_ptr = fp as png_voidp;
                    (*(*image).opaque).owned_file = 1;
                    return png_safe_execute(image, Some(png_image_read_header), image as png_voidp);
                }

                /* Clean up: just the opened file. */
                fclose(fp);
            } else {
                return png_image_error(image, strerror(errno()));
            }
        } else {
            return png_image_error(
                image,
                cstr(b"png_image_begin_read_from_file: invalid argument\0"),
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            cstr(b"png_image_begin_read_from_file: incorrect PNG_IMAGE_VERSION\0"),
        );
    }

    0
}

/* --------------------------------------------------------------- */
/* png_image_memory_read (static, PNGCBAPI)                         */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn png_image_memory_read(
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
                    memcpy(out as png_voidp, memory as png_const_voidp, need);
                    (*cp).memory = memory.add(need);
                    (*cp).size = size - need;
                    return;
                }

                png_error(png_ptr, cstr(b"read beyond end of data\0"));
            }
        }

        png_error(png_ptr, cstr(b"invalid memory read\0"));
    }
}

/* --------------------------------------------------------------- */
/* png_image_begin_read_from_memory (exported)                      */
/* --------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_begin_read_from_memory(
    image: png_imagep,
    memory: png_const_voidp,
    size: usize,
) -> c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION {
        if !memory.is_null() && size > 0 {
            if png_image_read_init(image) != 0 {
                /* Now set the IO functions to read from the memory buffer and
                 * store it into io_ptr.
                 */
                (*(*image).opaque).memory = memory as png_const_bytep;
                (*(*image).opaque).size = size;
                (*(*(*image).opaque).png_ptr).io_ptr = image as png_voidp;
                (*(*(*image).opaque).png_ptr).read_data_fn = Some(png_image_memory_read);

                return png_safe_execute(image, Some(png_image_read_header), image as png_voidp);
            }
        } else {
            return png_image_error(
                image,
                cstr(b"png_image_begin_read_from_memory: invalid argument\0"),
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            cstr(b"png_image_begin_read_from_memory: incorrect PNG_IMAGE_VERSION\0"),
        );
    }

    0
}

/* --------------------------------------------------------------- */
/* png_image_skip_unused_chunks (static)                            */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn png_image_skip_unused_chunks(png_ptr: png_structrp) {
    static chunks_to_process: [png_byte; 35] = [
        98, 75, 71, 68, 0, /* bKGD */
        99, 72, 82, 77, 0, /* cHRM */
        99, 73, 67, 80, 0, /* cICP */
        103, 65, 77, 65, 0, /* gAMA */
        109, 68, 67, 86, 0, /* mDCV */
        115, 66, 73, 84, 0, /* sBIT */
        115, 82, 71, 66, 0, /* sRGB */
    ];

    /* Ignore unknown chunks and all other chunks except for the
     * IHDR, PLTE, tRNS, IDAT, and IEND chunks.
     */
    png_set_keep_unknown_chunks(png_ptr, PNG_HANDLE_CHUNK_NEVER, core::ptr::null(), -1);

    /* But do not ignore image data handling chunks */
    png_set_keep_unknown_chunks(
        png_ptr,
        PNG_HANDLE_CHUNK_AS_DEFAULT,
        chunks_to_process.as_ptr(),
        (core::mem::size_of_val(&chunks_to_process) / 5) as c_int,
    );
}

/* --------------------------------------------------------------- */
/* set_file_encoding (static)                                       */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn set_file_encoding(display: *mut png_image_read_control) {
    let png_ptr: png_structrp = (*(*(*display).image).opaque).png_ptr;
    let g: png_fixed_point = png_resolve_file_gamma(png_ptr);

    /* PNGv3: the result may be 0 however the 'default_gamma' should have been
     * set before this is called so zero is an error:
     */
    if g == 0 {
        png_error(png_ptr, cstr(b"internal: default gamma not set\0"));
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

/* --------------------------------------------------------------- */
/* decode_gamma (static)                                            */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn decode_gamma(
    display: *mut png_image_read_control,
    mut value: png_uint_32,
    mut encoding: c_int,
) -> c_uint {
    if encoding == P_FILE {
        /* double check */
        encoding = (*display).file_encoding;
    }

    if encoding == P_NOTSET {
        /* must be the file encoding */
        set_file_encoding(display);
        encoding = (*display).file_encoding;
    }

    match encoding {
        P_FILE => {
            value = png_gamma_16bit_correct(
                value.wrapping_mul(257),
                (*display).gamma_to_linear,
            ) as png_uint_32;
        }

        P_sRGB => {
            value = png_sRGB_table[value as usize] as png_uint_32;
        }

        P_LINEAR => {}

        P_LINEAR8 => {
            value = value.wrapping_mul(257);
        }

        _ => {
            png_error(
                (*(*(*display).image).opaque).png_ptr,
                cstr(b"unexpected encoding (internal error)\0"),
            );
        }
    }

    value as c_uint
}

/* --------------------------------------------------------------- */
/* png_colormap_compose (static)                                   */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn png_colormap_compose(
    display: *mut png_image_read_control,
    foreground: png_uint_32,
    foreground_encoding: c_int,
    alpha: png_uint_32,
    background: png_uint_32,
    encoding: c_int,
) -> png_uint_32 {
    /* The file value is composed on the background, the background has the
     * given encoding and so does the result, the file is encoded with P_FILE
     * and the file and alpha are 8-bit values.  The (output) encoding will
     * always be P_LINEAR or P_sRGB.
     */
    let mut f: png_uint_32 = decode_gamma(display, foreground, foreground_encoding) as png_uint_32;
    let b: png_uint_32 = decode_gamma(display, background, encoding) as png_uint_32;

    /* The alpha is always an 8-bit value (it comes from the palette), the value
     * scaled by 255 is what PNG_sRGB_FROM_LINEAR requires.
     */
    f = f
        .wrapping_mul(alpha)
        .wrapping_add(b.wrapping_mul(255u32.wrapping_sub(alpha)));

    if encoding == P_LINEAR {
        /* Scale to 65535; divide by 255, approximately */
        f = f.wrapping_mul(257); /* Now scaled by 65535 */
        f = f.wrapping_add(f >> 16);
        f = (f.wrapping_add(32768)) >> 16;
    } else {
        /* P_sRGB */
        f = PNG_sRGB_FROM_LINEAR(f) as png_uint_32;
    }

    f
}

/* --------------------------------------------------------------- */
/* png_create_colormap_entry (static)                               */
/* --------------------------------------------------------------- */

/* NOTE: P_LINEAR values to this routine must be 16-bit, but P_FILE values
 * must be 8-bit.
 */
pub unsafe extern "C" fn png_create_colormap_entry(
    display: *mut png_image_read_control,
    ip: png_uint_32,
    mut red: png_uint_32,
    mut green: png_uint_32,
    mut blue: png_uint_32,
    mut alpha: png_uint_32,
    mut encoding: c_int,
) {
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
            cstr(b"color-map index out of range\0"),
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

        red = png_gamma_16bit_correct(red.wrapping_mul(257), g) as png_uint_32;
        green = png_gamma_16bit_correct(green.wrapping_mul(257), g) as png_uint_32;
        blue = png_gamma_16bit_correct(blue.wrapping_mul(257), g) as png_uint_32;

        if convert_to_Y != 0 || output_encoding == P_LINEAR {
            alpha = alpha.wrapping_mul(257);
            encoding = P_LINEAR;
        } else {
            red = PNG_sRGB_FROM_LINEAR(red.wrapping_mul(255)) as png_uint_32;
            green = PNG_sRGB_FROM_LINEAR(green.wrapping_mul(255)) as png_uint_32;
            blue = PNG_sRGB_FROM_LINEAR(blue.wrapping_mul(255)) as png_uint_32;
            encoding = P_sRGB;
        }
    } else if encoding == P_LINEAR8 {
        /* This encoding occurs quite frequently in test cases because PngSuite
         * includes a gAMA 1.0 chunk with most images.
         */
        red = red.wrapping_mul(257);
        green = green.wrapping_mul(257);
        blue = blue.wrapping_mul(257);
        alpha = alpha.wrapping_mul(257);
        encoding = P_LINEAR;
    } else if encoding == P_sRGB && (convert_to_Y != 0 || output_encoding == P_LINEAR) {
        /* The values are 8-bit sRGB values, but must be converted to 16-bit
         * linear.
         */
        red = png_sRGB_table[red as usize] as png_uint_32;
        green = png_sRGB_table[green as usize] as png_uint_32;
        blue = png_sRGB_table[blue as usize] as png_uint_32;
        alpha = alpha.wrapping_mul(257);
        encoding = P_LINEAR;
    }

    /* This is set if the color isn't gray but the output is. */
    if encoding == P_LINEAR {
        if convert_to_Y != 0 {
            /* NOTE: these values are copied from png_do_rgb_to_gray */
            let mut y: png_uint_32 = (6968u32)
                .wrapping_mul(red)
                .wrapping_add((23434u32).wrapping_mul(green))
                .wrapping_add((2366u32).wrapping_mul(blue));

            if output_encoding == P_LINEAR {
                y = (y.wrapping_add(16384)) >> 15;
            } else {
                /* y is scaled by 32768, we need it scaled by 255: */
                y = (y.wrapping_add(128)) >> 8;
                y = y.wrapping_mul(255);
                y = PNG_sRGB_FROM_LINEAR((y.wrapping_add(64)) >> 7) as png_uint_32;
                alpha = PNG_DIV257(alpha);
                encoding = P_sRGB;
            }

            blue = y;
            red = y;
            green = y;
        } else if output_encoding == P_sRGB {
            red = PNG_sRGB_FROM_LINEAR(red.wrapping_mul(255)) as png_uint_32;
            green = PNG_sRGB_FROM_LINEAR(green.wrapping_mul(255)) as png_uint_32;
            blue = PNG_sRGB_FROM_LINEAR(blue.wrapping_mul(255)) as png_uint_32;
            alpha = PNG_DIV257(alpha);
            encoding = P_sRGB;
        }
    }

    if encoding != output_encoding {
        png_error(
            (*(*image).opaque).png_ptr,
            cstr(b"bad encoding (internal error)\0"),
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

            /* The linear 16-bit values must be pre-multiplied by the alpha
             * channel value, if less than 65535.
             */
            match PNG_IMAGE_SAMPLE_CHANNELS((*image).format) {
                4 => {
                    *entry.add(if afirst != 0 { 0 } else { 3 }) = alpha as png_uint_16;
                    /* FALLTHROUGH */
                    if alpha < 65535 {
                        if alpha > 0 {
                            blue = (blue.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                            green = (green.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                            red = (red.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                        } else {
                            red = 0;
                            green = 0;
                            blue = 0;
                        }
                    }
                    *entry.add((afirst + (2 ^ bgr)) as usize) = blue as png_uint_16;
                    *entry.add((afirst + 1) as usize) = green as png_uint_16;
                    *entry.add((afirst + bgr) as usize) = red as png_uint_16;
                }

                3 => {
                    if alpha < 65535 {
                        if alpha > 0 {
                            blue = (blue.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                            green = (green.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                            red = (red.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                        } else {
                            red = 0;
                            green = 0;
                            blue = 0;
                        }
                    }
                    *entry.add((afirst + (2 ^ bgr)) as usize) = blue as png_uint_16;
                    *entry.add((afirst + 1) as usize) = green as png_uint_16;
                    *entry.add((afirst + bgr) as usize) = red as png_uint_16;
                }

                2 => {
                    *entry.add((1 ^ afirst) as usize) = alpha as png_uint_16;
                    /* FALLTHROUGH */
                    if alpha < 65535 {
                        if alpha > 0 {
                            green = (green.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                        } else {
                            green = 0;
                        }
                    }
                    *entry.add(afirst as usize) = green as png_uint_16;
                }

                1 => {
                    if alpha < 65535 {
                        if alpha > 0 {
                            green = (green.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                        } else {
                            green = 0;
                        }
                    }
                    *entry.add(afirst as usize) = green as png_uint_16;
                }

                _ => {}
            }
        } else {
            /* output encoding is P_sRGB */
            let mut entry: png_bytep = (*display).colormap as png_bytep;

            entry = entry.add((ip * PNG_IMAGE_SAMPLE_CHANNELS((*image).format)) as usize);

            match PNG_IMAGE_SAMPLE_CHANNELS((*image).format) {
                4 => {
                    *entry.add(if afirst != 0 { 0 } else { 3 }) = alpha as png_byte;
                    /* FALLTHROUGH */
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

/* --------------------------------------------------------------- */
/* make_gray_file_colormap (static)                                 */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn make_gray_file_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i: c_uint = 0;

    while i < 256 {
        png_create_colormap_entry(display, i, i, i, i, 255, P_FILE);
        i += 1;
    }

    i as c_int
}

/* --------------------------------------------------------------- */
/* make_gray_colormap (static)                                      */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn make_gray_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i: c_uint = 0;

    while i < 256 {
        png_create_colormap_entry(display, i, i, i, i, 255, P_sRGB);
        i += 1;
    }

    i as c_int
}

/* --------------------------------------------------------------- */
/* make_ga_colormap (static)                                        */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn make_ga_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i: c_uint;
    let mut a: c_uint;

    /* Alpha is retained; see comments in the C source. */
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
        let mut g: c_uint = 0;

        while g < 6 {
            png_create_colormap_entry(
                display,
                i,
                g * 51,
                g * 51,
                g * 51,
                a * 51,
                P_sRGB,
            );
            i += 1;
            g += 1;
        }
        a += 1;
    }

    i as c_int
}

/* --------------------------------------------------------------- */
/* make_rgb_colormap (static)                                       */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn make_rgb_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i: c_uint;
    let mut r: c_uint;

    /* Build a 6x6x6 opaque RGB cube */
    i = 0;
    r = 0;
    while r < 6 {
        let mut g: c_uint = 0;

        while g < 6 {
            let mut b: c_uint = 0;

            while b < 6 {
                png_create_colormap_entry(
                    display,
                    i,
                    r * 51,
                    g * 51,
                    b * 51,
                    255,
                    P_sRGB,
                );
                i += 1;
                b += 1;
            }
            g += 1;
        }
        r += 1;
    }

    i as c_int
}

/* --------------------------------------------------------------- */
/* png_image_read_colormap (static)                                 */
/* --------------------------------------------------------------- */

pub unsafe extern "C" fn png_image_read_colormap(argument: png_voidp) -> c_int {
    let display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let image: png_imagep = (*display).image;

    let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
    let output_format: png_uint_32 = (*image).format;
    let output_encoding: c_int = if (output_format & PNG_FORMAT_FLAG_LINEAR) != 0 {
        P_LINEAR
    } else {
        P_sRGB
    };

    let mut cmap_entries: c_uint = 0;
    let mut output_processing: c_uint; /* Output processing option */
    let mut data_encoding: c_int = P_NOTSET; /* Encoding libpng must produce */

    /* Background information; the background color and the index of this color
     * in the color-map if it exists (else 256).
     */
    let mut background_index: c_uint = 256;
    let mut back_r: png_uint_32;
    let mut back_g: png_uint_32;
    let mut back_b: png_uint_32;

    /* Flags to accumulate things that need to be done to the input. */
    let mut expand_tRNS: c_int = 0;

    /* Exclude the NYI feature of compositing onto a color-mapped buffer. */
    if (((*png_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0
        || (*png_ptr).num_trans > 0)
        && ((output_format & PNG_FORMAT_FLAG_ALPHA) == 0)
    {
        if output_encoding == P_LINEAR {
            /* compose on black */
            back_b = 0;
            back_g = 0;
            back_r = 0;
        } else if (*display).background.is_null() {
            /* no way to remove it */
            png_error(
                png_ptr,
                cstr(b"background color must be supplied to remove alpha/transparency\0"),
            );
        } else {
            /* Get a copy of the background color. */
            back_g = (*(*display).background).green as png_uint_32;
            if (output_format & PNG_FORMAT_FLAG_COLOR) != 0 {
                back_r = (*(*display).background).red as png_uint_32;
                back_b = (*(*display).background).blue as png_uint_32;
            } else {
                back_b = back_g;
                back_r = back_g;
            }
        }
    } else if output_encoding == P_LINEAR {
        back_b = 65535;
        back_r = 65535;
        back_g = 65535;
    } else {
        back_b = 255;
        back_r = 255;
        back_g = 255;
    }

    /* Default the input file gamma if required. */
    if (*png_ptr).bit_depth == 16 && ((*image).flags & PNG_IMAGE_FLAG_16BIT_sRGB) == 0 {
        (*png_ptr).default_gamma = PNG_GAMMA_LINEAR;
    } else {
        (*png_ptr).default_gamma = PNG_GAMMA_sRGB_INVERSE;
    }

    /* Decide what to do based on the PNG color type of the input data.  The
     * outer 'switch' is translated as a labelled block so that C `break`
     * (early exit from the switch) maps to `break 'colortype`.
     */
    'colortype: {
        match (*png_ptr).color_type as c_int {
            PNG_COLOR_TYPE_GRAY => {
                if (*png_ptr).bit_depth <= 8 {
                    /* At most 256 colors in the output, regardless of
                     * transparency.
                     */
                    let step: c_uint;
                    let mut i: c_uint;
                    let mut val: c_uint;
                    let mut trans: c_uint = 256; /*ignore*/
                    let mut back_alpha: c_uint = 0;

                    cmap_entries = 1u32 << (*png_ptr).bit_depth;
                    if cmap_entries > (*image).colormap_entries {
                        png_error(png_ptr, cstr(b"gray[8] color-map: too few entries\0"));
                    }

                    step = 255 / (cmap_entries - 1);
                    output_processing = PNG_CMAP_NONE as c_uint;

                    /* If there is a tRNS chunk then this either selects a
                     * transparent value or, if the output has no alpha, the
                     * background color.
                     */
                    if (*png_ptr).num_trans > 0 {
                        trans = (*png_ptr).trans_color.gray as c_uint;

                        if (output_format & PNG_FORMAT_FLAG_ALPHA) == 0 {
                            back_alpha = if output_encoding == P_LINEAR { 65535 } else { 255 };
                        }
                    }

                    i = 0;
                    val = 0;
                    while i < cmap_entries {
                        /* 'i' is a file value. */
                        if i != trans {
                            png_create_colormap_entry(
                                display, i, val, val, val, 255,
                                P_FILE, /*8-bit with file gamma*/
                            );
                        } else {
                            /* Else this entry is transparent. */
                            png_create_colormap_entry(
                                display, i, back_r, back_g, back_b, back_alpha,
                                output_encoding,
                            );
                        }
                        i += 1;
                        val = val.wrapping_add(step);
                    }

                    /* We need libpng to preserve the original encoding. */
                    data_encoding = P_FILE;

                    /* The rows may need to be expanded to 1 byte per pixel. */
                    if (*png_ptr).bit_depth < 8 {
                        png_set_packing(png_ptr);
                    }
                } else {
                    /* bit depth is 16 */
                    data_encoding = P_sRGB;

                    if PNG_GRAY_COLORMAP_ENTRIES > (*image).colormap_entries {
                        png_error(png_ptr, cstr(b"gray[16] color-map: too few entries\0"));
                    }

                    cmap_entries = make_gray_colormap(display) as c_uint;

                    if (*png_ptr).num_trans > 0 {
                        let back_alpha: c_uint;

                        if (output_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                            back_alpha = 0;
                        } else {
                            if back_r == back_g && back_g == back_b {
                                /* Background is gray; no special processing. */
                                let mut c: png_color_16 = core::mem::zeroed();
                                let mut gray: png_uint_32 = back_g;

                                if output_encoding == P_LINEAR {
                                    gray = PNG_sRGB_FROM_LINEAR(gray.wrapping_mul(255))
                                        as png_uint_32;

                                    /* Make sure the corresponding palette entry
                                     * matches.
                                     */
                                    png_create_colormap_entry(
                                        display, gray, back_g, back_g, back_g, 65535,
                                        P_LINEAR,
                                    );
                                }

                                /* The background passed to libpng must be the
                                 * sRGB value.
                                 */
                                c.index = 0; /*unused*/
                                c.gray = gray as png_uint_16;
                                c.red = gray as png_uint_16;
                                c.green = gray as png_uint_16;
                                c.blue = gray as png_uint_16;

                                png_set_background_fixed(
                                    png_ptr,
                                    &c,
                                    PNG_BACKGROUND_GAMMA_SCREEN,
                                    0, /*need_expand*/
                                    0, /*gamma: not used*/
                                );

                                output_processing = PNG_CMAP_NONE as c_uint;
                                break 'colortype;
                            }

                            back_alpha = if output_encoding == P_LINEAR { 65535 } else { 255 };
                        }

                        /* output_processing means that the libpng-processed row
                         * will be 8-bit GA.
                         */
                        expand_tRNS = 1;
                        output_processing = PNG_CMAP_TRANS as c_uint;
                        background_index = 254;

                        /* And set (overwrite) color-map entry 254 to the actual
                         * background color at full precision.
                         */
                        png_create_colormap_entry(
                            display, 254, back_r, back_g, back_b, back_alpha,
                            output_encoding,
                        );
                    } else {
                        output_processing = PNG_CMAP_NONE as c_uint;
                    }
                }
            }

            PNG_COLOR_TYPE_GRAY_ALPHA => {
                /* 8-bit or 16-bit PNG with two channels - gray and alpha. */
                data_encoding = P_sRGB;

                if (output_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                    if PNG_GA_COLORMAP_ENTRIES > (*image).colormap_entries {
                        png_error(png_ptr, cstr(b"gray+alpha color-map: too few entries\0"));
                    }

                    cmap_entries = make_ga_colormap(display) as c_uint;

                    background_index = PNG_CMAP_GA_BACKGROUND as c_uint;
                    output_processing = PNG_CMAP_GA as c_uint;
                } else {
                    /* alpha is removed */
                    if (output_format & PNG_FORMAT_FLAG_COLOR) == 0
                        || (back_r == back_g && back_g == back_b)
                    {
                        /* Background is gray; no special processing required. */
                        let mut c: png_color_16 = core::mem::zeroed();
                        let mut gray: png_uint_32 = back_g;

                        if PNG_GRAY_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, cstr(b"gray-alpha color-map: too few entries\0"));
                        }

                        cmap_entries = make_gray_colormap(display) as c_uint;

                        if output_encoding == P_LINEAR {
                            gray = PNG_sRGB_FROM_LINEAR(gray.wrapping_mul(255)) as png_uint_32;

                            /* And make sure the corresponding palette entry
                             * matches.
                             */
                            png_create_colormap_entry(
                                display, gray, back_g, back_g, back_g, 65535, P_LINEAR,
                            );
                        }

                        /* The background passed to libpng must be the sRGB
                         * value.
                         */
                        c.index = 0; /*unused*/
                        c.gray = gray as png_uint_16;
                        c.red = gray as png_uint_16;
                        c.green = gray as png_uint_16;
                        c.blue = gray as png_uint_16;

                        png_set_background_fixed(
                            png_ptr,
                            &c,
                            PNG_BACKGROUND_GAMMA_SCREEN,
                            0, /*need_expand*/
                            0, /*gamma: not used*/
                        );

                        output_processing = PNG_CMAP_NONE as c_uint;
                    } else {
                        let mut i: png_uint_32;
                        let mut a: png_uint_32;

                        /* Same as png_make_ga_colormap, above, except that the
                         * entries are all opaque.
                         */
                        if PNG_GA_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, cstr(b"ga-alpha color-map: too few entries\0"));
                        }

                        i = 0;
                        while i < 231 {
                            let gray: png_uint_32 = (i * 256 + 115) / 231;
                            png_create_colormap_entry(display, i, gray, gray, gray, 255, P_sRGB);
                            i += 1;
                        }

                        /* NOTE: this preserves the full precision of the
                         * application background color.
                         */
                        background_index = i;
                        png_create_colormap_entry(
                            display,
                            i,
                            back_r,
                            back_g,
                            back_b,
                            if output_encoding == P_LINEAR { 65535u32 } else { 255u32 },
                            output_encoding,
                        );
                        i += 1;

                        /* For non-opaque input composite on the sRGB
                         * background.
                         */
                        if output_encoding == P_sRGB {
                            /* else already linear */
                            back_r = png_sRGB_table[back_r as usize] as png_uint_32;
                            back_g = png_sRGB_table[back_g as usize] as png_uint_32;
                            back_b = png_sRGB_table[back_b as usize] as png_uint_32;
                        }

                        a = 1;
                        while a < 5 {
                            let mut g: c_uint;

                            /* PNG_sRGB_FROM_LINEAR expects a 16-bit linear value
                             * scaled by an 8-bit alpha value (0..255).
                             */
                            let alpha: png_uint_32 = 51 * a;
                            let back_rx: png_uint_32 = (255u32.wrapping_sub(alpha))
                                .wrapping_mul(back_r);
                            let back_gx: png_uint_32 = (255u32.wrapping_sub(alpha))
                                .wrapping_mul(back_g);
                            let back_bx: png_uint_32 = (255u32.wrapping_sub(alpha))
                                .wrapping_mul(back_b);

                            g = 0;
                            while g < 6 {
                                let gray: png_uint_32 =
                                    (png_sRGB_table[(g * 51) as usize] as png_uint_32)
                                        .wrapping_mul(alpha);

                                png_create_colormap_entry(
                                    display,
                                    i,
                                    PNG_sRGB_FROM_LINEAR(gray.wrapping_add(back_rx)) as png_uint_32,
                                    PNG_sRGB_FROM_LINEAR(gray.wrapping_add(back_gx)) as png_uint_32,
                                    PNG_sRGB_FROM_LINEAR(gray.wrapping_add(back_bx)) as png_uint_32,
                                    255,
                                    P_sRGB,
                                );
                                i += 1;
                                g += 1;
                            }
                            a += 1;
                        }

                        cmap_entries = i;
                        output_processing = PNG_CMAP_GA as c_uint;
                    }
                }
            }

            PNG_COLOR_TYPE_RGB | PNG_COLOR_TYPE_RGB_ALPHA => {
                /* Exclude the case where the output is gray. */
                if (output_format & PNG_FORMAT_FLAG_COLOR) == 0 {
                    /* The color-map will be grayscale. */
                    png_set_rgb_to_gray_fixed(png_ptr, PNG_ERROR_ACTION_NONE, -1, -1);
                    data_encoding = P_sRGB;

                    /* The output will now be one or two 8-bit gray or gray+alpha
                     * channels.
                     */
                    if ((*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
                        || (*png_ptr).num_trans > 0)
                        && (output_format & PNG_FORMAT_FLAG_ALPHA) != 0
                    {
                        /* Both input and output have an alpha channel. */
                        expand_tRNS = 1;

                        if PNG_GA_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, cstr(b"rgb[ga] color-map: too few entries\0"));
                        }

                        cmap_entries = make_ga_colormap(display) as c_uint;
                        background_index = PNG_CMAP_GA_BACKGROUND as c_uint;
                        output_processing = PNG_CMAP_GA as c_uint;
                    } else {
                        let gamma: png_fixed_point = png_resolve_file_gamma(png_ptr);

                        /* Either the input or the output has no alpha channel. */
                        if PNG_GRAY_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, cstr(b"rgb[gray] color-map: too few entries\0"));
                        }

                        /* Ideally this code would use libpng to do the gamma
                         * correction, but if an input alpha channel is to be
                         * removed we will hit the libpng bug.
                         */
                        if ((*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
                            || (*png_ptr).num_trans > 0)
                            && png_gamma_not_sRGB(gamma) != 0
                        {
                            cmap_entries = make_gray_file_colormap(display) as c_uint;
                            data_encoding = P_FILE;
                        } else {
                            cmap_entries = make_gray_colormap(display) as c_uint;
                        }

                        /* But if the input has alpha or transparency it must be
                         * removed.
                         */
                        if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
                            || (*png_ptr).num_trans > 0
                        {
                            let mut c: png_color_16 = core::mem::zeroed();
                            let mut gray: png_uint_32 = back_g;

                            /* We need to ensure that the application background
                             * exists in the colormap.
                             */
                            if data_encoding == P_FILE {
                                /* from the fixup above */
                                if output_encoding == P_sRGB {
                                    gray = png_sRGB_table[gray as usize] as png_uint_32;
                                    /* now P_LINEAR */
                                }

                                gray = PNG_DIV257(png_gamma_16bit_correct(gray, gamma)
                                    as png_uint_32);
                                /* now P_FILE */

                                /* And make sure the corresponding palette entry
                                 * contains exactly the required sRGB value.
                                 */
                                png_create_colormap_entry(
                                    display, gray, back_g, back_g, back_g, 0, /*unused*/
                                    output_encoding,
                                );
                            } else if output_encoding == P_LINEAR {
                                gray = PNG_sRGB_FROM_LINEAR(gray.wrapping_mul(255)) as png_uint_32;

                                /* And make sure the corresponding palette entry
                                 * matches.
                                 */
                                png_create_colormap_entry(
                                    display, gray, back_g, back_g, back_g, 0, /*unused*/
                                    P_LINEAR,
                                );
                            }

                            /* The background passed to libpng must be the output
                             * (normally sRGB) value.
                             */
                            c.index = 0; /*unused*/
                            c.gray = gray as png_uint_16;
                            c.red = gray as png_uint_16;
                            c.green = gray as png_uint_16;
                            c.blue = gray as png_uint_16;

                            /* NOTE: the following is apparently a bug in
                             * libpng.
                             */
                            expand_tRNS = 1;
                            png_set_background_fixed(
                                png_ptr,
                                &c,
                                PNG_BACKGROUND_GAMMA_SCREEN,
                                0, /*need_expand*/
                                0, /*gamma: not used*/
                            );
                        }

                        output_processing = PNG_CMAP_NONE as c_uint;
                    }
                } else {
                    /* output is color */
                    data_encoding = P_sRGB;

                    /* Is there any transparency or alpha? */
                    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
                        || (*png_ptr).num_trans > 0
                    {
                        /* Is there alpha in the output too? */
                        if (output_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                            let mut r: png_uint_32;

                            if PNG_RGB_COLORMAP_ENTRIES + 1 + 27 > (*image).colormap_entries {
                                png_error(png_ptr, cstr(b"rgb+alpha color-map: too few entries\0"));
                            }

                            cmap_entries = make_rgb_colormap(display) as c_uint;

                            /* Add a transparent entry. */
                            png_create_colormap_entry(
                                display, cmap_entries, 255, 255, 255, 0, P_sRGB,
                            );

                            /* This is stored as the background index for the
                             * processing algorithm.
                             */
                            background_index = cmap_entries;
                            cmap_entries += 1;

                            /* Add 27 r,g,b entries each with alpha 0.5. */
                            r = 0;
                            while r < 256 {
                                let mut g: png_uint_32 = 0;

                                while g < 256 {
                                    let mut b: png_uint_32 = 0;

                                    /* This generates components with the values
                                     * 0, 127 and 255
                                     */
                                    while b < 256 {
                                        png_create_colormap_entry(
                                            display, cmap_entries, r, g, b, 128, P_sRGB,
                                        );
                                        cmap_entries += 1;
                                        b = (b << 1) | 0x7f;
                                    }
                                    g = (g << 1) | 0x7f;
                                }
                                r = (r << 1) | 0x7f;
                            }

                            expand_tRNS = 1;
                            output_processing = PNG_CMAP_RGB_ALPHA as c_uint;
                        } else {
                            /* Alpha/transparency must be removed. */
                            let sample_size: c_uint = PNG_IMAGE_SAMPLE_SIZE(output_format);
                            let r: png_uint_32;
                            let g: png_uint_32;
                            let b: png_uint_32; /* sRGB background */

                            if PNG_RGB_COLORMAP_ENTRIES + 1 + 27 > (*image).colormap_entries {
                                png_error(png_ptr, cstr(b"rgb-alpha color-map: too few entries\0"));
                            }

                            cmap_entries = make_rgb_colormap(display) as c_uint;

                            png_create_colormap_entry(
                                display, cmap_entries, back_r, back_g, back_b, 0, /*unused*/
                                output_encoding,
                            );

                            if output_encoding == P_LINEAR {
                                r = PNG_sRGB_FROM_LINEAR(back_r.wrapping_mul(255)) as png_uint_32;
                                g = PNG_sRGB_FROM_LINEAR(back_g.wrapping_mul(255)) as png_uint_32;
                                b = PNG_sRGB_FROM_LINEAR(back_b.wrapping_mul(255)) as png_uint_32;
                            } else {
                                r = back_r;
                                g = back_g;
                                b = back_b;
                            }

                            /* Compare the newly-created color-map entry with the
                             * one the PNG_CMAP_RGB algorithm will use.
                             */
                            if memcmp(
                                ((*display).colormap as png_const_bytep)
                                    .add((sample_size * cmap_entries) as usize)
                                    as png_const_voidp,
                                ((*display).colormap as png_const_bytep).add(
                                    (sample_size * PNG_RGB_INDEX(r, g, b) as c_uint) as usize,
                                ) as png_const_voidp,
                                sample_size as usize,
                            ) != 0
                            {
                                /* The background color must be added. */
                                background_index = cmap_entries;
                                cmap_entries += 1;

                                /* Add 27 r,g,b entries created by composing with
                                 * the background at alpha 0.5.
                                 */
                                let mut rr: png_uint_32 = 0;
                                while rr < 256 {
                                    let mut gg: png_uint_32 = 0;
                                    while gg < 256 {
                                        /* This generates components with the
                                         * values 0, 127 and 255
                                         */
                                        let mut bb: png_uint_32 = 0;
                                        while bb < 256 {
                                            png_create_colormap_entry(
                                                display,
                                                cmap_entries,
                                                png_colormap_compose(
                                                    display, rr, P_sRGB, 128, back_r,
                                                    output_encoding,
                                                ),
                                                png_colormap_compose(
                                                    display, gg, P_sRGB, 128, back_g,
                                                    output_encoding,
                                                ),
                                                png_colormap_compose(
                                                    display, bb, P_sRGB, 128, back_b,
                                                    output_encoding,
                                                ),
                                                0, /*unused*/
                                                output_encoding,
                                            );
                                            cmap_entries += 1;
                                            bb = (bb << 1) | 0x7f;
                                        }
                                        gg = (gg << 1) | 0x7f;
                                    }
                                    rr = (rr << 1) | 0x7f;
                                }

                                expand_tRNS = 1;
                                output_processing = PNG_CMAP_RGB_ALPHA as c_uint;
                            } else {
                                /* background color is in the standard color-map */
                                let mut c: png_color_16 = core::mem::zeroed();

                                c.index = 0; /*unused*/
                                c.red = back_r as png_uint_16;
                                c.gray = back_g as png_uint_16;
                                c.green = back_g as png_uint_16;
                                c.blue = back_b as png_uint_16;

                                png_set_background_fixed(
                                    png_ptr,
                                    &c,
                                    PNG_BACKGROUND_GAMMA_SCREEN,
                                    0, /*need_expand*/
                                    0, /*gamma: not used*/
                                );

                                output_processing = PNG_CMAP_RGB as c_uint;
                            }
                        }
                    } else {
                        /* no alpha or transparency in the input */
                        if PNG_RGB_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, cstr(b"rgb color-map: too few entries\0"));
                        }

                        cmap_entries = make_rgb_colormap(display) as c_uint;
                        output_processing = PNG_CMAP_RGB as c_uint;
                    }
                }
            }

            PNG_COLOR_TYPE_PALETTE => {
                /* It's already got a color-map. */
                let mut num_trans: c_uint = (*png_ptr).num_trans as c_uint;
                let trans: png_const_bytep = if num_trans > 0 {
                    (*png_ptr).trans_alpha
                } else {
                    core::ptr::null()
                };
                let colormap: png_const_colorp = (*png_ptr).palette;
                let do_background: c_int =
                    (!trans.is_null() && (output_format & PNG_FORMAT_FLAG_ALPHA) == 0) as c_int;
                let mut i: c_uint;

                /* Just in case: */
                if trans.is_null() {
                    num_trans = 0;
                }

                output_processing = PNG_CMAP_NONE as c_uint;
                data_encoding = P_FILE; /* Don't change from color-map indices */
                cmap_entries = (*png_ptr).num_palette as c_uint;
                if cmap_entries > 256 {
                    cmap_entries = 256;
                }

                if cmap_entries > (*image).colormap_entries {
                    png_error(png_ptr, cstr(b"palette color-map: too few entries\0"));
                }

                i = 0;
                while i < cmap_entries {
                    if do_background != 0
                        && i < num_trans
                        && *trans.add(i as usize) < 255
                    {
                        if *trans.add(i as usize) == 0 {
                            png_create_colormap_entry(
                                display, i, back_r, back_g, back_b, 0, output_encoding,
                            );
                        } else {
                            /* Must compose the PNG file color in the color-map
                             * entry on the sRGB color in 'back'.
                             */
                            png_create_colormap_entry(
                                display,
                                i,
                                png_colormap_compose(
                                    display,
                                    (*colormap.add(i as usize)).red as png_uint_32,
                                    P_FILE,
                                    *trans.add(i as usize) as png_uint_32,
                                    back_r,
                                    output_encoding,
                                ),
                                png_colormap_compose(
                                    display,
                                    (*colormap.add(i as usize)).green as png_uint_32,
                                    P_FILE,
                                    *trans.add(i as usize) as png_uint_32,
                                    back_g,
                                    output_encoding,
                                ),
                                png_colormap_compose(
                                    display,
                                    (*colormap.add(i as usize)).blue as png_uint_32,
                                    P_FILE,
                                    *trans.add(i as usize) as png_uint_32,
                                    back_b,
                                    output_encoding,
                                ),
                                if output_encoding == P_LINEAR {
                                    (*trans.add(i as usize) as png_uint_32).wrapping_mul(257)
                                } else {
                                    *trans.add(i as usize) as png_uint_32
                                },
                                output_encoding,
                            );
                        }
                    } else {
                        png_create_colormap_entry(
                            display,
                            i,
                            (*colormap.add(i as usize)).red as png_uint_32,
                            (*colormap.add(i as usize)).green as png_uint_32,
                            (*colormap.add(i as usize)).blue as png_uint_32,
                            if i < num_trans {
                                *trans.add(i as usize) as png_uint_32
                            } else {
                                255u32
                            },
                            P_FILE, /*8-bit*/
                        );
                    }
                    i += 1;
                }

                /* The PNG data may have indices packed in fewer than 8 bits. */
                if (*png_ptr).bit_depth < 8 {
                    png_set_packing(png_ptr);
                }
            }

            _ => {
                png_error(png_ptr, cstr(b"invalid PNG color type\0"));
                /*NOT REACHED*/
            }
        }
    } /* 'colortype */

    /* Now deal with the output processing */
    if expand_tRNS != 0
        && (*png_ptr).num_trans > 0
        && ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) == 0
    {
        png_set_tRNS_to_alpha(png_ptr);
    }

    match data_encoding {
        P_sRGB => {
            /* Change to 8-bit sRGB */
            png_set_alpha_mode_fixed(png_ptr, PNG_ALPHA_PNG, PNG_GAMMA_sRGB);
            /* FALLTHROUGH */
            if (*png_ptr).bit_depth > 8 {
                png_set_scale_16(png_ptr);
            }
        }

        P_FILE => {
            if (*png_ptr).bit_depth > 8 {
                png_set_scale_16(png_ptr);
            }
        }

        _ => {
            png_error(png_ptr, cstr(b"bad data option (internal error)\0"));
        }
    }

    if cmap_entries > 256 || cmap_entries > (*image).colormap_entries {
        png_error(png_ptr, cstr(b"color map overflow (BAD internal error)\0"));
    }

    (*image).colormap_entries = cmap_entries;

    /* Double check using the recorded background index */
    let mut bad_background = false;
    match output_processing as c_int {
        PNG_CMAP_NONE => {
            if background_index != PNG_CMAP_NONE_BACKGROUND as c_uint {
                bad_background = true;
            }
        }

        PNG_CMAP_GA => {
            if background_index != PNG_CMAP_GA_BACKGROUND as c_uint {
                bad_background = true;
            }
        }

        PNG_CMAP_TRANS => {
            if background_index >= cmap_entries
                || background_index != PNG_CMAP_TRANS_BACKGROUND as c_uint
            {
                bad_background = true;
            }
        }

        PNG_CMAP_RGB => {
            if background_index != PNG_CMAP_RGB_BACKGROUND as c_uint {
                bad_background = true;
            }
        }

        PNG_CMAP_RGB_ALPHA => {
            if background_index != PNG_CMAP_RGB_ALPHA_BACKGROUND as c_uint {
                bad_background = true;
            }
        }

        _ => {
            png_error(png_ptr, cstr(b"bad processing option (internal error)\0"));
        }
    }

    if bad_background {
        png_error(png_ptr, cstr(b"bad background index (internal error)\0"));
    }

    (*display).colormap_processing = output_processing as c_int;

    1 /*ok*/
}
