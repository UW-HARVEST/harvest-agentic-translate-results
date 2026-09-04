//! Translation of c_src/src/png.c lines 1501..2725
use crate::prelude::*;

/* <float.h>: minimum base-10 exponent for a normalized double (IEEE-754). */
const DBL_MIN_10_EXP: c_int = -307;

/* ------------------------------------------------------------------ */
/* PNG_READ_iCCP_SUPPORTED                                             */
/* ------------------------------------------------------------------ */

/* Error message generation */
pub unsafe extern "C" fn png_icc_tag_char(mut byte: png_uint_32) -> c_char {
    byte &= 0xff;
    if byte >= 32 && byte <= 126 {
        byte as c_char
    } else {
        b'?' as c_char
    }
}

pub unsafe extern "C" fn png_icc_tag_name(name: *mut c_char, tag: png_uint_32) {
    *name.add(0) = b'\'' as c_char;
    *name.add(1) = png_icc_tag_char(tag >> 24);
    *name.add(2) = png_icc_tag_char(tag >> 16);
    *name.add(3) = png_icc_tag_char(tag >> 8);
    *name.add(4) = png_icc_tag_char(tag);
    *name.add(5) = b'\'' as c_char;
}

pub unsafe extern "C" fn is_ICC_signature_char(it: png_alloc_size_t) -> c_int {
    (it == 32 || (it >= 48 && it <= 57) || (it >= 65 && it <= 90) || (it >= 97 && it <= 122))
        as c_int
}

pub unsafe extern "C" fn is_ICC_signature(it: png_alloc_size_t) -> c_int {
    (is_ICC_signature_char(it >> 24) != 0 /* checks all the top bits */
        && is_ICC_signature_char((it >> 16) & 0xff) != 0
        && is_ICC_signature_char((it >> 8) & 0xff) != 0
        && is_ICC_signature_char(it & 0xff) != 0) as c_int
}

pub unsafe extern "C" fn png_icc_profile_error(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    value: png_alloc_size_t,
    reason: png_const_charp,
) -> c_int {
    let mut pos: usize;
    let mut message: [c_char; 196] = [0; 196]; /* see below for calculation */

    pos = png_safecat(
        message.as_mut_ptr(),
        core::mem::size_of_val(&message),
        0,
        cstr(b"profile '\0"),
    ); /* 9 chars */
    pos = png_safecat(message.as_mut_ptr(), pos + 79, pos, name); /* Truncate to 79 chars */
    pos = png_safecat(
        message.as_mut_ptr(),
        core::mem::size_of_val(&message),
        pos,
        cstr(b"': \0"),
    ); /* +2 = 90 */
    if is_ICC_signature(value) != 0 {
        /* So 'value' is at most 4 bytes and the following cast is safe */
        png_icc_tag_name(message.as_mut_ptr().add(pos), value as png_uint_32);
        pos += 6; /* total +8; less than the else clause */
        *message.as_mut_ptr().add(pos) = b':' as c_char;
        pos += 1;
        *message.as_mut_ptr().add(pos) = b' ' as c_char;
        pos += 1;
    } else {
        let mut number: [c_char; PNG_NUMBER_BUFFER_SIZE] = [0; PNG_NUMBER_BUFFER_SIZE]; /* +24 = 114 */

        pos = png_safecat(
            message.as_mut_ptr(),
            core::mem::size_of_val(&message),
            pos,
            png_format_number(
                number.as_ptr(),
                number.as_mut_ptr().add(core::mem::size_of_val(&number)),
                PNG_NUMBER_FORMAT_x,
                value,
            ),
        );
        pos = png_safecat(
            message.as_mut_ptr(),
            core::mem::size_of_val(&message),
            pos,
            cstr(b"h: \0"),
        ); /* +2 = 116 */
    }
    /* The 'reason' is an arbitrary message, allow +79 maximum 195 */
    pos = png_safecat(
        message.as_mut_ptr(),
        core::mem::size_of_val(&message),
        pos,
        reason,
    );
    let _ = pos;

    png_chunk_benign_error(png_ptr, message.as_ptr());

    0
}

/* Encoded value of D50 as an ICC XYZNumber.  From the ICC 2010 spec the value
 * is XYZ(0.9642,1.0,0.8249), which scales to:
 *
 *    (63189.8112, 65536, 54060.6464)
 */
static D50_nCIEXYZ: [png_byte; 12] = [
    0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
];

