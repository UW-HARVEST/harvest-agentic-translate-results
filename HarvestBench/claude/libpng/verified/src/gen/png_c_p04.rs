/* png.c lines 1502..1960 */

/* Error message generation */
/* png_icc_tag_char */
unsafe fn png_icc_tag_char(mut byte: png_uint_32) -> c_char {
    byte &= 0xff;
    if byte >= 32 && byte <= 126 {
        byte as c_char
    } else {
        b'?' as c_char
    }
}

/* png_icc_tag_name */
unsafe fn png_icc_tag_name(name: *mut c_char, tag: png_uint_32) {
    *name.add(0) = b'\'' as c_char;
    *name.add(1) = png_icc_tag_char(tag >> 24);
    *name.add(2) = png_icc_tag_char(tag >> 16);
    *name.add(3) = png_icc_tag_char(tag >> 8);
    *name.add(4) = png_icc_tag_char(tag);
    *name.add(5) = b'\'' as c_char;
}

/* is_ICC_signature_char */
unsafe fn is_ICC_signature_char(it: png_alloc_size_t) -> c_int {
    (it == 32
        || (it >= 48 && it <= 57)
        || (it >= 65 && it <= 90)
        || (it >= 97 && it <= 122)) as c_int
}

/* is_ICC_signature */
unsafe fn is_ICC_signature(it: png_alloc_size_t) -> c_int {
    (is_ICC_signature_char(it >> 24) != 0 /* checks all the top bits */
        && is_ICC_signature_char((it >> 16) & 0xff) != 0
        && is_ICC_signature_char((it >> 8) & 0xff) != 0
        && is_ICC_signature_char(it & 0xff) != 0) as c_int
}

/* png_icc_profile_error */
unsafe fn png_icc_profile_error(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    value: png_alloc_size_t,
    reason: png_const_charp,
) -> c_int {
    let mut pos: usize;
    let mut message: [c_char; 196] = [0; 196]; /* see below for calculation */

    pos = png_safecat(
        message.as_mut_ptr(),
        196, /* sizeof message */
        0,
        b"profile '\0".as_ptr() as png_const_charp,
    ); /* 9 chars */
    pos = png_safecat(message.as_mut_ptr(), pos + 79, pos, name); /* Truncate to 79 chars */
    pos = png_safecat(
        message.as_mut_ptr(),
        196, /* sizeof message */
        pos,
        b"': \0".as_ptr() as png_const_charp,
    ); /* +2 = 90 */
    if is_ICC_signature(value) != 0 {
        /* So 'value' is at most 4 bytes and the following cast is safe */
        png_icc_tag_name(message.as_mut_ptr().add(pos), value as png_uint_32);
        pos += 6; /* total +8; less than the else clause */
        message[pos] = b':' as c_char;
        pos += 1;
        message[pos] = b' ' as c_char;
        pos += 1;
    } else {
        let mut number: [c_char; PNG_NUMBER_BUFFER_SIZE] = [0; PNG_NUMBER_BUFFER_SIZE]; /* +24 = 114 */

        pos = png_safecat(
            message.as_mut_ptr(),
            196, /* sizeof message */
            pos,
            png_format_number(
                number.as_ptr(),
                number.as_mut_ptr().add(PNG_NUMBER_BUFFER_SIZE),
                PNG_NUMBER_FORMAT_x,
                value,
            ),
        );
        pos = png_safecat(
            message.as_mut_ptr(),
            196, /* sizeof message */
            pos,
            b"h: \0".as_ptr() as png_const_charp,
        ); /* +2 = 116 */
    }
    /* The 'reason' is an arbitrary message, allow +79 maximum 195 */
    pos = png_safecat(message.as_mut_ptr(), 196, pos, reason);

    png_chunk_benign_error(png_ptr, message.as_ptr());

    0
}

/* Encoded value of D50 as an ICC XYZNumber.  From the ICC 2010 spec the value
 * is XYZ(0.9642,1.0,0.8249), which scales to:
 *
 *    (63189.8112, 65536, 54060.6464)
 */
/* static const png_byte D50_nCIEXYZ[12] is declared in src/gen/png_c_tables.rs */

/* icc_check_length */
unsafe fn icc_check_length(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    profile_length: png_uint_32,
) -> c_int /* bool */ {
    if profile_length < 132 {
        return png_icc_profile_error(
            png_ptr,
            name,
            profile_length as png_alloc_size_t,
            b"too short\0".as_ptr() as png_const_charp,
        );
    }
    1
}

