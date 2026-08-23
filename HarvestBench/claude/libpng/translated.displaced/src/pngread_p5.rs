use crate::*;

unsafe extern "C" fn png_image_read_colormap(argument: png_voidp) -> c_int {
    let display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let image: png_imagep = (*display).image;

    let png_ptr: png_structrp = (*(*image).opaque).png_ptr;
    let output_format: png_uint_32 = (*image).format;
    let output_encoding: c_int = if (output_format & PNG_FORMAT_FLAG_LINEAR) != 0 {
        P_LINEAR as c_int
    } else {
        P_sRGB as c_int
    };

    let mut cmap_entries: c_uint = 0;
    let mut output_processing: c_uint = 0; /* Output processing option */
    let mut data_encoding: c_uint = P_NOTSET as c_uint; /* Encoding libpng must produce */

    /* Background information; the background color and the index of this color
     * in the color-map if it exists (else 256).
     */
    let mut background_index: c_uint = 256;
    let mut back_r: png_uint_32 = 0;
    let mut back_g: png_uint_32 = 0;
    let mut back_b: png_uint_32 = 0;

    /* Flags to accumulate things that need to be done to the input. */
    let mut expand_tRNS: c_int = 0;

    /* Exclude the NYI feature of compositing onto a color-mapped buffer; it is
     * very difficult to do, the results look awful, and it is difficult to see
     * what possible use it is because the application can't control the
     * color-map.
     */
    if (((*png_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) != 0
        || (*png_ptr).num_trans > 0) /* alpha in input */
        && (output_format & PNG_FORMAT_FLAG_ALPHA) == 0
    /* no alpha in output */
    {
        if output_encoding == P_LINEAR as c_int
        /* compose on black */
        {
            back_r = 0;
            back_g = 0;
            back_b = 0;
        } else if (*display).background.is_null()
        /* no way to remove it */
        {
            png_error(
                png_ptr,
                cstr!("background color must be supplied to remove alpha/transparency"),
            );
        }
        /* Get a copy of the background color (this avoids repeating the checks
         * below.)  The encoding is 8-bit sRGB or 16-bit linear, depending on the
         * output format.
         */
        else {
            back_g = (*(*display).background).green as png_uint_32;
            if (output_format & PNG_FORMAT_FLAG_COLOR) != 0 {
                back_r = (*(*display).background).red as png_uint_32;
                back_b = (*(*display).background).blue as png_uint_32;
            } else {
                back_r = back_g;
                back_b = back_g;
            }
        }
    } else if output_encoding == P_LINEAR as c_int {
        back_g = 65535;
        back_r = 65535;
        back_b = 65535;
    } else {
        back_g = 255;
        back_r = 255;
        back_b = 255;
    }

    /* Default the input file gamma if required - this is necessary because
     * libpng assumes that if no gamma information is present the data is in the
     * output format, but the simplified API deduces the gamma from the input
     * format.  The 'default' gamma value is also set by png_set_alpha_mode, but
     * this is happening before any such call, so:
     *
     * TODO: should be an internal API and all this code should be copied into a
     * single common gamma+colorspace file.
     */
    if (*png_ptr).bit_depth == 16 && ((*image).flags & PNG_IMAGE_FLAG_16BIT_sRGB) == 0 {
        (*png_ptr).default_gamma = PNG_GAMMA_LINEAR;
    } else {
        (*png_ptr).default_gamma = PNG_GAMMA_sRGB_INVERSE;
    }

    /* Decide what to do based on the PNG color type of the input data.  The
     * utility function png_create_colormap_entry deals with most aspects of the
     * output transformations; this code works out how to produce bytes of
     * color-map entries from the original format.
     */
    'color_type: loop {
        match (*png_ptr).color_type as c_int {
            PNG_COLOR_TYPE_GRAY => {
                if (*png_ptr).bit_depth <= 8 {
                    /* There at most 256 colors in the output, regardless of
                     * transparency.
                     */
                    let step: c_uint;
                    let mut i: c_uint;
                    let mut val: c_uint;
                    let mut trans: c_uint = 256; /*ignore*/
                    let mut back_alpha: c_uint = 0;

                    cmap_entries = 1u32 << (*png_ptr).bit_depth as c_uint;
                    if cmap_entries > (*image).colormap_entries {
                        png_error(png_ptr, cstr!("gray[8] color-map: too few entries"));
                    }

                    step = 255 / (cmap_entries - 1);
                    output_processing = PNG_CMAP_NONE as c_uint;

                    /* If there is a tRNS chunk then this either selects a transparent
                     * value or, if the output has no alpha, the background color.
                     */
                    if (*png_ptr).num_trans > 0 {
                        trans = (*png_ptr).trans_color.gray as c_uint;

                        if (output_format & PNG_FORMAT_FLAG_ALPHA) == 0 {
                            back_alpha = if output_encoding == P_LINEAR as c_int {
                                65535
                            } else {
                                255
                            };
                        }
                    }

                    /* png_create_colormap_entry just takes an RGBA and writes the
                     * corresponding color-map entry using the format from 'image',
                     * including the required conversion to sRGB or linear as
                     * appropriate.  The input values are always either sRGB (if the
                     * gamma correction flag is 0) or 0..255 scaled file encoded values
                     * (if the function must gamma correct them).
                     */
                    i = 0;
                    val = 0;
                    while i < cmap_entries {
                        /* 'i' is a file value.  While this will result in duplicated
                         * entries for 8-bit non-sRGB encoded files it is necessary to
                         * have non-gamma corrected values to do tRNS handling.
                         */
                        if i != trans {
                            png_create_colormap_entry(
                                display,
                                i,
                                val,
                                val,
                                val,
                                255,
                                P_FILE as c_int, /*8-bit with file gamma*/
                            );
                        }
                        /* Else this entry is transparent.  The colors don't matter if
                         * there is an alpha channel (back_alpha == 0), but it does no
                         * harm to pass them in; the values are not set above so this
                         * passes in white.
                         *
                         * NOTE: this preserves the full precision of the application
                         * supplied background color when it is used.
                         */
                        else {
                            png_create_colormap_entry(
                                display,
                                i,
                                back_r,
                                back_g,
                                back_b,
                                back_alpha,
                                output_encoding,
                            );
                        }

                        i += 1;
                        val += step;
                    }

                    /* We need libpng to preserve the original encoding. */
                    data_encoding = P_FILE as c_uint;

                    /* The rows from libpng, while technically gray values, are now also
                     * color-map indices; however, they may need to be expanded to 1
                     * byte per pixel.  This is what png_set_packing does (i.e., it
                     * unpacks the bit values into bytes.)
                     */
                    if (*png_ptr).bit_depth < 8 {
                        png_set_packing(png_ptr);
                    }
                } else
                /* bit depth is 16 */
                {
                    /* The 16-bit input values can be converted directly to 8-bit gamma
                     * encoded values; however, if a tRNS chunk is present 257 color-map
                     * entries are required.  This means that the extra entry requires
                     * special processing; add an alpha channel, sacrifice gray level
                     * 254 and convert transparent (alpha==0) entries to that.
                     *
                     * Use libpng to chop the data to 8 bits.  Convert it to sRGB at the
                     * same time to minimize quality loss.  If a tRNS chunk is present
                     * this means libpng must handle it too; otherwise it is impossible
                     * to do the exact match on the 16-bit value.
                     *
                     * If the output has no alpha channel *and* the background color is
                     * gray then it is possible to let libpng handle the substitution by
                     * ensuring that the corresponding gray level matches the background
                     * color exactly.
                     */
                    data_encoding = P_sRGB as c_uint;

                    if PNG_GRAY_COLORMAP_ENTRIES > (*image).colormap_entries {
                        png_error(png_ptr, cstr!("gray[16] color-map: too few entries"));
                    }

                    cmap_entries = make_gray_colormap(display) as c_uint;

                    if (*png_ptr).num_trans > 0 {
                        let back_alpha: c_uint;

                        if (output_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                            back_alpha = 0;
                        } else {
                            if back_r == back_g && back_g == back_b {
                                /* Background is gray; no special processing will be
                                 * required.
                                 */
                                let mut c: png_color_16 = core::mem::zeroed();
                                let mut gray: png_uint_32 = back_g;

                                if output_encoding == P_LINEAR as c_int {
                                    gray = PNG_sRGB_FROM_LINEAR(gray * 255) as png_uint_32;

                                    /* And make sure the corresponding palette entry
                                     * matches.
                                     */
                                    png_create_colormap_entry(
                                        display,
                                        gray,
                                        back_g,
                                        back_g,
                                        back_g,
                                        65535,
                                        P_LINEAR as c_int,
                                    );
                                }

                                /* The background passed to libpng, however, must be the
                                 * sRGB value.
                                 */
                                c.index = 0; /*unused*/
                                c.blue = gray as png_uint_16;
                                c.green = c.blue;
                                c.red = c.green;
                                c.gray = c.red;

                                /* NOTE: does this work without expanding tRNS to alpha?
                                 * It should be the color->gray case below apparently
                                 * doesn't.
                                 */
                                png_set_background_fixed(
                                    png_ptr,
                                    &c as png_const_color_16p,
                                    PNG_BACKGROUND_GAMMA_SCREEN,
                                    0, /*need_expand*/
                                    0, /*gamma: not used*/
                                );

                                output_processing = PNG_CMAP_NONE as c_uint;
                                break 'color_type;
                            }

                            back_alpha = if output_encoding == P_LINEAR as c_int {
                                65535
                            } else {
                                255
                            };
                        }

                        /* output_processing means that the libpng-processed row will be
                         * 8-bit GA and it has to be processing to single byte color-map
                         * values.  Entry 254 is replaced by either a completely
                         * transparent entry or by the background color at full
                         * precision (and the background color is not a simple gray
                         * level in this case.)
                         */
                        expand_tRNS = 1;
                        output_processing = PNG_CMAP_TRANS as c_uint;
                        background_index = 254;

                        /* And set (overwrite) color-map entry 254 to the actual
                         * background color at full precision.
                         */
                        png_create_colormap_entry(
                            display,
                            254,
                            back_r,
                            back_g,
                            back_b,
                            back_alpha,
                            output_encoding,
                        );
                    } else {
                        output_processing = PNG_CMAP_NONE as c_uint;
                    }
                }
            }

            PNG_COLOR_TYPE_GRAY_ALPHA => {
                /* 8-bit or 16-bit PNG with two channels - gray and alpha.  A minimum
                 * of 65536 combinations.  If, however, the alpha channel is to be
                 * removed there are only 256 possibilities if the background is gray.
                 * (Otherwise there is a subset of the 65536 possibilities defined by
                 * the triangle between black, white and the background color.)
                 *
                 * Reduce 16-bit files to 8-bit and sRGB encode the result.  No need to
                 * worry about tRNS matching - tRNS is ignored if there is an alpha
                 * channel.
                 */
                data_encoding = P_sRGB as c_uint;

                if (output_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                    if PNG_GA_COLORMAP_ENTRIES > (*image).colormap_entries {
                        png_error(png_ptr, cstr!("gray+alpha color-map: too few entries"));
                    }

                    cmap_entries = make_ga_colormap(display) as c_uint;

                    background_index = PNG_CMAP_GA_BACKGROUND as c_uint;
                    output_processing = PNG_CMAP_GA as c_uint;
                } else
                /* alpha is removed */
                {
                    /* Alpha must be removed as the PNG data is processed when the
                     * background is a color because the G and A channels are
                     * independent and the vector addition (non-parallel vectors) is a
                     * 2-D problem.
                     *
                     * This can be reduced to the same algorithm as above by making a
                     * colormap containing gray levels (for the opaque grays), a
                     * background entry (for a transparent pixel) and a set of four six
                     * level color values, one set for each intermediate alpha value.
                     * See the comments in make_ga_colormap for how this works in the
                     * per-pixel processing.
                     *
                     * If the background is gray, however, we only need a 256 entry gray
                     * level color map.  It is sufficient to make the entry generated
                     * for the background color be exactly the color specified.
                     */
                    if (output_format & PNG_FORMAT_FLAG_COLOR) == 0
                        || (back_r == back_g && back_g == back_b)
                    {
                        /* Background is gray; no special processing will be required. */
                        let mut c: png_color_16 = core::mem::zeroed();
                        let mut gray: png_uint_32 = back_g;

                        if PNG_GRAY_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, cstr!("gray-alpha color-map: too few entries"));
                        }

                        cmap_entries = make_gray_colormap(display) as c_uint;

                        if output_encoding == P_LINEAR as c_int {
                            gray = PNG_sRGB_FROM_LINEAR(gray * 255) as png_uint_32;

                            /* And make sure the corresponding palette entry matches. */
                            png_create_colormap_entry(
                                display,
                                gray,
                                back_g,
                                back_g,
                                back_g,
                                65535,
                                P_LINEAR as c_int,
                            );
                        }

                        /* The background passed to libpng, however, must be the sRGB
                         * value.
                         */
                        c.index = 0; /*unused*/
                        c.blue = gray as png_uint_16;
                        c.green = c.blue;
                        c.red = c.green;
                        c.gray = c.red;

                        png_set_background_fixed(
                            png_ptr,
                            &c as png_const_color_16p,
                            PNG_BACKGROUND_GAMMA_SCREEN,
                            0, /*need_expand*/
                            0, /*gamma: not used*/
                        );

                        output_processing = PNG_CMAP_NONE as c_uint;
                    } else {
                        let mut i: png_uint_32;
                        let mut a: png_uint_32;

                        /* This is the same as png_make_ga_colormap, above, except that
                         * the entries are all opaque.
                         */
                        if PNG_GA_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, cstr!("ga-alpha color-map: too few entries"));
                        }

                        i = 0;
                        while i < 231 {
                            let gray: png_uint_32 = (i * 256 + 115) / 231;
                            png_create_colormap_entry(
                                display,
                                i,
                                gray,
                                gray,
                                gray,
                                255,
                                P_sRGB as c_int,
                            );
                            i += 1;
                        }

                        /* NOTE: this preserves the full precision of the application
                         * background color.
                         */
                        background_index = i;
                        png_create_colormap_entry(
                            display,
                            i,
                            back_r,
                            back_g,
                            back_b,
                            if output_encoding == P_LINEAR as c_int {
                                65535u32
                            } else {
                                255u32
                            },
                            output_encoding,
                        );
                        i += 1;

                        /* For non-opaque input composite on the sRGB background - this
                         * requires inverting the encoding for each component.  The input
                         * is still converted to the sRGB encoding because this is a
                         * reasonable approximate to the logarithmic curve of human
                         * visual sensitivity, at least over the narrow range which PNG
                         * represents.  Consequently 'G' is always sRGB encoded, while
                         * 'A' is linear.  We need the linear background colors.
                         */
                        if output_encoding == P_sRGB as c_int
                        /* else already linear */
                        {
                            /* This may produce a value not exactly matching the
                             * background, but that's ok because these numbers are only
                             * used when alpha != 0
                             */
                            back_r = png_sRGB_table[back_r as usize] as png_uint_32;
                            back_g = png_sRGB_table[back_g as usize] as png_uint_32;
                            back_b = png_sRGB_table[back_b as usize] as png_uint_32;
                        }

                        a = 1;
                        while a < 5 {
                            let mut g: c_uint;

                            /* PNG_sRGB_FROM_LINEAR expects a 16-bit linear value scaled
                             * by an 8-bit alpha value (0..255).
                             */
                            let alpha: png_uint_32 = 51 * a;
                            let back_rx: png_uint_32 = (255 - alpha) * back_r;
                            let back_gx: png_uint_32 = (255 - alpha) * back_g;
                            let back_bx: png_uint_32 = (255 - alpha) * back_b;

                            g = 0;
                            while g < 6 {
                                let gray: png_uint_32 =
                                    png_sRGB_table[(g * 51) as usize] as png_uint_32 * alpha;

                                png_create_colormap_entry(
                                    display,
                                    i,
                                    PNG_sRGB_FROM_LINEAR(gray + back_rx) as png_uint_32,
                                    PNG_sRGB_FROM_LINEAR(gray + back_gx) as png_uint_32,
                                    PNG_sRGB_FROM_LINEAR(gray + back_bx) as png_uint_32,
                                    255,
                                    P_sRGB as c_int,
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
                /* Exclude the case where the output is gray; we can always handle this
                 * with the cases above.
                 */
                if (output_format & PNG_FORMAT_FLAG_COLOR) == 0 {
                    /* The color-map will be grayscale, so we may as well convert the
                     * input RGB values to a simple grayscale and use the grayscale
                     * code above.
                     *
                     * NOTE: calling this apparently damages the recognition of the
                     * transparent color in background color handling; call
                     * png_set_tRNS_to_alpha before png_set_background_fixed.
                     */
                    png_set_rgb_to_gray_fixed(png_ptr, PNG_ERROR_ACTION_NONE, -1, -1);
                    data_encoding = P_sRGB as c_uint;

                    /* The output will now be one or two 8-bit gray or gray+alpha
                     * channels.  The more complex case arises when the input has alpha.
                     */
                    if ((*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
                        || (*png_ptr).num_trans > 0)
                        && (output_format & PNG_FORMAT_FLAG_ALPHA) != 0
                    {
                        /* Both input and output have an alpha channel, so no background
                         * processing is required; just map the GA bytes to the right
                         * color-map entry.
                         */
                        expand_tRNS = 1;

                        if PNG_GA_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, cstr!("rgb[ga] color-map: too few entries"));
                        }

                        cmap_entries = make_ga_colormap(display) as c_uint;
                        background_index = PNG_CMAP_GA_BACKGROUND as c_uint;
                        output_processing = PNG_CMAP_GA as c_uint;
                    } else {
                        let gamma: png_fixed_point = png_resolve_file_gamma(png_ptr);

                        /* Either the input or the output has no alpha channel, so there
                         * will be no non-opaque pixels in the color-map; it will just be
                         * grayscale.
                         */
                        if PNG_GRAY_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, cstr!("rgb[gray] color-map: too few entries"));
                        }

                        /* Ideally this code would use libpng to do the gamma correction,
                         * but if an input alpha channel is to be removed we will hit the
                         * libpng bug in gamma+compose+rgb-to-gray (the double gamma
                         * correction bug).  Fix this by dropping the gamma correction in
                         * this case and doing it in the palette; this will result in
                         * duplicate palette entries, but that's better than the
                         * alternative of double gamma correction.
                         *
                         * NOTE: PNGv3: check the resolved result of all the potentially
                         * different colour space chunks.
                         */
                        if ((*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
                            || (*png_ptr).num_trans > 0)
                            && png_gamma_not_sRGB(gamma) != 0
                        {
                            cmap_entries = make_gray_file_colormap(display) as c_uint;
                            data_encoding = P_FILE as c_uint;
                        } else {
                            cmap_entries = make_gray_colormap(display) as c_uint;
                        }

                        /* But if the input has alpha or transparency it must be removed
                         */
                        if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
                            || (*png_ptr).num_trans > 0
                        {
                            let mut c: png_color_16 = core::mem::zeroed();
                            let mut gray: png_uint_32 = back_g;

                            /* We need to ensure that the application background exists in
                             * the colormap and that completely transparent pixels map to
                             * it.  Achieve this simply by ensuring that the entry
                             * selected for the background really is the background color.
                             */
                            if data_encoding == P_FILE as c_uint
                            /* from the fixup above */
                            {
                                /* The app supplied a gray which is in output_encoding, we
                                 * need to convert it to a value of the input (P_FILE)
                                 * encoding then set this palette entry to the required
                                 * output encoding.
                                 */
                                if output_encoding == P_sRGB as c_int {
                                    gray = png_sRGB_table[gray as usize] as png_uint_32;
                                    /* now P_LINEAR */
                                }

                                gray = PNG_DIV257(
                                    png_gamma_16bit_correct(gray, gamma) as png_uint_32
                                );
                                /* now P_FILE */

                                /* And make sure the corresponding palette entry contains
                                 * exactly the required sRGB value.
                                 */
                                png_create_colormap_entry(
                                    display,
                                    gray,
                                    back_g,
                                    back_g,
                                    back_g,
                                    0, /*unused*/
                                    output_encoding,
                                );
                            } else if output_encoding == P_LINEAR as c_int {
                                gray = PNG_sRGB_FROM_LINEAR(gray * 255) as png_uint_32;

                                /* And make sure the corresponding palette entry matches.
                                 */
                                png_create_colormap_entry(
                                    display,
                                    gray,
                                    back_g,
                                    back_g,
                                    back_g,
                                    0, /*unused*/
                                    P_LINEAR as c_int,
                                );
                            }

                            /* The background passed to libpng, however, must be the
                             * output (normally sRGB) value.
                             */
                            c.index = 0; /*unused*/
                            c.blue = gray as png_uint_16;
                            c.green = c.blue;
                            c.red = c.green;
                            c.gray = c.red;

                            /* NOTE: the following is apparently a bug in libpng. Without
                             * it the transparent color recognition in
                             * png_set_background_fixed seems to go wrong.
                             */
                            expand_tRNS = 1;
                            png_set_background_fixed(
                                png_ptr,
                                &c as png_const_color_16p,
                                PNG_BACKGROUND_GAMMA_SCREEN,
                                0, /*need_expand*/
                                0, /*gamma: not used*/
                            );
                        }

                        output_processing = PNG_CMAP_NONE as c_uint;
                    }
                } else
                /* output is color */
                {
                    /* We could use png_quantize here so long as there is no transparent
                     * color or alpha; png_quantize ignores alpha.  Easier overall just
                     * to do it once and using PNG_DIV51 on the 6x6x6 reduced RGB cube.
                     * Consequently we always want libpng to produce sRGB data.
                     */
                    data_encoding = P_sRGB as c_uint;

                    /* Is there any transparency or alpha? */
                    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA
                        || (*png_ptr).num_trans > 0
                    {
                        /* Is there alpha in the output too?  If so all four channels are
                         * processed into a special RGB cube with alpha support.
                         */
                        if (output_format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                            let mut r: png_uint_32;

                            if PNG_RGB_COLORMAP_ENTRIES + 1 + 27 > (*image).colormap_entries {
                                png_error(png_ptr, cstr!("rgb+alpha color-map: too few entries"));
                            }

                            cmap_entries = make_rgb_colormap(display) as c_uint;

                            /* Add a transparent entry. */
                            png_create_colormap_entry(
                                display,
                                cmap_entries,
                                255,
                                255,
                                255,
                                0,
                                P_sRGB as c_int,
                            );

                            /* This is stored as the background index for the processing
                             * algorithm.
                             */
                            background_index = cmap_entries;
                            cmap_entries += 1;

                            /* Add 27 r,g,b entries each with alpha 0.5. */
                            r = 0;
                            while r < 256 {
                                let mut g: png_uint_32;

                                g = 0;
                                while g < 256 {
                                    let mut b: png_uint_32;

                                    /* This generates components with the values 0, 127 and
                                     * 255
                                     */
                                    b = 0;
                                    while b < 256 {
                                        png_create_colormap_entry(
                                            display,
                                            cmap_entries,
                                            r,
                                            g,
                                            b,
                                            128,
                                            P_sRGB as c_int,
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
                            /* Alpha/transparency must be removed.  The background must
                             * exist in the color map (achieved by setting adding it after
                             * the 666 color-map).  If the standard processing code will
                             * pick up this entry automatically that's all that is
                             * required; libpng can be called to do the background
                             * processing.
                             */
                            let sample_size: c_uint = PNG_IMAGE_SAMPLE_SIZE(output_format);
                            let mut r: png_uint_32;
                            let mut g: png_uint_32;
                            let mut b: png_uint_32; /* sRGB background */

                            if PNG_RGB_COLORMAP_ENTRIES + 1 + 27 > (*image).colormap_entries {
                                png_error(png_ptr, cstr!("rgb-alpha color-map: too few entries"));
                            }

                            cmap_entries = make_rgb_colormap(display) as c_uint;

                            png_create_colormap_entry(
                                display,
                                cmap_entries,
                                back_r,
                                back_g,
                                back_b,
                                0, /*unused*/
                                output_encoding,
                            );

                            if output_encoding == P_LINEAR as c_int {
                                r = PNG_sRGB_FROM_LINEAR(back_r * 255) as png_uint_32;
                                g = PNG_sRGB_FROM_LINEAR(back_g * 255) as png_uint_32;
                                b = PNG_sRGB_FROM_LINEAR(back_b * 255) as png_uint_32;
                            } else {
                                r = back_r;
                                g = back_g;
                                b = back_b;
                            }

                            /* Compare the newly-created color-map entry with the one the
                             * PNG_CMAP_RGB algorithm will use.  If the two entries don't
                             * match, add the new one and set this as the background
                             * index.
                             */
                            if memcmp(
                                ((*display).colormap as png_const_bytep)
                                    .offset((sample_size * cmap_entries) as isize)
                                    as *const c_void,
                                ((*display).colormap as png_const_bytep).offset(
                                    (sample_size * PNG_RGB_INDEX(r, g, b) as c_uint) as isize,
                                ) as *const c_void,
                                sample_size as usize,
                            ) != 0
                            {
                                /* The background color must be added. */
                                background_index = cmap_entries;
                                cmap_entries += 1;

                                /* Add 27 r,g,b entries each with created by composing with
                                 * the background at alpha 0.5.
                                 */
                                r = 0;
                                while r < 256 {
                                    g = 0;
                                    while g < 256 {
                                        /* This generates components with the values 0, 127
                                         * and 255
                                         */
                                        b = 0;
                                        while b < 256 {
                                            png_create_colormap_entry(
                                                display,
                                                cmap_entries,
                                                png_colormap_compose(
                                                    display,
                                                    r,
                                                    P_sRGB as c_int,
                                                    128,
                                                    back_r,
                                                    output_encoding,
                                                ),
                                                png_colormap_compose(
                                                    display,
                                                    g,
                                                    P_sRGB as c_int,
                                                    128,
                                                    back_g,
                                                    output_encoding,
                                                ),
                                                png_colormap_compose(
                                                    display,
                                                    b,
                                                    P_sRGB as c_int,
                                                    128,
                                                    back_b,
                                                    output_encoding,
                                                ),
                                                0, /*unused*/
                                                output_encoding,
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
                            } else
                            /* background color is in the standard color-map */
                            {
                                let mut c: png_color_16 = core::mem::zeroed();

                                c.index = 0; /*unused*/
                                c.red = back_r as png_uint_16;
                                c.green = back_g as png_uint_16;
                                c.gray = c.green;
                                c.blue = back_b as png_uint_16;

                                png_set_background_fixed(
                                    png_ptr,
                                    &c as png_const_color_16p,
                                    PNG_BACKGROUND_GAMMA_SCREEN,
                                    0, /*need_expand*/
                                    0, /*gamma: not used*/
                                );

                                output_processing = PNG_CMAP_RGB as c_uint;
                            }
                        }
                    } else
                    /* no alpha or transparency in the input */
                    {
                        /* Alpha in the output is irrelevant, simply map the opaque input
                         * pixels to the 6x6x6 color-map.
                         */
                        if PNG_RGB_COLORMAP_ENTRIES > (*image).colormap_entries {
                            png_error(png_ptr, cstr!("rgb color-map: too few entries"));
                        }

                        cmap_entries = make_rgb_colormap(display) as c_uint;
                        output_processing = PNG_CMAP_RGB as c_uint;
                    }
                }
            }

            PNG_COLOR_TYPE_PALETTE => {
                /* It's already got a color-map.  It may be necessary to eliminate the
                 * tRNS entries though.
                 */
                {
                    let mut num_trans: c_uint = (*png_ptr).num_trans as c_uint;
                    let trans: png_const_bytep = if num_trans > 0 {
                        (*png_ptr).trans_alpha as png_const_bytep
                    } else {
                        core::ptr::null()
                    };
                    let colormap: png_const_colorp = (*png_ptr).palette as png_const_colorp;
                    let do_background: c_int = (!trans.is_null()
                        && (output_format & PNG_FORMAT_FLAG_ALPHA) == 0)
                        as c_int;
                    let mut i: c_uint;

                    /* Just in case: */
                    if trans.is_null() {
                        num_trans = 0;
                    }

                    output_processing = PNG_CMAP_NONE as c_uint;
                    data_encoding = P_FILE as c_uint; /* Don't change from color-map indices */
                    cmap_entries = (*png_ptr).num_palette as c_uint;
                    if cmap_entries > 256 {
                        cmap_entries = 256;
                    }

                    if cmap_entries > (*image).colormap_entries as c_uint {
                        png_error(png_ptr, cstr!("palette color-map: too few entries"));
                    }

                    i = 0;
                    while i < cmap_entries {
                        if do_background != 0
                            && i < num_trans
                            && *trans.offset(i as isize) < 255
                        {
                            if *trans.offset(i as isize) == 0 {
                                png_create_colormap_entry(
                                    display,
                                    i,
                                    back_r,
                                    back_g,
                                    back_b,
                                    0,
                                    output_encoding,
                                );
                            } else {
                                /* Must compose the PNG file color in the color-map entry
                                 * on the sRGB color in 'back'.
                                 */
                                png_create_colormap_entry(
                                    display,
                                    i,
                                    png_colormap_compose(
                                        display,
                                        (*colormap.offset(i as isize)).red as png_uint_32,
                                        P_FILE as c_int,
                                        *trans.offset(i as isize) as png_uint_32,
                                        back_r,
                                        output_encoding,
                                    ),
                                    png_colormap_compose(
                                        display,
                                        (*colormap.offset(i as isize)).green as png_uint_32,
                                        P_FILE as c_int,
                                        *trans.offset(i as isize) as png_uint_32,
                                        back_g,
                                        output_encoding,
                                    ),
                                    png_colormap_compose(
                                        display,
                                        (*colormap.offset(i as isize)).blue as png_uint_32,
                                        P_FILE as c_int,
                                        *trans.offset(i as isize) as png_uint_32,
                                        back_b,
                                        output_encoding,
                                    ),
                                    if output_encoding == P_LINEAR as c_int {
                                        *trans.offset(i as isize) as c_uint * 257u32
                                    } else {
                                        *trans.offset(i as isize) as c_uint
                                    },
                                    output_encoding,
                                );
                            }
                        } else {
                            png_create_colormap_entry(
                                display,
                                i,
                                (*colormap.offset(i as isize)).red as png_uint_32,
                                (*colormap.offset(i as isize)).green as png_uint_32,
                                (*colormap.offset(i as isize)).blue as png_uint_32,
                                if i < num_trans {
                                    *trans.offset(i as isize) as c_uint
                                } else {
                                    255u32
                                },
                                P_FILE as c_int, /*8-bit*/
                            );
                        }

                        i += 1;
                    }

                    /* The PNG data may have indices packed in fewer than 8 bits, it
                     * must be expanded if so.
                     */
                    if (*png_ptr).bit_depth < 8 {
                        png_set_packing(png_ptr);
                    }
                }
            }

            _ => {
                png_error(png_ptr, cstr!("invalid PNG color type"));
                /*NOT REACHED*/
            }
        }

        break 'color_type;
    }

    /* Now deal with the output processing */
    if expand_tRNS != 0
        && (*png_ptr).num_trans > 0
        && ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_ALPHA) == 0
    {
        png_set_tRNS_to_alpha(png_ptr);
    }

    if data_encoding == P_sRGB as c_uint || data_encoding == P_FILE as c_uint {
        if data_encoding == P_sRGB as c_uint {
            /* Change to 8-bit sRGB */
            png_set_alpha_mode_fixed(png_ptr, PNG_ALPHA_PNG, PNG_GAMMA_sRGB);
            /* FALLTHROUGH */
        }

        /* case P_FILE: */
        if (*png_ptr).bit_depth > 8 {
            png_set_scale_16(png_ptr);
        }
    } else {
        png_error(png_ptr, cstr!("bad data option (internal error)"));
    }

    if cmap_entries > 256 || cmap_entries > (*image).colormap_entries {
        png_error(png_ptr, cstr!("color map overflow (BAD internal error)"));
    }

    (*image).colormap_entries = cmap_entries;

    /* Double check using the recorded background index */
    let bad_background: bool = if output_processing == PNG_CMAP_NONE as c_uint {
        background_index != PNG_CMAP_NONE_BACKGROUND as c_uint
    } else if output_processing == PNG_CMAP_GA as c_uint {
        background_index != PNG_CMAP_GA_BACKGROUND as c_uint
    } else if output_processing == PNG_CMAP_TRANS as c_uint {
        background_index >= cmap_entries
            || background_index != PNG_CMAP_TRANS_BACKGROUND as c_uint
    } else if output_processing == PNG_CMAP_RGB as c_uint {
        background_index != PNG_CMAP_RGB_BACKGROUND as c_uint
    } else if output_processing == PNG_CMAP_RGB_ALPHA as c_uint {
        background_index != PNG_CMAP_RGB_ALPHA_BACKGROUND as c_uint
    } else {
        png_error(png_ptr, cstr!("bad processing option (internal error)"))
    };

    if bad_background {
        png_error(png_ptr, cstr!("bad background index (internal error)"));
    }

    (*display).colormap_processing = output_processing as c_int;

    1 /*ok*/
}