pub unsafe extern "C" fn icc_check_length(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    profile_length: png_uint_32,
) -> c_int {
    if profile_length < 132 {
        return png_icc_profile_error(
            png_ptr,
            name,
            profile_length as png_alloc_size_t,
            cstr(b"too short\0"),
        );
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_icc_check_length(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    profile_length: png_uint_32,
) -> c_int {
    if icc_check_length(png_ptr, name, profile_length) == 0 {
        return 0;
    }

    /* This needs to be here because the 'normal' check is in
     * png_decompress_chunk, yet this happens after the attempt to
     * png_malloc_base the required data.  We only need this on read; on write
     * the caller supplies the profile buffer so libpng doesn't allocate it.  See
     * the call to icc_check_length below (the write case).
     */
    if profile_length as png_alloc_size_t > png_chunk_max(png_ptr) {
        return png_icc_profile_error(
            png_ptr,
            name,
            profile_length as png_alloc_size_t,
            cstr(b"profile too long\0"),
        );
    }

    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_icc_check_header(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    profile_length: png_uint_32,
    profile: png_const_bytep, /* first 132 bytes only */
    color_type: c_int,
) -> c_int {
    let mut temp: png_uint_32;

    /* Length check; this cannot be ignored in this code because profile_length
     * is used later to check the tag table, so even if the profile seems over
     * long profile_length from the caller must be correct.
     */
    temp = png_get_uint_32(profile);
    if temp != profile_length {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            cstr(b"length does not match profile\0"),
        );
    }

    temp = *(profile.add(8)) as png_uint_32;
    if temp > 3 && (profile_length & 3) != 0 {
        return png_icc_profile_error(
            png_ptr,
            name,
            profile_length as png_alloc_size_t,
            cstr(b"invalid length\0"),
        );
    }

    temp = png_get_uint_32(profile.add(128)); /* tag count: 12 bytes/tag */
    if temp > 357913930 || /* (2^32-4-132)/12: maximum possible tag count */
        profile_length < 132u32.wrapping_add(12u32.wrapping_mul(temp))
    /* truncated tag table */
    {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            cstr(b"tag count too large\0"),
        );
    }

    /* The 'intent' must be valid or we can't store it, ICC limits the intent to
     * 16 bits.
     */
    temp = png_get_uint_32(profile.add(64));
    if temp >= 0xffff {
        /* The ICC limit */
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            cstr(b"invalid rendering intent\0"),
        );
    }

    /* This is just a warning because the profile may be valid in future
     * versions.
     */
    if temp >= PNG_sRGB_INTENT_LAST as png_uint_32 {
        let _ = png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            cstr(b"intent outside defined range\0"),
        );
    }

    /* Data checks (could be skipped). */
    temp = png_get_uint_32(profile.add(36)); /* signature 'ascp' */
    if temp != 0x61637370 {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            cstr(b"invalid signature\0"),
        );
    }

    /* PCS illuminant/adopted white point required to be D50 (warning only). */
    if memcmp(
        profile.add(68) as *const c_void,
        D50_nCIEXYZ.as_ptr() as *const c_void,
        12,
    ) != 0
    {
        let _ = png_icc_profile_error(
            png_ptr,
            name,
            0, /*no tag value*/
            cstr(b"PCS illuminant is not D50\0"),
        );
    }

    temp = png_get_uint_32(profile.add(16)); /* data colour space field */
    match temp {
        0x52474220 => {
            /* 'RGB ' */
            if (color_type & PNG_COLOR_MASK_COLOR) == 0 {
                return png_icc_profile_error(
                    png_ptr,
                    name,
                    temp as png_alloc_size_t,
                    cstr(b"RGB color space not permitted on grayscale PNG\0"),
                );
            }
        }

        0x47524159 => {
            /* 'GRAY' */
            if (color_type & PNG_COLOR_MASK_COLOR) != 0 {
                return png_icc_profile_error(
                    png_ptr,
                    name,
                    temp as png_alloc_size_t,
                    cstr(b"Gray color space not permitted on RGB PNG\0"),
                );
            }
        }

        _ => {
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                cstr(b"invalid ICC profile color space\0"),
            );
        }
    }

    temp = png_get_uint_32(profile.add(12)); /* profile/device class */
    match temp {
        0x73636e72 /* 'scnr' */
        | 0x6d6e7472 /* 'mntr' */
        | 0x70727472 /* 'prtr' */
        | 0x73706163 /* 'spac' */ => {
            /* All supported */
        }

        0x61627374 => {
            /* 'abst' - May not be embedded in an image */
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                cstr(b"invalid embedded Abstract ICC profile\0"),
            );
        }

        0x6c696e6b => {
            /* 'link' - DeviceLink profiles cannot be embedded */
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                cstr(b"unexpected DeviceLink ICC profile class\0"),
            );
        }

        0x6e6d636c => {
            /* 'nmcl' - device specific, warning */
            let _ = png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                cstr(b"unexpected NamedColor ICC profile class\0"),
            );
        }

        _ => {
            /* accept unrecognized profile classes with a warning */
            let _ = png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                cstr(b"unrecognized ICC profile class\0"),
            );
        }
    }

    /* For any profile other than a device link one the PCS must be encoded
     * either in XYZ or Lab.
     */
    temp = png_get_uint_32(profile.add(20));
    match temp {
        0x58595a20 /* 'XYZ ' */ | 0x4c616220 /* 'Lab ' */ => {}

        _ => {
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                cstr(b"unexpected ICC PCS encoding\0"),
            );
        }
    }

    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_icc_check_tag_table(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    profile_length: png_uint_32,
    profile: png_const_bytep, /* header plus whole tag table */
) -> c_int {
    let tag_count: png_uint_32 = png_get_uint_32(profile.add(128));
    let mut itag: png_uint_32;
    let mut tag: png_const_bytep = profile.add(132); /* The first tag */

    /* First scan all the tags in the table. */
    itag = 0;
    while itag < tag_count {
        let tag_id: png_uint_32 = png_get_uint_32(tag.add(0));
        let tag_start: png_uint_32 = png_get_uint_32(tag.add(4)); /* must be aligned */
        let tag_length: png_uint_32 = png_get_uint_32(tag.add(8)); /* not padded */

        /* This is a hard error; potentially it can cause read outside the
         * profile.
         */
        if tag_start > profile_length || tag_length > profile_length - tag_start {
            return png_icc_profile_error(
                png_ptr,
                name,
                tag_id as png_alloc_size_t,
                cstr(b"ICC profile tag outside profile\0"),
            );
        }

        if (tag_start & 3) != 0 {
            /* CNHP730S.icc shipped with Microsoft Windows 64 violates this; it is
             * only a warning here because libpng does not care about the
             * alignment.
             */
            let _ = png_icc_profile_error(
                png_ptr,
                name,
                tag_id as png_alloc_size_t,
                cstr(b"ICC profile tag start not a multiple of 4\0"),
            );
        }

        itag += 1;
        tag = tag.add(12);
    }

    1 /* success, maybe with warnings */
}

