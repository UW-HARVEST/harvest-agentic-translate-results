/* pngread.c lines 1528..1959 */

/* Utility functions to make particular color-maps */
unsafe fn set_file_encoding(display: *mut png_image_read_control) {
    let png_ptr: png_structrp = (*(*(*display).image).opaque).png_ptr;
    let g: png_fixed_point = png_resolve_file_gamma(png_ptr);

    /* PNGv3: the result may be 0 however the 'default_gamma' should have been
     * set before this is called so zero is an error:
     */
    if g == 0 {
        png_error(
            png_ptr,
            b"internal: default gamma not set\0".as_ptr() as png_const_charp,
        );
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
) -> png_uint_32 {
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
            value = png_gamma_16bit_correct(value.wrapping_mul(257), (*display).gamma_to_linear)
                as png_uint_32;
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
                b"unexpected encoding (internal error)\0".as_ptr() as png_const_charp,
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
    f = f
        .wrapping_mul(alpha)
        .wrapping_add(b.wrapping_mul(255u32.wrapping_sub(alpha)));

    if encoding == P_LINEAR {
        /* Scale to 65535; divide by 255, approximately (in fact this is extremely
         * accurate, it divides by 255.00000005937181414556, with no overflow.)
         */
        f = f.wrapping_mul(257); /* Now scaled by 65535 */
        f = f.wrapping_add(f >> 16);
        f = (f.wrapping_add(32768)) >> 16;
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
            b"color-map index out of range\0".as_ptr() as png_const_charp,
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
            let mut y: png_uint_32 = (6968 as png_uint_32)
                .wrapping_mul(red)
                .wrapping_add((23434 as png_uint_32).wrapping_mul(green))
                .wrapping_add((2366 as png_uint_32).wrapping_mul(blue));

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

            green = y;
            red = green;
            blue = red;
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
            b"bad encoding (internal error)\0".as_ptr() as png_const_charp,
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
                    *entry.offset(if afirst != 0 { 0 } else { 3 }) = alpha as png_uint_16;
                    /* FALLTHROUGH */

                    /* case 3: */
                    if alpha < 65535 {
                        if alpha > 0 {
                            blue = (blue.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                            green = (green.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                            red = (red.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                        } else {
                            blue = 0;
                            green = blue;
                            red = green;
                        }
                    }
                    *entry.offset((afirst + (2 ^ bgr)) as isize) = blue as png_uint_16;
                    *entry.offset((afirst + 1) as isize) = green as png_uint_16;
                    *entry.offset((afirst + bgr) as isize) = red as png_uint_16;
                }

                3 => {
                    if alpha < 65535 {
                        if alpha > 0 {
                            blue = (blue.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                            green = (green.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                            red = (red.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                        } else {
                            blue = 0;
                            green = blue;
                            red = green;
                        }
                    }
                    *entry.offset((afirst + (2 ^ bgr)) as isize) = blue as png_uint_16;
                    *entry.offset((afirst + 1) as isize) = green as png_uint_16;
                    *entry.offset((afirst + bgr) as isize) = red as png_uint_16;
                }

                2 => {
                    *entry.offset((1 ^ afirst) as isize) = alpha as png_uint_16;
                    /* FALLTHROUGH */

                    /* case 1: */
                    if alpha < 65535 {
                        if alpha > 0 {
                            green = (green.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
                        } else {
                            green = 0;
                        }
                    }
                    *entry.offset(afirst as isize) = green as png_uint_16;
                }

                1 => {
                    if alpha < 65535 {
                        if alpha > 0 {
                            green = (green.wrapping_mul(alpha).wrapping_add(32767u32)) / 65535u32;
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

            entry = entry.add((ip * PNG_IMAGE_SAMPLE_CHANNELS((*image).format)) as usize);

            match PNG_IMAGE_SAMPLE_CHANNELS((*image).format) {
                4 => {
                    *entry.offset(if afirst != 0 { 0 } else { 3 }) = alpha as png_byte;
                    /* FALLTHROUGH */

                    /* case 3: */
                    *entry.offset((afirst + (2 ^ bgr)) as isize) = blue as png_byte;
                    *entry.offset((afirst + 1) as isize) = green as png_byte;
                    *entry.offset((afirst + bgr) as isize) = red as png_byte;
                }

                3 => {
                    *entry.offset((afirst + (2 ^ bgr)) as isize) = blue as png_byte;
                    *entry.offset((afirst + 1) as isize) = green as png_byte;
                    *entry.offset((afirst + bgr) as isize) = red as png_byte;
                }

                2 => {
                    *entry.offset((1 ^ afirst) as isize) = alpha as png_byte;
                    /* FALLTHROUGH */

                    /* case 1: */
                    *entry.offset(afirst as isize) = green as png_byte;
                }

                1 => {
                    *entry.offset(afirst as isize) = green as png_byte;
                }

                _ => {}
            }
        }
    }
}

unsafe fn make_gray_file_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i: c_uint = 0;

    while i < 256 {
        png_create_colormap_entry(display, i, i, i, i, 255, P_FILE);
        i += 1;
    }

    i as c_int
}

unsafe fn make_gray_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i: c_uint = 0;

    while i < 256 {
        png_create_colormap_entry(display, i, i, i, i, 255, P_sRGB);
        i += 1;
    }

    i as c_int
}

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
        let gray: c_uint = (i.wrapping_mul(256).wrapping_add(115)) / 231;
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
                g.wrapping_mul(51),
                g.wrapping_mul(51),
                g.wrapping_mul(51),
                a.wrapping_mul(51),
                P_sRGB,
            );
            i += 1;
            g += 1;
        }

        a += 1;
    }

    i as c_int
}

unsafe fn make_rgb_colormap(display: *mut png_image_read_control) -> c_int {
    let mut i: c_uint;
    let mut r: c_uint;

    /* Build a 6x6x6 opaque RGB cube */
    r = 0;
    i = r;
    while r < 6 {
        let mut g: c_uint = 0;

        while g < 6 {
            let mut b: c_uint = 0;

            while b < 6 {
                png_create_colormap_entry(
                    display,
                    i,
                    r.wrapping_mul(51),
                    g.wrapping_mul(51),
                    b.wrapping_mul(51),
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