/* png_icc_check_length */
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
    if (profile_length as png_alloc_size_t) > png_chunk_max(png_ptr) {
        return png_icc_profile_error(
            png_ptr,
            name,
            profile_length as png_alloc_size_t,
            b"profile too long\0".as_ptr() as png_const_charp,
        );
    }

    1
}

/* png_icc_check_header */
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
     * long profile_length from the caller must be correct.  The caller can fix
     * this up on read or write by just passing in the profile header length.
     */
    temp = PNG_get_uint_32(profile);
    if temp != profile_length {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            b"length does not match profile\0".as_ptr() as png_const_charp,
        );
    }

    temp = *(profile.add(8)) as png_uint_32;
    if temp > 3 && (profile_length & 3) != 0 {
        return png_icc_profile_error(
            png_ptr,
            name,
            profile_length as png_alloc_size_t,
            b"invalid length\0".as_ptr() as png_const_charp,
        );
    }

    temp = PNG_get_uint_32(profile.add(128)); /* tag count: 12 bytes/tag */
    if temp > 357913930 || /* (2^32-4-132)/12: maximum possible tag count */
        profile_length < 132u32.wrapping_add(12u32.wrapping_mul(temp))
    /* truncated tag table */
    {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            b"tag count too large\0".as_ptr() as png_const_charp,
        );
    }

    /* The 'intent' must be valid or we can't store it, ICC limits the intent to
     * 16 bits.
     */
    temp = PNG_get_uint_32(profile.add(64));
    if temp >= 0xffff
    /* The ICC limit */
    {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            b"invalid rendering intent\0".as_ptr() as png_const_charp,
        );
    }

    /* This is just a warning because the profile may be valid in future
     * versions.
     */
    if temp >= PNG_sRGB_INTENT_LAST as png_uint_32 {
        png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            b"intent outside defined range\0".as_ptr() as png_const_charp,
        );
    }

    /* At this point the tag table can't be checked because it hasn't necessarily
     * been loaded; however, various header fields can be checked.  These checks
     * are for values permitted by the PNG spec in an ICC profile; the PNG spec
     * restricts the profiles that can be passed in an iCCP chunk (they must be
     * appropriate to processing PNG data!)
     */

    /* Data checks (could be skipped).  These checks must be independent of the
     * version number; however, the version number doesn't accommodate changes in
     * the header fields (just the known tags and the interpretation of the
     * data.)
     */
    temp = PNG_get_uint_32(profile.add(36)); /* signature 'ascp' */
    if temp != 0x61637370 {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            b"invalid signature\0".as_ptr() as png_const_charp,
        );
    }

    /* Currently the PCS illuminant/adopted white point (the computational
     * white point) are required to be D50,
     * however the profile contains a record of the illuminant so perhaps ICC
     * expects to be able to change this in the future (despite the rationale in
     * the introduction for using a fixed PCS adopted white.)  Consequently the
     * following is just a warning.
     */
    if memcmp(
        profile.add(68) as *const c_void,
        D50_nCIEXYZ.as_ptr() as *const c_void,
        12,
    ) != 0
    {
        png_icc_profile_error(
            png_ptr,
            name,
            0, /*no tag value*/
            b"PCS illuminant is not D50\0".as_ptr() as png_const_charp,
        );
    }

    /* The PNG spec requires this:
     * "If the iCCP chunk is present, the image samples conform to the colour
     * space represented by the embedded ICC profile as defined by the
     * International Color Consortium [ICC]. The colour space of the ICC profile
     * shall be an RGB colour space for colour images (PNG colour types 2, 3, and
     * 6), or a greyscale colour space for greyscale images (PNG colour types 0
     * and 4)."
     *
     * This checking code ensures the embedded profile (on either read or write)
     * conforms to the specification requirements.  Notice that an ICC 'gray'
     * color-space profile contains the information to transform the monochrome
     * data to XYZ or L*a*b (according to which PCS the profile uses) and this
     * should be used in preference to the standard libpng K channel replication
     * into R, G and B channels.
     *
     * Previously it was suggested that an RGB profile on grayscale data could be
     * handled.  However it is clear that using an RGB profile in this context
     * must be an error - there is no specification of what it means.  Thus it is
     * almost certainly more correct to ignore the profile.
     */
    temp = PNG_get_uint_32(profile.add(16)); /* data colour space field */
    match temp {
        0x52474220 =>
        /* 'RGB ' */
        {
            if (color_type & PNG_COLOR_MASK_COLOR) == 0 {
                return png_icc_profile_error(
                    png_ptr,
                    name,
                    temp as png_alloc_size_t,
                    b"RGB color space not permitted on grayscale PNG\0".as_ptr()
                        as png_const_charp,
                );
            }
        }

        0x47524159 =>
        /* 'GRAY' */
        {
            if (color_type & PNG_COLOR_MASK_COLOR) != 0 {
                return png_icc_profile_error(
                    png_ptr,
                    name,
                    temp as png_alloc_size_t,
                    b"Gray color space not permitted on RGB PNG\0".as_ptr() as png_const_charp,
                );
            }
        }

        _ => {
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                b"invalid ICC profile color space\0".as_ptr() as png_const_charp,
            );
        }
    }

    /* It is up to the application to check that the profile class matches the
     * application requirements; the spec provides no guidance, but it's pretty
     * weird if the profile is not scanner ('scnr'), monitor ('mntr'), printer
     * ('prtr') or 'spac' (for generic color spaces).  Issue a warning in these
     * cases.  Issue an error for device link or abstract profiles - these don't
     * contain the records necessary to transform the color-space to anything
     * other than the target device (and not even that for an abstract profile).
     * Profiles of these classes may not be embedded in images.
     */
    temp = PNG_get_uint_32(profile.add(12)); /* profile/device class */
    match temp {
        /* 'scnr' */
        /* 'mntr' */
        /* 'prtr' */
        /* 'spac' */
        0x73636e72 | 0x6d6e7472 | 0x70727472 | 0x73706163 => {
            /* All supported */
        }

        0x61627374 =>
        /* 'abst' */
        {
            /* May not be embedded in an image */
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                b"invalid embedded Abstract ICC profile\0".as_ptr() as png_const_charp,
            );
        }

        0x6c696e6b =>
        /* 'link' */
        {
            /* DeviceLink profiles cannot be interpreted in a non-device specific
             * fashion, if an app uses the AToB0Tag in the profile the results are
             * undefined unless the result is sent to the intended device,
             * therefore a DeviceLink profile should not be found embedded in a
             * PNG.
             */
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                b"unexpected DeviceLink ICC profile class\0".as_ptr() as png_const_charp,
            );
        }

        0x6e6d636c =>
        /* 'nmcl' */
        {
            /* A NamedColor profile is also device specific, however it doesn't
             * contain an AToB0 tag that is open to misinterpretation.  Almost
             * certainly it will fail the tests below.
             */
            png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                b"unexpected NamedColor ICC profile class\0".as_ptr() as png_const_charp,
            );
        }

        _ => {
            /* To allow for future enhancements to the profile accept unrecognized
             * profile classes with a warning, these then hit the test below on the
             * tag content to ensure they are backward compatible with one of the
             * understood profiles.
             */
            png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                b"unrecognized ICC profile class\0".as_ptr() as png_const_charp,
            );
        }
    }

    /* For any profile other than a device link one the PCS must be encoded
     * either in XYZ or Lab.
     */
    temp = PNG_get_uint_32(profile.add(20));
    match temp {
        /* 'XYZ ' */
        /* 'Lab ' */
        0x58595a20 | 0x4c616220 => {}

        _ => {
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                b"unexpected ICC PCS encoding\0".as_ptr() as png_const_charp,
            );
        }
    }

    1
}