/* ------------------------------------------------------------------ */
/* PNG_READ_RGB_TO_GRAY_SUPPORTED                                      */
/* (READ_mDCV_SUPPORTED && READ_cHRM_SUPPORTED both defined)           */
/* ------------------------------------------------------------------ */

pub unsafe extern "C" fn have_chromaticities(png_ptr: png_const_structrp) -> c_int {
    /* Handle new PNGv3 chunks and the precedence rules to determine whether
     * png_struct::chromaticities must be processed.  Only required for RGB to
     * gray.
     */

    /* mDCV */
    if png_file_has_chunk(png_ptr, PNG_INDEX_mDCV) {
        return 1;
    }

    /* sRGB */
    if png_file_has_chunk(png_ptr, PNG_INDEX_sRGB) {
        return 0;
    }

    /* cHRM */
    if png_file_has_chunk(png_ptr, PNG_INDEX_cHRM) {
        return 1;
    }

    0 /* sRGB defaults */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_rgb_coefficients(png_ptr: png_structrp) {
    /* Set the rgb_to_gray coefficients from the colorspace if available. */
    if (*png_ptr).rgb_to_gray_coefficients_set == 0 {
        /* check_chromaticities == 1 */
        let mut xyz: png_XYZ = png_XYZ::default();

        if have_chromaticities(png_ptr) != 0
            && png_XYZ_from_xy(&mut xyz, &(*png_ptr).chromaticities) == 0
        {
            /* png_set_rgb_to_gray has not set the coefficients, get them from the
             * Y values of the colorspace colorants.
             */
            let mut r: png_fixed_point = xyz.red_Y;
            let mut g: png_fixed_point = xyz.green_Y;
            let mut b: png_fixed_point = xyz.blue_Y;
            let total: png_fixed_point = r + g + b;

            if total > 0
                && r >= 0
                && png_muldiv(&mut r, r, 32768, total) != 0
                && r >= 0
                && r <= 32768
                && g >= 0
                && png_muldiv(&mut g, g, 32768, total) != 0
                && g >= 0
                && g <= 32768
                && b >= 0
                && png_muldiv(&mut b, b, 32768, total) != 0
                && b >= 0
                && b <= 32768
                && r + g + b <= 32769
            {
                /* We allow 0 coefficients here.  r+g+b may be 32769 if two or
                 * all of the coefficients were rounded up.  Handle this by
                 * reducing the *largest* coefficient by 1.
                 */
                let mut add: c_int = 0;

                if r + g + b > 32768 {
                    add = -1;
                } else if r + g + b < 32768 {
                    add = 1;
                }

                if add != 0 {
                    if g >= r && g >= b {
                        g += add;
                    } else if r >= g && r >= b {
                        r += add;
                    } else {
                        b += add;
                    }
                }

                /* Check for an internal error. */
                if r + g + b != 32768 {
                    png_error(
                        png_ptr,
                        cstr(b"internal error handling cHRM coefficients\0"),
                    );
                } else {
                    (*png_ptr).rgb_to_gray_red_coeff = r as png_uint_16;
                    (*png_ptr).rgb_to_gray_green_coeff = g as png_uint_16;
                }
            }
        } else {
            /* Use the historical REC 709 (etc) values: */
            (*png_ptr).rgb_to_gray_red_coeff = 6968;
            (*png_ptr).rgb_to_gray_green_coeff = 23434;
            /* png_ptr->rgb_to_gray_blue_coeff  = 2366; */
        }
    }
}

/* ------------------------------------------------------------------ */
/* png_check_IHDR                                                      */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_check_IHDR(
    png_ptr: png_const_structrp,
    width: png_uint_32,
    height: png_uint_32,
    bit_depth: c_int,
    color_type: c_int,
    interlace_type: c_int,
    compression_type: c_int,
    filter_type: c_int,
) {
    let mut error: c_int = 0;

    /* Check for width and height valid values */
    if width == 0 {
        png_warning(png_ptr, cstr(b"Image width is zero in IHDR\0"));
        error = 1;
    }

    if width > PNG_UINT_31_MAX {
        png_warning(png_ptr, cstr(b"Invalid image width in IHDR\0"));
        error = 1;
    }

    /* The bit mask on the first line below must be at least as big as a
     * png_uint_32. Casting to (png_alloc_size_t) makes the type of the result
     * at least as big (in bits) as the RHS of the > operator.
     */
    if (((width as png_alloc_size_t + 7) & !(7 as png_alloc_size_t))
        > (((PNG_SIZE_MAX
            - 48        /* big_row_buf hack */
            - 1)        /* filter byte */
            / 8)        /* 8-byte RGBA pixels */
            - 1))
    /* extra max_pixel_depth pad */
    {
        png_warning(
            png_ptr,
            cstr(b"Image width is too large for this architecture\0"),
        );
        error = 1;
    }

    if width > (*png_ptr).user_width_max {
        png_warning(png_ptr, cstr(b"Image width exceeds user limit in IHDR\0"));
        error = 1;
    }

    if height == 0 {
        png_warning(png_ptr, cstr(b"Image height is zero in IHDR\0"));
        error = 1;
    }

    if height > PNG_UINT_31_MAX {
        png_warning(png_ptr, cstr(b"Invalid image height in IHDR\0"));
        error = 1;
    }

    if height > (*png_ptr).user_height_max {
        png_warning(png_ptr, cstr(b"Image height exceeds user limit in IHDR\0"));
        error = 1;
    }

    /* Check other values */
    if bit_depth != 1 && bit_depth != 2 && bit_depth != 4 && bit_depth != 8 && bit_depth != 16 {
        png_warning(png_ptr, cstr(b"Invalid bit depth in IHDR\0"));
        error = 1;
    }

    if color_type < 0 || color_type == 1 || color_type == 5 || color_type > 6 {
        png_warning(png_ptr, cstr(b"Invalid color type in IHDR\0"));
        error = 1;
    }

    if ((color_type == PNG_COLOR_TYPE_PALETTE) && bit_depth > 8)
        || ((color_type == PNG_COLOR_TYPE_RGB
            || color_type == PNG_COLOR_TYPE_GRAY_ALPHA
            || color_type == PNG_COLOR_TYPE_RGB_ALPHA)
            && bit_depth < 8)
    {
        png_warning(
            png_ptr,
            cstr(b"Invalid color type/bit depth combination in IHDR\0"),
        );
        error = 1;
    }

    if interlace_type >= PNG_INTERLACE_LAST {
        png_warning(png_ptr, cstr(b"Unknown interlace method in IHDR\0"));
        error = 1;
    }

    if compression_type != PNG_COMPRESSION_TYPE_BASE {
        png_warning(png_ptr, cstr(b"Unknown compression method in IHDR\0"));
        error = 1;
    }

    /* PNG_MNG_FEATURES_SUPPORTED */
    /* Accept filter_method 64 (intrapixel differencing) only if ... */
    if ((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) != 0 && (*png_ptr).mng_features_permitted != 0 {
        png_warning(
            png_ptr,
            cstr(b"MNG features are not allowed in a PNG datastream\0"),
        );
    }

    if filter_type != PNG_FILTER_TYPE_BASE {
        if !(((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
            && (filter_type == PNG_INTRAPIXEL_DIFFERENCING)
            && (((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) == 0)
            && (color_type == PNG_COLOR_TYPE_RGB || color_type == PNG_COLOR_TYPE_RGB_ALPHA))
        {
            png_warning(png_ptr, cstr(b"Unknown filter method in IHDR\0"));
            error = 1;
        }

        if ((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) != 0 {
            png_warning(png_ptr, cstr(b"Invalid filter method in IHDR\0"));
            error = 1;
        }
    }

    if error == 1 {
        png_error(png_ptr, cstr(b"Invalid IHDR data\0"));
    }
}

/* ------------------------------------------------------------------ */
/* ASCII to fp functions (pCAL || sCAL)                                */
/* ------------------------------------------------------------------ */

/* The following is used internally to preserve the sticky flags:
 *   png_fp_add(state, flags) == (state) |= (flags)
 *   png_fp_set(state, value) == (state) = (value) | ((state) & PNG_FP_STICKY)
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_check_fp_number(
    string: png_const_charp,
    size: usize,
    statep: *mut c_int,
    whereami: *mut usize,
) -> c_int {
    let mut state: c_int = *statep;
    let mut i: usize = *whereami;

    'fp_end: loop {
        while i < size {
            let type_: c_int;
            /* First find the type of the next character */
            match *string.add(i) as c_int {
                43 => type_ = PNG_FP_SAW_SIGN,
                45 => type_ = PNG_FP_SAW_SIGN + PNG_FP_NEGATIVE,
                46 => type_ = PNG_FP_SAW_DOT,
                48 => type_ = PNG_FP_SAW_DIGIT,
                49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 | 57 => {
                    type_ = PNG_FP_SAW_DIGIT + PNG_FP_NONZERO
                }
                69 | 101 => type_ = PNG_FP_SAW_E,
                _ => break 'fp_end,
            }

            /* Now deal with this type according to the current state. */
            match (state & PNG_FP_STATE) + (type_ & PNG_FP_SAW_ANY) {
                x if x == PNG_FP_INTEGER + PNG_FP_SAW_SIGN => {
                    if (state & PNG_FP_SAW_ANY) != 0 {
                        break 'fp_end; /* not a part of the number */
                    }

                    state |= type_;
                }

                x if x == PNG_FP_INTEGER + PNG_FP_SAW_DOT => {
                    /* Ok as trailer, ok as lead of fraction. */
                    if (state & PNG_FP_SAW_DOT) != 0 {
                        /* two dots */
                        break 'fp_end;
                    } else if (state & PNG_FP_SAW_DIGIT) != 0 {
                        /* trailing dot? */
                        state |= type_;
                    } else {
                        state = (PNG_FP_FRACTION | type_) | (state & PNG_FP_STICKY);
                    }
                }

                x if x == PNG_FP_INTEGER + PNG_FP_SAW_DIGIT => {
                    if (state & PNG_FP_SAW_DOT) != 0 {
                        /* delayed fraction */
                        state = (PNG_FP_FRACTION | PNG_FP_SAW_DOT) | (state & PNG_FP_STICKY);
                    }

                    state |= type_ | PNG_FP_WAS_VALID;
                }

                x if x == PNG_FP_INTEGER + PNG_FP_SAW_E => {
                    if (state & PNG_FP_SAW_DIGIT) == 0 {
                        break 'fp_end;
                    }

                    state = PNG_FP_EXPONENT | (state & PNG_FP_STICKY);
                }

                /* case PNG_FP_FRACTION + PNG_FP_SAW_SIGN: no sign in fraction */
                /* case PNG_FP_FRACTION + PNG_FP_SAW_DOT: SAW_DOT always set */
                x if x == PNG_FP_FRACTION + PNG_FP_SAW_DIGIT => {
                    state |= type_ | PNG_FP_WAS_VALID;
                }

                x if x == PNG_FP_FRACTION + PNG_FP_SAW_E => {
                    /* This is correct because the trailing '.' on an integer is
                     * handled above - so we can only get here with ".E".
                     */
                    if (state & PNG_FP_SAW_DIGIT) == 0 {
                        break 'fp_end;
                    }

                    state = PNG_FP_EXPONENT | (state & PNG_FP_STICKY);
                }

                x if x == PNG_FP_EXPONENT + PNG_FP_SAW_SIGN => {
                    if (state & PNG_FP_SAW_ANY) != 0 {
                        break 'fp_end; /* not a part of the number */
                    }

                    state |= PNG_FP_SAW_SIGN;
                }

                /* case PNG_FP_EXPONENT + PNG_FP_SAW_DOT: */
                x if x == PNG_FP_EXPONENT + PNG_FP_SAW_DIGIT => {
                    state |= PNG_FP_SAW_DIGIT | PNG_FP_WAS_VALID;
                }

                /* case PNG_FP_EXPONENT + PNG_FP_SAW_E: */
                _ => break 'fp_end, /* I.e. break 2 */
            }

            /* The character seems ok, continue. */
            i += 1;
        }

        break 'fp_end;
    }

    /* Here at the end, update the state and return the correct return code. */
    *statep = state;
    *whereami = i;

    ((state & PNG_FP_SAW_DIGIT) != 0) as c_int
}

/* The same but for a complete string. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_check_fp_string(string: png_const_charp, size: usize) -> c_int {
    let mut state: c_int = 0;
    let mut char_index: usize = 0;

    if png_check_fp_number(string, size, &mut state, &mut char_index) != 0
        && (char_index == size || *string.add(char_index) == 0)
    {
        return state; /* must be non-zero - see above */
    }

    0 /* i.e. fail */
}

/* ------------------------------------------------------------------ */
/* PNG_sCAL_SUPPORTED && PNG_FLOATING_POINT_SUPPORTED                  */
/* ------------------------------------------------------------------ */

/* Utility used below - a simple accurate power of ten from an integral
 * exponent.
 */
pub unsafe extern "C" fn png_pow10(mut power: c_int) -> f64 {
    let mut recip: c_int = 0;
    let mut d: f64 = 1.0;

    /* Handle negative exponent with a reciprocal at the end because
     * 10 is exact whereas .1 is inexact in base 2
     */
    if power < 0 {
        if power < DBL_MIN_10_EXP {
            return 0.0;
        }
        recip = 1;
        power = -power;
    }

    if power > 0 {
        /* Decompose power bitwise. */
        let mut mult: f64 = 10.0;
        loop {
            if power & 1 != 0 {
                d *= mult;
            }
            mult *= mult;
            power >>= 1;

            if !(power > 0) {
                break;
            }
        }

        if recip != 0 {
            d = 1.0 / d;
        }
    }
    /* else power is 0 and d is 1 */

    d
}

/* Function to format a floating point value in ASCII with a given precision. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_ascii_from_fp(
    png_ptr: png_const_structrp,
    mut ascii: png_charp,
    mut size: usize,
    mut fp: f64,
    mut precision: c_uint,
) {
    /* We use standard functions from math.h, but not printf. */
    if precision < 1 {
        precision = DBL_DIG as c_uint;
    }

    /* Enforce the limit of the implementation precision too. */
    if precision > (DBL_DIG + 1) as c_uint {
        precision = (DBL_DIG + 1) as c_uint;
    }

    /* Basic sanity checks */
    if size >= (precision + 5) as usize {
        /* See the requirements below. */
        if fp < 0.0 {
            fp = -fp;
            *ascii = 45; /* '-'  PLUS 1 TOTAL 1 */
            ascii = ascii.add(1);
            size -= 1;
        }

        if fp >= DBL_MIN && fp <= DBL_MAX {
            let mut exp_b10: c_int = 0; /* A base 10 exponent */
            let mut base: f64; /* 10^exp_b10 */

            /* First extract a base 10 exponent of the number. */
            frexp(fp, &mut exp_b10); /* exponent to base 2 */

            exp_b10 = (exp_b10 * 77) >> 8; /* <= exponent to base 10 */

            /* Avoid underflow here. */
            base = png_pow10(exp_b10); /* May underflow */

            while base < DBL_MIN || base < fp {
                /* And this may overflow. */
                let test: f64 = png_pow10(exp_b10 + 1);

                if test <= DBL_MAX {
                    exp_b10 += 1;
                    base = test;
                } else {
                    break;
                }
            }

            /* Normalize fp and correct exp_b10, after this fp is in the
             * range [.1,1) and exp_b10 is both the exponent and the digit
             * *before* which the decimal point should be inserted.
             */
            fp /= base;
            while fp >= 1.0 {
                fp /= 10.0;
                exp_b10 += 1;
            }

            /* Because of the code above fp may, at this point, be less than .1 */

            {
                let mut czero: c_uint;
                let mut clead: c_uint;
                let mut cdigits: c_uint;
                let mut exponent: [c_char; 10] = [0; 10];

                /* Allow up to two leading zeros. */
                if exp_b10 < 0 && exp_b10 > -3 {
                    /* PLUS 3 TOTAL 4 */
                    czero = (0u32).wrapping_sub(exp_b10 as c_uint); /* PLUS 2 digits: TOTAL 3 */
                    exp_b10 = 0; /* Dot added below before first output. */
                } else {
                    czero = 0; /* No zeros to add */
                }

                /* Generate the digit list. */
                clead = czero; /* Count of leading zeros */
                cdigits = 0; /* Count of digits in list. */

                loop {
                    let mut d: f64;

                    fp *= 10.0;
                    /* Use modf here, not floor and subtract. */
                    if cdigits + czero + 1 < precision + clead {
                        d = 0.0;
                        fp = modf(fp, &mut d);
                    } else {
                        d = floor(fp + 0.5);

                        if d > 9.0 {
                            /* Rounding up to 10, handle that here. */
                            if czero > 0 {
                                czero -= 1;
                                d = 1.0;
                                if cdigits == 0 {
                                    clead -= 1;
                                }
                            } else {
                                while cdigits > 0 && d > 9.0 {
                                    ascii = ascii.sub(1);
                                    let mut ch: c_int = *ascii as c_int;

                                    if exp_b10 != -1 {
                                        exp_b10 += 1;
                                    } else if ch == 46 {
                                        ascii = ascii.sub(1);
                                        ch = *ascii as c_int;
                                        size += 1;
                                        /* Advance exp_b10 to '1'. */
                                        exp_b10 = 1;
                                    }

                                    cdigits -= 1;
                                    d = (ch - 47) as f64; /* I.e. 1+(ch-48) */
                                }

                                /* Did we reach the beginning? */
                                if d > 9.0 {
                                    /* cdigits == 0 */
                                    if exp_b10 == -1 {
                                        /* Leading decimal point (plus zeros?). */
                                        ascii = ascii.sub(1);
                                        let ch: c_int = *ascii as c_int;

                                        if ch == 46 {
                                            size += 1;
                                            exp_b10 = 1;
                                        }
                                    /* Else lost a leading zero, so 'exp_b10' is
                                     * still ok at (-1)
                                     */
                                    } else {
                                        exp_b10 += 1;
                                    }

                                    /* In all cases we output a '1' */
                                    d = 1.0;
                                }
                            }
                        }
                        fp = 0.0; /* Guarantees termination below. */
                    }

                    if d == 0.0 {
                        czero += 1;
                        if cdigits == 0 {
                            clead += 1;
                        }
                    } else {
                        /* Included embedded zeros in the digit count. */
                        cdigits += czero - clead;
                        clead = 0;

                        while czero > 0 {
                            /* exp_b10 == (-1) means we just output the decimal
                             * place - after the DP don't adjust 'exp_b10' any more!
                             */
                            if exp_b10 != -1 {
                                if exp_b10 == 0 {
                                    *ascii = 46;
                                    ascii = ascii.add(1);
                                    size -= 1;
                                }
                                /* PLUS 1: TOTAL 4 */
                                exp_b10 -= 1;
                            }
                            *ascii = 48;
                            ascii = ascii.add(1);
                            czero -= 1;
                        }

                        if exp_b10 != -1 {
                            if exp_b10 == 0 {
                                *ascii = 46;
                                ascii = ascii.add(1);
                                size -= 1; /* counted above */
                            }

                            exp_b10 -= 1;
                        }
                        *ascii = (48 + d as c_int) as c_char;
                        ascii = ascii.add(1);
                        cdigits += 1;
                    }

                    if !(cdigits + czero < precision + clead && fp > DBL_MIN) {
                        break;
                    }
                }

                /* The total output count (max) is now 4+precision */

                /* Check for an exponent. */
                if exp_b10 >= -1 && exp_b10 <= 2 {
                    /* The following only happens if we didn't output the leading
                     * zeros above for negative exponent.
                     */
                    while {
                        let old = exp_b10;
                        exp_b10 -= 1;
                        old > 0
                    } {
                        *ascii = 48;
                        ascii = ascii.add(1);
                    }

                    *ascii = 0;

                    /* Total buffer requirement (including the '\0') is
                     * 5+precision - see check at the start.
                     */
                    return;
                }

                /* Here if an exponent is required, adjust size for the digits we
                 * output but did not count.
                 */
                size -= cdigits as usize;

                *ascii = 69;
                ascii = ascii.add(1);
                size -= 1; /* 'E': PLUS 1 TOTAL 2+precision */

                /* The following use of an unsigned temporary avoids ambiguities. */
                {
                    let mut uexp_b10: c_uint;

                    if exp_b10 < 0 {
                        *ascii = 45;
                        ascii = ascii.add(1);
                        size -= 1; /* '-': PLUS 1 TOTAL 3+precision */
                        uexp_b10 = (0u32).wrapping_sub(exp_b10 as c_uint);
                    } else {
                        uexp_b10 = (0u32).wrapping_add(exp_b10 as c_uint);
                    }

                    cdigits = 0;

                    while uexp_b10 > 0 {
                        exponent[cdigits as usize] = (48 + uexp_b10 % 10) as c_char;
                        cdigits += 1;
                        uexp_b10 /= 10;
                    }
                }

                /* Need another size check here for the exponent digits. */
                if size > cdigits as usize {
                    while cdigits > 0 {
                        cdigits -= 1;
                        *ascii = exponent[cdigits as usize];
                        ascii = ascii.add(1);
                    }

                    *ascii = 0;

                    return;
                }
            }
        } else if !(fp >= DBL_MIN) {
            *ascii = 48; /* '0' */
            ascii = ascii.add(1);
            *ascii = 0;
            return;
        } else {
            *ascii = 105; /* 'i' */
            ascii = ascii.add(1);
            *ascii = 110; /* 'n' */
            ascii = ascii.add(1);
            *ascii = 102; /* 'f' */
            ascii = ascii.add(1);
            *ascii = 0;
            return;
        }
    }

    /* Here on buffer too small. */
    png_error(png_ptr, cstr(b"ASCII conversion buffer too small\0"));
}

