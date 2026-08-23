use crate::*;

/* Utility functions to make particular color-maps */
unsafe fn set_file_encoding(display: *mut png_image_read_control) {
    let png_ptr: png_structrp = (*(*(*display).image).opaque).png_ptr;
    let g: png_fixed_point = png_resolve_file_gamma(png_ptr);

    /* PNGv3: the result may be 0 however the 'default_gamma' should have been
     * set before this is called so zero is an error:
     */
    if g == 0 {
        png_error(png_ptr, cstr!("internal: default gamma not set"));
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

unsafe fn decode_gamma(
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
            value = png_gamma_16bit_correct(value * 257, (*display).gamma_to_linear)
                as png_uint_32;
        }

        P_sRGB => {
            value = png_sRGB_table[value as usize] as png_uint_32;
        }

        P_LINEAR => {}

        P_LINEAR8 => {
            value *= 257;
        }

        _ => {
            png_error(
                (*(*(*display).image).opaque).png_ptr,
                cstr!("unexpected encoding (internal error)"),
            );
        }
    }

    value
}

unsafe fn png_colormap_compose(
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
    } else {
        /* P_sRGB */
        f = PNG_sRGB_FROM_LINEAR(f) as png_uint_32;
    }

    f
}

/* NOTE: P_LINEAR values to this routine must be 16-bit, but P_FILE values must
 * be 8-bit.
 */
unsafe fn png_create_colormap_entry(
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
            cstr!("color-map index out of range"),
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
            red = y;
            blue = y;
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
            cstr!("bad encoding (internal error)"),
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

            entry = entry.offset((ip * PNG_IMAGE_SAMPLE_CHANNELS((*image).format)) as isize);

            /* The linear 16-bit values must be pre-multiplied by the alpha channel
             * value, if less than 65535 (this is, effectively, composite on black
             * if the alpha channel is removed.)
             */
            let channels: png_uint_32 = PNG_IMAGE_SAMPLE_CHANNELS((*image).format);
            match channels {
                4 | 3 => {
                    if channels == 4 {
                        *entry.offset(if afirst != 0 { 0 } else { 3 }) = alpha as png_uint_16;
                        /* FALLTHROUGH */
                    }

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
                    *entry.offset((afirst + (2 ^ bgr)) as isize) = blue as png_uint_16;
                    *entry.offset((afirst + 1) as isize) = green as png_uint_16;
                    *entry.offset((afirst + bgr) as isize) = red as png_uint_16;
                }

                2 | 1 => {
                    if channels == 2 {
                        *entry.offset((1 ^ afirst) as isize) = alpha as png_uint_16;
                        /* FALLTHROUGH */
                    }

                    /* case 1: */
                    if alpha < 65535 {
                        if alpha > 0 {
                            green = (green * alpha + 32767u32) / 65535u32;
                        } else {
                            green = 0;
                        }
                    }
                    *entry.offset(afirst as isize) = green as png_uint_16;
                }

                _ => {}
            }
        } else {
            /* output encoding is P_sRGB */
            let mut entry: png_bytep = (*display).colormap as png_bytep;

            entry = entry.offset((ip * PNG_IMAGE_SAMPLE_CHANNELS((*image).format)) as isize);

            let channels: png_uint_32 = PNG_IMAGE_SAMPLE_CHANNELS((*image).format);
            match channels {
                4 | 3 => {
                    if channels == 4 {
                        *entry.offset(if afirst != 0 { 0 } else { 3 }) = alpha as png_byte;
                        /* FALLTHROUGH */
                    }
                    /* case 3: */
                    *entry.offset((afirst + (2 ^ bgr)) as isize) = blue as png_byte;
                    *entry.offset((afirst + 1) as isize) = green as png_byte;
                    *entry.offset((afirst + bgr) as isize) = red as png_byte;
                }

                2 | 1 => {
                    if channels == 2 {
                        *entry.offset((1 ^ afirst) as isize) = alpha as png_byte;
                        /* FALLTHROUGH */
                    }
                    /* case 1: */
                    *entry.offset(afirst as isize) = green as png_byte;
                }

                _ => {}
            }
        }
    }
}

unsafe fn make_gray_file_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i: c_uint;

    i = 0;
    while i < 256 {
        png_create_colormap_entry(display, i, i, i, i, 255, P_FILE);
        i += 1;
    }

    i as c_int
}

unsafe fn make_gray_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i: c_uint;

    i = 0;
    while i < 256 {
        png_create_colormap_entry(display, i, i, i, i, 255, P_sRGB);
        i += 1;
    }

    i as c_int
}
const PNG_GRAY_COLORMAP_ENTRIES: c_uint = 256;

unsafe fn make_ga_colormap(display: *mut png_image_read_control) -> c_int {
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

const PNG_GA_COLORMAP_ENTRIES: c_uint = 256;

unsafe fn make_rgb_colormap(display: *mut png_image_read_control) -> c_int {
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

const PNG_RGB_COLORMAP_ENTRIES: c_uint = 216;

/* Return a palette index to the above palette given three 8-bit sRGB values.
 *
 * NOTE: the PNG_DIV51(v8) macro (`((v8) * 5 + 130) >> 8`) is defined outside
 * this chunk; the expression is written out here so that this file has no
 * dependency on it.
 */
#[inline]
fn PNG_RGB_INDEX(r: png_uint_32, g: png_uint_32, b: png_uint_32) -> png_byte {
    (6 * (6 * ((r * 5 + 130) >> 8) + ((g * 5 + 130) >> 8)) + ((b * 5 + 130) >> 8)) as png_byte
}