/* png_icc_check_tag_table */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_icc_check_tag_table(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    profile_length: png_uint_32,
    profile: png_const_bytep, /* header plus whole tag table */
) -> c_int {
    let tag_count: png_uint_32 = PNG_get_uint_32(profile.add(128));
    let mut itag: png_uint_32;
    let mut tag: png_const_bytep = profile.add(132); /* The first tag */

    /* First scan all the tags in the table and add bits to the icc_info value
     * (temporarily in 'tags').
     */
    itag = 0;
    while itag < tag_count {
        let tag_id: png_uint_32 = PNG_get_uint_32(tag.add(0));
        let tag_start: png_uint_32 = PNG_get_uint_32(tag.add(4)); /* must be aligned */
        let tag_length: png_uint_32 = PNG_get_uint_32(tag.add(8)); /* not padded */

        /* The ICC specification does not exclude zero length tags, therefore the
         * start might actually be anywhere if there is no data, but this would be
         * a clear abuse of the intent of the standard so the start is checked for
         * being in range.  All defined tag types have an 8 byte header - a 4 byte
         * type signature then 0.
         */

        /* This is a hard error; potentially it can cause read outside the
         * profile.
         */
        if tag_start > profile_length || tag_length > profile_length.wrapping_sub(tag_start) {
            return png_icc_profile_error(
                png_ptr,
                name,
                tag_id as png_alloc_size_t,
                b"ICC profile tag outside profile\0".as_ptr() as png_const_charp,
            );
        }

        if (tag_start & 3) != 0 {
            /* CNHP730S.icc shipped with Microsoft Windows 64 violates this; it is
             * only a warning here because libpng does not care about the
             * alignment.
             */
            png_icc_profile_error(
                png_ptr,
                name,
                tag_id as png_alloc_size_t,
                b"ICC profile tag start not a multiple of 4\0".as_ptr() as png_const_charp,
            );
        }

        itag += 1;
        tag = tag.add(12);
    }

    1 /* success, maybe with warnings */
}