/* ------------------------------------------------------------------ */
/* PNG_sCAL_SUPPORTED && PNG_FIXED_POINT_SUPPORTED                     */
/* ------------------------------------------------------------------ */

/* Function to format a fixed point value in ASCII. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_ascii_from_fixed(
    png_ptr: png_const_structrp,
    mut ascii: png_charp,
    size: usize,
    fp: png_fixed_point,
) {
    /* Require space for 10 decimal digits, a decimal point, a minus sign and a
     * trailing \0, 13 characters:
     */
    if size > 12 {
        let mut num: png_uint_32;

        /* Avoid overflow here on the minimum integer. */
        if fp < 0 {
            *ascii = 45;
            ascii = ascii.add(1);
            num = (-fp) as png_uint_32;
        } else {
            num = fp as png_uint_32;
        }

        if num <= 0x80000000 {
            /* else overflowed */
            let mut ndigits: c_uint = 0;
            let mut first: c_uint = 16; /* flag value */
            let mut digits: [c_char; 10] = [0; 10];

            while num != 0 {
                /* Split the low digit off num: */
                let tmp: c_uint = num / 10;
                num -= tmp.wrapping_mul(10);
                digits[ndigits as usize] = (48 + num) as c_char;
                ndigits += 1;
                /* Record the first non-zero digit. */
                if first == 16 && num > 0 {
                    first = ndigits;
                }
                num = tmp;
            }

            if ndigits > 0 {
                while ndigits > 5 {
                    ndigits -= 1;
                    *ascii = digits[ndigits as usize];
                    ascii = ascii.add(1);
                }
                /* The remaining digits are fractional digits. */
                if first <= 5 {
                    let mut i: c_uint;
                    *ascii = 46; /* decimal point */
                    ascii = ascii.add(1);
                    /* ndigits may be <5 for small numbers, output leading zeros
                     * then ndigits digits to first:
                     */
                    i = 5;
                    while ndigits < i {
                        *ascii = 48;
                        ascii = ascii.add(1);
                        i -= 1;
                    }
                    while ndigits >= first {
                        ndigits -= 1;
                        *ascii = digits[ndigits as usize];
                        ascii = ascii.add(1);
                    }
                    /* Don't output the trailing zeros! */
                }
            } else {
                *ascii = 48;
                ascii = ascii.add(1);
            }

            /* And null terminate the string: */
            *ascii = 0;
            return;
        }
    }

    /* Here on buffer too small. */
    png_error(png_ptr, cstr(b"ASCII conversion buffer too small\0"));
}