/* have_chromaticities */
unsafe fn have_chromaticities(png_ptr: png_const_structrp) -> c_int {
    /* Handle new PNGv3 chunks and the precedence rules to determine whether
     * png_struct::chromaticities must be processed.  Only required for RGB to
     * gray.
     *
     * mDCV: this is the mastering colour space and it is independent of the
     *       encoding so it needs to be used regardless of the encoded space.
     *
     * cICP: first in priority but not yet implemented - the chromaticities come
     *       from the 'primaries'.
     *
     * iCCP: not supported by libpng (so ignored)
     *
     * sRGB: the defaults match sRGB
     *
     * cHRM: calculate the coefficients
     */
    if png_file_has_chunk(png_ptr, PNG_INDEX_mDCV) {
        return 1;
    }

    if png_file_has_chunk(png_ptr, PNG_INDEX_sRGB) {
        return 0;
    }

    if png_file_has_chunk(png_ptr, PNG_INDEX_cHRM) {
        return 1;
    }

    0 /* sRGB defaults */
}

/* png_set_rgb_coefficients */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_rgb_coefficients(png_ptr: png_structrp) {
    /* Set the rgb_to_gray coefficients from the colorspace if available.  Note
     * that '_set' means that png_rgb_to_gray was called **and** it successfully
     * set up the coefficients.
     */
    if (*png_ptr).rgb_to_gray_coefficients_set == 0 {
        /* check_chromaticities is 1 (READ_mDCV and READ_cHRM are supported) */
        let mut xyz: png_XYZ = Default::default();

        if have_chromaticities(png_ptr) != 0
            && png_XYZ_from_xy(
                core::ptr::addr_of_mut!(xyz),
                core::ptr::addr_of!((*png_ptr).chromaticities),
            ) == 0
        {
            /* png_set_rgb_to_gray has not set the coefficients, get them from the
             * Y * values of the colorspace colorants.
             */
            let mut r: png_fixed_point = xyz.red_Y;
            let mut g: png_fixed_point = xyz.green_Y;
            let mut b: png_fixed_point = xyz.blue_Y;
            let total: png_fixed_point = r.wrapping_add(g).wrapping_add(b);

            if total > 0
                && r >= 0
                && png_muldiv(core::ptr::addr_of_mut!(r), r, 32768, total) != 0
                && r >= 0
                && r <= 32768
                && g >= 0
                && png_muldiv(core::ptr::addr_of_mut!(g), g, 32768, total) != 0
                && g >= 0
                && g <= 32768
                && b >= 0
                && png_muldiv(core::ptr::addr_of_mut!(b), b, 32768, total) != 0
                && b >= 0
                && b <= 32768
                && r.wrapping_add(g).wrapping_add(b) <= 32769
            {
                /* We allow 0 coefficients here.  r+g+b may be 32769 if two or
                 * all of the coefficients were rounded up.  Handle this by
                 * reducing the *largest* coefficient by 1; this matches the
                 * approach used for the default coefficients in pngrtran.c
                 */
                let mut add: c_int = 0;

                if r.wrapping_add(g).wrapping_add(b) > 32768 {
                    add = -1;
                } else if r.wrapping_add(g).wrapping_add(b) < 32768 {
                    add = 1;
                }

                if add != 0 {
                    if g >= r && g >= b {
                        g = g.wrapping_add(add);
                    } else if r >= g && r >= b {
                        r = r.wrapping_add(add);
                    } else {
                        b = b.wrapping_add(add);
                    }
                }

                /* Check for an internal error. */
                if r.wrapping_add(g).wrapping_add(b) != 32768 {
                    png_error(
                        png_ptr,
                        b"internal error handling cHRM coefficients\0".as_ptr()
                            as png_const_charp,
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
