// pngrutil.c part A2 (lines 901-2712)

// LZ77Min (pngrutil.c): minimum zlib stream length (2 byte header + 5 byte
// minimum deflate stream + 4 byte checksum).
const LZ77Min: uInt = 2 + 5 + 4;

// PNG_INDEX_ values for png_has_chunk (pngstruct.h / pngpriv.h).
const PNG_INDEX_bKGD: c_int = 5;
const PNG_INDEX_cICP: c_int = 7;
const PNG_INDEX_mDCV: c_int = 16;
const PNG_INDEX_tRNS: c_int = 26;

// png_has_chunk(png_ptr, cHNK) == png_file_has_chunk(png_ptr, PNG_INDEX_##cHNK)
// == ((png_ptr)->chunks & (0x80000000U >> (31 - i))) != 0  (pngstruct.h).
#[inline]
unsafe fn a2_has_chunk(png_ptr: png_const_structrp, i: c_int) -> bool {
    ((*png_ptr).chunks & (0x80000000u32 >> (31 - i))) != 0
}

pub(crate) unsafe fn png_handle_IHDR(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buf = [0u8; 13];
    let width: png_uint_32;
    let height: png_uint_32;
    let bit_depth: c_int;
    let color_type: c_int;
    let compression_type: c_int;
    let filter_type: c_int;
    let interlace_type: c_int;

    /* Length and position are checked by the caller. */

    (*png_ptr).mode |= PNG_HAVE_IHDR;

    png_crc_read(png_ptr, buf.as_mut_ptr(), 13);
    png_crc_finish(png_ptr, 0);

    width = png_get_uint_31(png_ptr, buf.as_ptr());
    height = png_get_uint_31(png_ptr, buf.as_ptr().add(4));
    bit_depth = buf[8] as c_int;
    color_type = buf[9] as c_int;
    compression_type = buf[10] as c_int;
    filter_type = buf[11] as c_int;
    interlace_type = buf[12] as c_int;

    /* Set internal variables */
    (*png_ptr).width = width;
    (*png_ptr).height = height;
    (*png_ptr).bit_depth = bit_depth as png_byte;
    (*png_ptr).interlaced = interlace_type as png_byte;
    (*png_ptr).color_type = color_type as png_byte;
    (*png_ptr).filter_type = filter_type as png_byte;
    (*png_ptr).compression_type = compression_type as png_byte;

    /* Find number of channels */
    match (*png_ptr).color_type as c_int {
        // default / PNG_COLOR_TYPE_GRAY / PNG_COLOR_TYPE_PALETTE
        x if x == PNG_COLOR_TYPE_RGB => {
            (*png_ptr).channels = 3;
        }
        x if x == PNG_COLOR_TYPE_GRAY_ALPHA => {
            (*png_ptr).channels = 2;
        }
        x if x == PNG_COLOR_TYPE_RGB_ALPHA => {
            (*png_ptr).channels = 4;
        }
        _ => {
            (*png_ptr).channels = 1;
        }
    }

    /* Set up other useful info */
    (*png_ptr).pixel_depth = ((*png_ptr).bit_depth as c_int * (*png_ptr).channels as c_int) as png_byte;
    (*png_ptr).rowbytes = png_rowbytes((*png_ptr).pixel_depth as u32, (*png_ptr).width);

    /* Rely on png_set_IHDR to completely validate the data and call png_error if
     * it's wrong.
     */
    png_set_IHDR(
        png_ptr,
        info_ptr,
        width,
        height,
        bit_depth,
        color_type,
        interlace_type,
        compression_type,
        filter_type,
    );

    let _ = length;
    handled_ok
}

/* Read and check the palette */
pub(crate) unsafe fn png_handle_PLTE(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut errmsg: png_const_charp = ptr::null();

    if ((*png_ptr).mode & PNG_HAVE_PLTE) != 0 {
        errmsg = c"duplicate".as_ptr();
    } else if ((*png_ptr).mode & PNG_HAVE_IDAT) != 0 {
        errmsg = c"out of place".as_ptr();
    } else if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0 {
        errmsg = c"ignored in grayscale PNG".as_ptr();
    } else if length > 3 * PNG_MAX_PALETTE_LENGTH as png_uint_32 || (length % 3) != 0 {
        errmsg = c"invalid".as_ptr();
    } else if (*png_ptr).color_type as c_int != PNG_COLOR_TYPE_PALETTE
        && (a2_has_chunk(png_ptr, PNG_INDEX_tRNS) || a2_has_chunk(png_ptr, PNG_INDEX_bKGD))
    {
        errmsg = c"out of place".as_ptr();
    } else {
        /* If the palette has 256 or fewer entries but is too large for the bit
         * depth we don't issue an error to preserve the behavior of previous
         * libpng versions.
         */
        let max_palette_length: c_uint = if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            1u32 << (*png_ptr).bit_depth
        } else {
            PNG_MAX_PALETTE_LENGTH as c_uint
        };

        let num: c_uint = if length > 3u32.wrapping_mul(max_palette_length) {
            max_palette_length
        } else {
            length / 3
        };

        let mut i: c_uint;
        let mut j: c_uint;
        let mut buf = [0u8; 3 * PNG_MAX_PALETTE_LENGTH as usize];
        let mut palette = [png_color {
            red: 0,
            green: 0,
            blue: 0,
        }; PNG_MAX_PALETTE_LENGTH as usize];

        /* Read the chunk into the buffer then read to the end of the chunk. */
        png_crc_read(png_ptr, buf.as_mut_ptr(), num * 3);
        png_crc_finish_critical(
            png_ptr,
            length - 3 * num,
            /* Handle as ancillary if PLTE is optional: */
            ((*png_ptr).color_type as c_int != PNG_COLOR_TYPE_PALETTE) as c_int,
        );

        i = 0;
        j = 0;
        while i < num {
            palette[i as usize].red = buf[j as usize];
            j += 1;
            palette[i as usize].green = buf[j as usize];
            j += 1;
            palette[i as usize].blue = buf[j as usize];
            j += 1;
            i += 1;
        }

        /* A valid PLTE chunk has been read */
        (*png_ptr).mode |= PNG_HAVE_PLTE;

        png_set_PLTE(png_ptr, info_ptr, palette.as_ptr(), num as c_int);
        return handled_ok;
    }

    /* Here on error: errmsg is non NULL. */
    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        png_crc_finish(png_ptr, length);
        png_chunk_error(png_ptr, errmsg);
    } else {
        /* not critical to this image */
        png_crc_finish_critical(png_ptr, length, 1 /*handle as ancillary*/);
        png_chunk_benign_error(png_ptr, errmsg);
    }

    if !errmsg.is_null() {
        handled_error
    } else {
        handled_error
    }
}

pub(crate) unsafe fn png_handle_IEND(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    (*png_ptr).mode |= PNG_AFTER_IDAT | PNG_HAVE_IEND;

    if length != 0 {
        png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
    }

    png_crc_finish_critical(png_ptr, length, 1 /*handle as ancillary*/);

    let _ = info_ptr;
    handled_ok
}

pub(crate) unsafe fn png_handle_gAMA(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let ugamma: png_uint_32;
    let mut buf = [0u8; 4];

    png_crc_read(png_ptr, buf.as_mut_ptr(), 4);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    ugamma = png_get_uint_32(buf.as_ptr());

    if ugamma > PNG_UINT_31_MAX {
        png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
        return handled_error;
    }

    png_set_gAMA_fixed(png_ptr, info_ptr, ugamma as png_fixed_point /*SAFE*/);

    /* PNGv3: chunk precedence for gamma is cICP, [iCCP], sRGB, gAMA.  gAMA is
     * at the end of the chain so simply check for an unset value.
     */
    if (*png_ptr).chunk_gamma == 0 {
        (*png_ptr).chunk_gamma = ugamma as png_fixed_point /*SAFE*/;
    }

    let _ = length;
    handled_ok
}

pub(crate) unsafe fn png_handle_sBIT(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let truelen: c_uint;
    let mut i: c_uint;
    let sample_depth: png_byte;
    let mut buf = [0u8; 4];

    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        truelen = 3;
        sample_depth = 8;
    } else {
        truelen = (*png_ptr).channels as c_uint;
        sample_depth = (*png_ptr).bit_depth;
    }

    if length != truelen {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"bad length".as_ptr());
        return handled_error;
    }

    buf[0] = sample_depth;
    buf[1] = sample_depth;
    buf[2] = sample_depth;
    buf[3] = sample_depth;
    png_crc_read(png_ptr, buf.as_mut_ptr(), truelen);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    i = 0;
    while i < truelen {
        if buf[i as usize] == 0 || buf[i as usize] > sample_depth {
            png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
            return handled_error;
        }
        i += 1;
    }

    if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        (*png_ptr).sig_bit.red = buf[0];
        (*png_ptr).sig_bit.green = buf[1];
        (*png_ptr).sig_bit.blue = buf[2];
        (*png_ptr).sig_bit.alpha = buf[3];
    } else {
        /* grayscale */
        (*png_ptr).sig_bit.gray = buf[0];
        (*png_ptr).sig_bit.red = buf[0];
        (*png_ptr).sig_bit.green = buf[0];
        (*png_ptr).sig_bit.blue = buf[0];
        (*png_ptr).sig_bit.alpha = buf[1];
    }

    png_set_sBIT(png_ptr, info_ptr, &(*png_ptr).sig_bit);
    handled_ok
}

pub(crate) unsafe fn png_get_int_32_checked(buf: png_const_bytep, error: *mut c_int) -> png_int_32 {
    let mut uval = png_get_uint_32(buf);
    if (uval & 0x80000000) == 0 {
        /* non-negative */
        return uval as png_int_32;
    }

    uval = (uval ^ 0xffffffff) + 1; /* 2's complement: -x = ~x+1 */
    if (uval & 0x80000000) == 0 {
        /* no overflow */
        return -(uval as png_int_32);
    }

    /* This version of png_get_int_32 has a way of returning the error to the
     * caller, so:
     */
    *error = 1;
    0 /* Safe */
}

pub(crate) unsafe fn png_handle_cHRM(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut error: c_int = 0;
    let mut xy = png_xy::default();
    let mut buf = [0u8; 32];

    png_crc_read(png_ptr, buf.as_mut_ptr(), 32);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    xy.whitex = png_get_int_32_checked(buf.as_ptr().add(0), &mut error);
    xy.whitey = png_get_int_32_checked(buf.as_ptr().add(4), &mut error);
    xy.redx = png_get_int_32_checked(buf.as_ptr().add(8), &mut error);
    xy.redy = png_get_int_32_checked(buf.as_ptr().add(12), &mut error);
    xy.greenx = png_get_int_32_checked(buf.as_ptr().add(16), &mut error);
    xy.greeny = png_get_int_32_checked(buf.as_ptr().add(20), &mut error);
    xy.bluex = png_get_int_32_checked(buf.as_ptr().add(24), &mut error);
    xy.bluey = png_get_int_32_checked(buf.as_ptr().add(28), &mut error);

    if error != 0 {
        png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
        return handled_error;
    }

    png_set_cHRM_fixed(
        png_ptr, info_ptr, xy.whitex, xy.whitey, xy.redx, xy.redy, xy.greenx, xy.greeny, xy.bluex,
        xy.bluey,
    );

    /* We only use 'chromaticities' for RGB to gray */
    if !a2_has_chunk(png_ptr, PNG_INDEX_mDCV) {
        (*png_ptr).chromaticities = xy;
    }

    let _ = length;
    handled_ok
}

pub(crate) unsafe fn png_handle_sRGB(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut intent: png_byte = 0;

    png_crc_read(png_ptr, &mut intent, 1);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    if intent > 3 /*PNGv3 spec*/ {
        png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
        return handled_error;
    }

    png_set_sRGB(png_ptr, info_ptr, intent as c_int);

    /* PNGv3: chunk precedence for gamma is cICP, [iCCP], sRGB, gAMA.  iCCP is
     * not supported by libpng so the only requirement is to check for cICP
     * setting the gamma (this is NYI, but this check is safe.)
     */
    if !a2_has_chunk(png_ptr, PNG_INDEX_cICP) || (*png_ptr).chunk_gamma == 0 {
        (*png_ptr).chunk_gamma = PNG_GAMMA_sRGB_INVERSE;
    }

    let _ = length;
    handled_ok
}

pub(crate) unsafe fn png_handle_iCCP(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    mut length: png_uint_32,
) -> png_handle_result_code {
    let mut errmsg: png_const_charp = ptr::null(); /* error message output, or no error */
    let mut finished: c_int = 0; /* crc checked */

    /* PNGv3: allow PNG files with both sRGB and iCCP. */
    {
        let mut read_length: uInt;
        let mut keyword_length: uInt;
        let mut keyword = [0 as c_char; 81];

        /* Find the keyword; the keyword plus separator and compression method
         * bytes can be at most 81 characters long.
         */
        read_length = 81; /* maximum */
        if read_length as png_uint_32 > length {
            read_length = length as uInt /*SAFE*/;
        }

        png_crc_read(png_ptr, keyword.as_mut_ptr() as png_bytep, read_length as png_uint_32);
        length -= read_length as png_uint_32;

        if length < LZ77Min as png_uint_32 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"too short".as_ptr());
            return handled_error;
        }

        keyword_length = 0;
        while keyword_length < 80
            && keyword_length < read_length
            && keyword[keyword_length as usize] != 0
        {
            keyword_length += 1;
        }

        /* TODO: make the keyword checking common */
        if keyword_length >= 1 && keyword_length <= 79 {
            /* We only understand '0' compression - deflate. */
            if keyword_length + 1 < read_length
                && keyword[(keyword_length + 1) as usize] == PNG_COMPRESSION_TYPE_BASE as c_char
            {
                read_length -= keyword_length + 2;

                if png_inflate_claim(png_ptr, png_iCCP) == Z_OK {
                    let mut profile_header = [0u8; 132];
                    let mut local_buffer = [0u8; PNG_INFLATE_BUF_SIZE];
                    let mut size: png_alloc_size_t = core::mem::size_of_val(&profile_header);

                    (*png_ptr).zstream.next_in =
                        (keyword.as_ptr() as *const Bytef).add((keyword_length + 2) as usize);
                    (*png_ptr).zstream.avail_in = read_length;
                    png_inflate_read(
                        png_ptr,
                        local_buffer.as_mut_ptr(),
                        local_buffer.len() as uInt,
                        &mut length,
                        profile_header.as_mut_ptr(),
                        &mut size,
                        0, /*finish: don't, because the output is too small*/
                    );

                    if size == 0 {
                        /* We have the ICC profile header; do the basic header checks. */
                        let profile_length = png_get_uint_32(profile_header.as_ptr());

                        if png_icc_check_length(png_ptr, keyword.as_ptr(), profile_length) != 0 {
                            /* The length is apparently ok, so we can check the 132
                             * byte header.
                             */
                            if png_icc_check_header(
                                png_ptr,
                                keyword.as_ptr(),
                                profile_length,
                                profile_header.as_ptr(),
                                (*png_ptr).color_type as c_int,
                            ) != 0
                            {
                                /* Now read the tag table. */
                                let tag_count = png_get_uint_32(profile_header.as_ptr().add(128));
                                let profile = png_read_buffer(png_ptr, profile_length as png_alloc_size_t);

                                if !profile.is_null() {
                                    memcpy(
                                        profile as *mut c_void,
                                        profile_header.as_ptr() as *const c_void,
                                        core::mem::size_of_val(&profile_header),
                                    );

                                    size = (12u32.wrapping_mul(tag_count)) as png_alloc_size_t;

                                    png_inflate_read(
                                        png_ptr,
                                        local_buffer.as_mut_ptr(),
                                        local_buffer.len() as uInt,
                                        &mut length,
                                        profile.add(core::mem::size_of_val(&profile_header)),
                                        &mut size,
                                        0,
                                    );

                                    /* Still expect a buffer error because we expect
                                     * there to be some tag data!
                                     */
                                    if size == 0 {
                                        if png_icc_check_tag_table(
                                            png_ptr,
                                            keyword.as_ptr(),
                                            profile_length,
                                            profile,
                                        ) != 0
                                        {
                                            /* The profile has been validated. */
                                            size = profile_length as png_alloc_size_t
                                                - core::mem::size_of_val(&profile_header)
                                                - (12u32.wrapping_mul(tag_count)) as png_alloc_size_t;

                                            png_inflate_read(
                                                png_ptr,
                                                local_buffer.as_mut_ptr(),
                                                local_buffer.len() as uInt,
                                                &mut length,
                                                profile
                                                    .add(core::mem::size_of_val(&profile_header))
                                                    .add((12u32.wrapping_mul(tag_count)) as usize),
                                                &mut size,
                                                1, /*finish*/
                                            );

                                            if length > 0
                                                && ((*png_ptr).flags & PNG_FLAG_BENIGN_ERRORS_WARN)
                                                    == 0
                                            {
                                                errmsg = c"extra compressed data".as_ptr();
                                            }
                                            /* But otherwise allow extra data: */
                                            else if size == 0 {
                                                if length > 0 {
                                                    /* This can be handled completely, so
                                                     * keep going.
                                                     */
                                                    png_chunk_warning(
                                                        png_ptr,
                                                        c"extra compressed data".as_ptr(),
                                                    );
                                                }

                                                png_crc_finish(png_ptr, length);
                                                finished = 1;

                                                /* Steal the profile for info_ptr. */
                                                if !info_ptr.is_null() {
                                                    png_free_data(
                                                        png_ptr,
                                                        info_ptr,
                                                        PNG_FREE_ICCP,
                                                        0,
                                                    );

                                                    (*info_ptr).iccp_name = png_malloc_base(
                                                        png_ptr,
                                                        (keyword_length + 1) as png_alloc_size_t,
                                                    )
                                                        as *mut c_char;
                                                    if !(*info_ptr).iccp_name.is_null() {
                                                        memcpy(
                                                            (*info_ptr).iccp_name as *mut c_void,
                                                            keyword.as_ptr() as *const c_void,
                                                            (keyword_length + 1) as size_t,
                                                        );
                                                        (*info_ptr).iccp_proflen = profile_length;
                                                        (*info_ptr).iccp_profile = profile;
                                                        (*png_ptr).read_buffer = ptr::null_mut(); /*steal*/
                                                        (*info_ptr).free_me |= PNG_FREE_ICCP;
                                                        (*info_ptr).valid |= PNG_INFO_iCCP;
                                                    } else {
                                                        errmsg = c"out of memory".as_ptr();
                                                    }
                                                }

                                                /* else the profile remains in the read
                                                 * buffer which gets reused.
                                                 */

                                                if errmsg.is_null() {
                                                    (*png_ptr).zowner = 0;
                                                    return handled_ok;
                                                }
                                            }
                                            if errmsg.is_null() {
                                                errmsg = (*png_ptr).zstream.msg;
                                            }
                                        }
                                        /* else png_icc_check_tag_table output an error */
                                    } else {
                                        /* profile truncated */
                                        errmsg = (*png_ptr).zstream.msg;
                                    }
                                } else {
                                    errmsg = c"out of memory".as_ptr();
                                }
                            }
                            /* else png_icc_check_header output an error */
                        }
                        /* else png_icc_check_length output an error */
                    } else {
                        /* profile truncated */
                        errmsg = (*png_ptr).zstream.msg;
                    }

                    /* Release the stream */
                    (*png_ptr).zowner = 0;
                } else {
                    /* png_inflate_claim failed */
                    errmsg = (*png_ptr).zstream.msg;
                }
            } else {
                errmsg = c"bad compression method".as_ptr(); /* or missing */
            }
        } else {
            errmsg = c"bad keyword".as_ptr();
        }
    }

    /* Failure: the reason is in 'errmsg' */
    if finished == 0 {
        png_crc_finish(png_ptr, length);
    }

    if !errmsg.is_null() {
        /* else already output */
        png_chunk_benign_error(png_ptr, errmsg);
    }

    handled_error
}

pub(crate) unsafe fn png_handle_sPLT(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let buffer: png_bytep;
    let mut entry_start: png_bytep;
    let mut new_palette: png_sPLT_t = png_sPLT_t {
        name: ptr::null_mut(),
        depth: 0,
        entries: ptr::null_mut(),
        nentries: 0,
    };
    let mut pp: png_sPLT_entryp;
    let data_length: png_uint_32;
    let entry_size: c_int;
    let mut i: c_int;
    let skip: png_uint_32 = 0;
    let dl: png_uint_32;
    let max_dl: size_t;

    if (*png_ptr).user_chunk_cache_max != 0 {
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            return handled_error;
        }

        (*png_ptr).user_chunk_cache_max -= 1;
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_warning(png_ptr, c"No space in chunk cache for sPLT".as_ptr());
            png_crc_finish(png_ptr, length);
            return handled_error;
        }
    }

    buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);
    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, skip) != 0 {
        return handled_error;
    }

    *buffer.offset(length as isize) = 0;

    entry_start = buffer;
    while *entry_start != 0 {
        entry_start = entry_start.add(1);
    }

    entry_start = entry_start.add(1);

    /* A sample depth should follow the separator, and we should be on it  */
    if length < 2 || entry_start > buffer.offset((length - 2) as isize) {
        png_warning(png_ptr, c"malformed sPLT chunk".as_ptr());
        return handled_error;
    }

    new_palette.depth = *entry_start;
    entry_start = entry_start.add(1);
    entry_size = if new_palette.depth == 8 { 6 } else { 10 };
    data_length = length - (entry_start.offset_from(buffer) as png_uint_32);

    /* Integrity-check the data length */
    if (data_length % entry_size as c_uint) != 0 {
        png_warning(png_ptr, c"sPLT chunk has bad length".as_ptr());
        return handled_error;
    }

    dl = data_length / entry_size as c_uint;
    max_dl = PNG_SIZE_MAX / core::mem::size_of::<png_sPLT_entry>();

    if dl as size_t > max_dl {
        png_warning(png_ptr, c"sPLT chunk too long".as_ptr());
        return handled_error;
    }

    new_palette.nentries = (data_length / entry_size as c_uint) as png_int_32;

    new_palette.entries = png_malloc_warn(
        png_ptr,
        new_palette.nentries as png_alloc_size_t * core::mem::size_of::<png_sPLT_entry>(),
    ) as png_sPLT_entryp;

    if new_palette.entries.is_null() {
        png_warning(png_ptr, c"sPLT chunk requires too much memory".as_ptr());
        return handled_error;
    }

    i = 0;
    while i < new_palette.nentries {
        pp = new_palette.entries.offset(i as isize);

        if new_palette.depth == 8 {
            (*pp).red = *entry_start as png_uint_16;
            entry_start = entry_start.add(1);
            (*pp).green = *entry_start as png_uint_16;
            entry_start = entry_start.add(1);
            (*pp).blue = *entry_start as png_uint_16;
            entry_start = entry_start.add(1);
            (*pp).alpha = *entry_start as png_uint_16;
            entry_start = entry_start.add(1);
        } else {
            (*pp).red = png_get_uint_16(entry_start);
            entry_start = entry_start.add(2);
            (*pp).green = png_get_uint_16(entry_start);
            entry_start = entry_start.add(2);
            (*pp).blue = png_get_uint_16(entry_start);
            entry_start = entry_start.add(2);
            (*pp).alpha = png_get_uint_16(entry_start);
            entry_start = entry_start.add(2);
        }

        (*pp).frequency = png_get_uint_16(entry_start);
        entry_start = entry_start.add(2);
        i += 1;
    }

    /* Discard all chunk data except the name and stash that */
    new_palette.name = buffer as png_charp;

    png_set_sPLT(png_ptr, info_ptr, &new_palette, 1);

    png_free(png_ptr, new_palette.entries as png_voidp);
    handled_ok
}

pub(crate) unsafe fn png_handle_tRNS(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut readbuf = [0u8; PNG_MAX_PALETTE_LENGTH as usize];

    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY {
        let mut buf = [0u8; 2];

        if length != 2 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
            return handled_error;
        }

        png_crc_read(png_ptr, buf.as_mut_ptr(), 2);
        (*png_ptr).num_trans = 1;
        (*png_ptr).trans_color.gray = png_get_uint_16(buf.as_ptr());
    } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB {
        let mut buf = [0u8; 6];

        if length != 6 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
            return handled_error;
        }

        png_crc_read(png_ptr, buf.as_mut_ptr(), length);
        (*png_ptr).num_trans = 1;
        (*png_ptr).trans_color.red = png_get_uint_16(buf.as_ptr());
        (*png_ptr).trans_color.green = png_get_uint_16(buf.as_ptr().add(2));
        (*png_ptr).trans_color.blue = png_get_uint_16(buf.as_ptr().add(4));
    } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        if ((*png_ptr).mode & PNG_HAVE_PLTE) == 0 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"out of place".as_ptr());
            return handled_error;
        }

        if length > (*png_ptr).num_palette as c_uint
            || length > PNG_MAX_PALETTE_LENGTH as c_uint
            || length == 0
        {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
            return handled_error;
        }

        png_crc_read(png_ptr, readbuf.as_mut_ptr(), length);
        (*png_ptr).num_trans = length as png_uint_16;
    } else {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"invalid with alpha channel".as_ptr());
        return handled_error;
    }

    if png_crc_finish(png_ptr, 0) != 0 {
        (*png_ptr).num_trans = 0;
        return handled_error;
    }

    png_set_tRNS(
        png_ptr,
        info_ptr,
        readbuf.as_ptr(),
        (*png_ptr).num_trans as c_int,
        &(*png_ptr).trans_color,
    );
    handled_ok
}

pub(crate) unsafe fn png_handle_bKGD(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let truelen: c_uint;
    let mut buf = [0u8; 6];
    let mut background = png_color_16 {
        index: 0,
        red: 0,
        green: 0,
        blue: 0,
        gray: 0,
    };

    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        if ((*png_ptr).mode & PNG_HAVE_PLTE) == 0 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"out of place".as_ptr());
            return handled_error;
        }

        truelen = 1;
    } else if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        truelen = 6;
    } else {
        truelen = 2;
    }

    if length != truelen {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
        return handled_error;
    }

    png_crc_read(png_ptr, buf.as_mut_ptr(), truelen);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        background.index = buf[0];

        if !info_ptr.is_null() && (*info_ptr).num_palette != 0 {
            if buf[0] as png_uint_16 >= (*info_ptr).num_palette {
                png_chunk_benign_error(png_ptr, c"invalid index".as_ptr());
                return handled_error;
            }

            background.red = (*(*png_ptr).palette.offset(buf[0] as isize)).red as png_uint_16;
            background.green = (*(*png_ptr).palette.offset(buf[0] as isize)).green as png_uint_16;
            background.blue = (*(*png_ptr).palette.offset(buf[0] as isize)).blue as png_uint_16;
        } else {
            background.red = 0;
            background.green = 0;
            background.blue = 0;
        }

        background.gray = 0;
    } else if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0 {
        /* GRAY */
        if (*png_ptr).bit_depth <= 8 {
            if buf[0] != 0 || buf[1] as c_uint >= (1u32 << (*png_ptr).bit_depth) {
                png_chunk_benign_error(png_ptr, c"invalid gray level".as_ptr());
                return handled_error;
            }
        }

        background.index = 0;
        let v = png_get_uint_16(buf.as_ptr());
        background.red = v;
        background.green = v;
        background.blue = v;
        background.gray = v;
    } else {
        if (*png_ptr).bit_depth <= 8 {
            if buf[0] != 0 || buf[2] != 0 || buf[4] != 0 {
                png_chunk_benign_error(png_ptr, c"invalid color".as_ptr());
                return handled_error;
            }
        }

        background.index = 0;
        background.red = png_get_uint_16(buf.as_ptr());
        background.green = png_get_uint_16(buf.as_ptr().add(2));
        background.blue = png_get_uint_16(buf.as_ptr().add(4));
        background.gray = 0;
    }

    png_set_bKGD(png_ptr, info_ptr, &background);
    handled_ok
}

pub(crate) unsafe fn png_handle_cICP(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buf = [0u8; 4];

    png_crc_read(png_ptr, buf.as_mut_ptr(), 4);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    png_set_cICP(png_ptr, info_ptr, buf[0], buf[1], buf[2], buf[3]);

    /* We only use 'chromaticities' for RGB to gray */
    if !a2_has_chunk(png_ptr, PNG_INDEX_mDCV) {
        /* TODO: png_ptr->chromaticities = chromaticities; */
    }

    let _ = length;
    handled_ok
}

pub(crate) unsafe fn png_handle_cLLI(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buf = [0u8; 8];

    png_crc_read(png_ptr, buf.as_mut_ptr(), 8);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* The error checking happens here, this puts it in just one place: */
    png_set_cLLI_fixed(
        png_ptr,
        info_ptr,
        png_get_uint_32(buf.as_ptr()),
        png_get_uint_32(buf.as_ptr().add(4)),
    );
    let _ = length;
    handled_ok
}

pub(crate) unsafe fn png_handle_mDCV(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut chromaticities = png_xy::default();
    let mut buf = [0u8; 24];

    png_crc_read(png_ptr, buf.as_mut_ptr(), 24);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    chromaticities.redx = (png_get_uint_16(buf.as_ptr().add(0)) as png_fixed_point) << 1; /* red x */
    chromaticities.redy = (png_get_uint_16(buf.as_ptr().add(2)) as png_fixed_point) << 1; /* red y */
    chromaticities.greenx = (png_get_uint_16(buf.as_ptr().add(4)) as png_fixed_point) << 1; /* green x */
    chromaticities.greeny = (png_get_uint_16(buf.as_ptr().add(6)) as png_fixed_point) << 1; /* green y */
    chromaticities.bluex = (png_get_uint_16(buf.as_ptr().add(8)) as png_fixed_point) << 1; /* blue x */
    chromaticities.bluey = (png_get_uint_16(buf.as_ptr().add(10)) as png_fixed_point) << 1; /* blue y */
    chromaticities.whitex = (png_get_uint_16(buf.as_ptr().add(12)) as png_fixed_point) << 1; /* white x */
    chromaticities.whitey = (png_get_uint_16(buf.as_ptr().add(14)) as png_fixed_point) << 1; /* white y */

    png_set_mDCV_fixed(
        png_ptr,
        info_ptr,
        chromaticities.whitex,
        chromaticities.whitey,
        chromaticities.redx,
        chromaticities.redy,
        chromaticities.greenx,
        chromaticities.greeny,
        chromaticities.bluex,
        chromaticities.bluey,
        png_get_uint_32(buf.as_ptr().add(16)), /* peak luminance */
        png_get_uint_32(buf.as_ptr().add(20)), /* minimum perceivable luminance */
    );

    /* We only use 'chromaticities' for RGB to gray */
    (*png_ptr).chromaticities = chromaticities;

    let _ = length;
    handled_ok
}

pub(crate) unsafe fn png_handle_eXIf(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let buffer: png_bytep;

    buffer = png_read_buffer(png_ptr, length as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* PNGv3: the first 4 bytes should be checked. */
    {
        let header = png_get_uint_32(buffer);

        /* These numbers are copied from the PNGv3 spec: */
        if header != 0x49492A00 && header != 0x4D4D002A {
            png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
            return handled_error;
        }
    }

    png_set_eXIf_1(png_ptr, info_ptr, length, buffer);
    handled_ok
}

pub(crate) unsafe fn png_handle_hIST(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let num: c_uint;
    let mut i: c_uint;
    let mut readbuf = [0u16; PNG_MAX_PALETTE_LENGTH as usize];

    num = length / 2;

    if length != num * 2
        || num != (*png_ptr).num_palette as c_uint
        || num > PNG_MAX_PALETTE_LENGTH as c_uint
    {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
        return handled_error;
    }

    i = 0;
    while i < num {
        let mut buf = [0u8; 2];

        png_crc_read(png_ptr, buf.as_mut_ptr(), 2);
        readbuf[i as usize] = png_get_uint_16(buf.as_ptr());
        i += 1;
    }

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    png_set_hIST(png_ptr, info_ptr, readbuf.as_ptr());
    handled_ok
}

pub(crate) unsafe fn png_handle_pHYs(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buf = [0u8; 9];
    let res_x: png_uint_32;
    let res_y: png_uint_32;
    let unit_type: c_int;

    png_crc_read(png_ptr, buf.as_mut_ptr(), 9);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    res_x = png_get_uint_32(buf.as_ptr());
    res_y = png_get_uint_32(buf.as_ptr().add(4));
    unit_type = buf[8] as c_int;
    png_set_pHYs(png_ptr, info_ptr, res_x, res_y, unit_type);
    let _ = length;
    handled_ok
}

pub(crate) unsafe fn png_handle_oFFs(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buf = [0u8; 9];
    let offset_x: png_int_32;
    let offset_y: png_int_32;
    let unit_type: c_int;

    png_crc_read(png_ptr, buf.as_mut_ptr(), 9);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    offset_x = png_get_int_32(buf.as_ptr());
    offset_y = png_get_int_32(buf.as_ptr().add(4));
    unit_type = buf[8] as c_int;
    png_set_oFFs(png_ptr, info_ptr, offset_x, offset_y, unit_type);
    let _ = length;
    handled_ok
}

/* Read the pCAL chunk (described in the PNG Extensions document) */
pub(crate) unsafe fn png_handle_pCAL(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let buffer: png_bytep;
    let mut buf: png_bytep;
    let endptr: png_bytep;
    let X0: png_int_32;
    let X1: png_int_32;
    let type_: png_byte;
    let nparams: png_byte;
    let units: png_bytep;
    let params: png_charpp;
    let mut i: c_int;

    buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    *buffer.offset(length as isize) = 0; /* Null terminate the last string */

    buf = buffer;
    while *buf != 0 {
        buf = buf.add(1);
    }

    endptr = buffer.offset(length as isize);

    /* We need to have at least 12 bytes after the purpose string
     * in order to get the parameter information.
     */
    if endptr.offset_from(buf) <= 12 {
        png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
        return handled_error;
    }

    X0 = png_get_int_32(buf.add(1));
    X1 = png_get_int_32(buf.add(5));
    type_ = *buf.add(9);
    nparams = *buf.add(10);
    units = buf.add(11);

    /* Check that we have the right number of parameters for known
     * equation types.
     */
    if (type_ as c_int == PNG_EQUATION_LINEAR && nparams != 2)
        || (type_ as c_int == PNG_EQUATION_BASE_E && nparams != 3)
        || (type_ as c_int == PNG_EQUATION_ARBITRARY && nparams != 3)
        || (type_ as c_int == PNG_EQUATION_HYPERBOLIC && nparams != 4)
    {
        png_chunk_benign_error(png_ptr, c"invalid parameter count".as_ptr());
        return handled_error;
    } else if type_ as c_int >= PNG_EQUATION_LAST {
        png_chunk_benign_error(png_ptr, c"unrecognized equation type".as_ptr());
    }

    buf = units;
    while *buf != 0 {
        buf = buf.add(1);
    }

    params = png_malloc_warn(
        png_ptr,
        nparams as png_alloc_size_t * core::mem::size_of::<png_charp>(),
    ) as png_charpp;

    if params.is_null() {
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
        return handled_error;
    }

    /* Get pointers to the start of each parameter string. */
    i = 0;
    while i < nparams as c_int {
        buf = buf.add(1); /* Skip the null string terminator from previous parameter. */

        *params.offset(i as isize) = buf as png_charp;
        while buf <= endptr && *buf != 0 {
            buf = buf.add(1);
        }

        /* Make sure we haven't run out of data yet */
        if buf > endptr {
            png_free(png_ptr, params as png_voidp);
            png_chunk_benign_error(png_ptr, c"invalid data".as_ptr());
            return handled_error;
        }
        i += 1;
    }

    png_set_pCAL(
        png_ptr,
        info_ptr,
        buffer as png_charp,
        X0,
        X1,
        type_ as c_int,
        nparams as c_int,
        units as png_charp,
        params,
    );

    png_free(png_ptr, params as png_voidp);
    handled_ok
}

/* Read the sCAL chunk */
pub(crate) unsafe fn png_handle_sCAL(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let buffer: png_bytep;
    let mut i: size_t;
    let mut state: c_int;

    buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);
    *buffer.offset(length as isize) = 0; /* Null terminate the last string */

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* Validate the unit. */
    if *buffer.offset(0) != 1 && *buffer.offset(0) != 2 {
        png_chunk_benign_error(png_ptr, c"invalid unit".as_ptr());
        return handled_error;
    }

    /* Validate the ASCII numbers, need two ASCII numbers separated by
     * a '\0' and they need to fit exactly in the chunk data.
     */
    i = 1;
    state = 0;

    if png_check_fp_number(buffer as png_const_charp, length as size_t, &mut state, &mut i) == 0
        || i >= length as size_t
        || {
            let v = *buffer.offset(i as isize);
            i += 1;
            v != 0
        }
    {
        png_chunk_benign_error(png_ptr, c"bad width format".as_ptr());
    } else if (state & PNG_FP_NZ_MASK) != PNG_FP_Z_MASK {
        png_chunk_benign_error(png_ptr, c"non-positive width".as_ptr());
    } else {
        let heighti = i;

        state = 0;
        if png_check_fp_number(buffer as png_const_charp, length as size_t, &mut state, &mut i) == 0
            || i != length as size_t
        {
            png_chunk_benign_error(png_ptr, c"bad height format".as_ptr());
        } else if (state & PNG_FP_NZ_MASK) != PNG_FP_Z_MASK {
            png_chunk_benign_error(png_ptr, c"non-positive height".as_ptr());
        } else {
            /* This is the (only) success case. */
            png_set_sCAL_s(
                png_ptr,
                info_ptr,
                *buffer.offset(0) as c_int,
                buffer.offset(1) as png_charp,
                buffer.offset(heighti as isize) as png_charp,
            );
            return handled_ok;
        }
    }

    handled_error
}

pub(crate) unsafe fn png_handle_tIME(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buf = [0u8; 7];
    let mut mod_time = png_time {
        year: 0,
        month: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
    };

    if ((*png_ptr).mode & PNG_HAVE_IDAT) != 0 {
        (*png_ptr).mode |= PNG_AFTER_IDAT;
    }

    png_crc_read(png_ptr, buf.as_mut_ptr(), 7);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    mod_time.second = buf[6];
    mod_time.minute = buf[5];
    mod_time.hour = buf[4];
    mod_time.day = buf[3];
    mod_time.month = buf[2];
    mod_time.year = png_get_uint_16(buf.as_ptr());

    png_set_tIME(png_ptr, info_ptr, &mod_time);
    let _ = length;
    handled_ok
}

pub(crate) unsafe fn png_handle_tEXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut text_info: png_text = png_text {
        compression: 0,
        key: ptr::null_mut(),
        text: ptr::null_mut(),
        text_length: 0,
        itxt_length: 0,
        lang: ptr::null_mut(),
        lang_key: ptr::null_mut(),
    };
    let buffer: png_bytep;
    let key: png_charp;
    let mut text: png_charp;
    let skip: png_uint_32 = 0;

    if (*png_ptr).user_chunk_cache_max != 0 {
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            return handled_error;
        }

        (*png_ptr).user_chunk_cache_max -= 1;
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"no space in chunk cache".as_ptr());
            return handled_error;
        }
    }

    buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, skip) != 0 {
        return handled_error;
    }

    key = buffer as png_charp;
    *key.offset(length as isize) = 0;

    text = key;
    while *text != 0 {
        text = text.add(1);
    }

    if text != key.offset(length as isize) {
        text = text.add(1);
    }

    text_info.compression = PNG_TEXT_COMPRESSION_NONE;
    text_info.key = key;
    text_info.lang = ptr::null_mut();
    text_info.lang_key = ptr::null_mut();
    text_info.itxt_length = 0;
    text_info.text = text;
    text_info.text_length = strlen(text);

    if png_set_text_2(png_ptr, info_ptr, &text_info, 1) == 0 {
        return handled_ok;
    }

    png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
    handled_error
}

pub(crate) unsafe fn png_handle_zTXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut errmsg: png_const_charp = ptr::null();
    let mut buffer: png_bytep;
    let mut keyword_length: png_uint_32;

    if (*png_ptr).user_chunk_cache_max != 0 {
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            return handled_error;
        }

        (*png_ptr).user_chunk_cache_max -= 1;
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"no space in chunk cache".as_ptr());
            return handled_error;
        }
    }

    /* Note, "length" is sufficient here; we won't be adding
     * a null terminator later.
     */
    buffer = png_read_buffer(png_ptr, length as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* TODO: also check that the keyword contents match the spec! */
    keyword_length = 0;
    while keyword_length < length && *buffer.offset(keyword_length as isize) != 0 {
        keyword_length += 1;
    }

    if keyword_length > 79 || keyword_length < 1 {
        errmsg = c"bad keyword".as_ptr();
    }
    /* zTXt must have some LZ data after the keyword. */
    else if keyword_length + 3 > length {
        errmsg = c"truncated".as_ptr();
    } else if *buffer.offset((keyword_length + 1) as isize) != PNG_COMPRESSION_TYPE_BASE as png_byte
    {
        errmsg = c"unknown compression type".as_ptr();
    } else {
        let mut uncompressed_length: png_alloc_size_t = PNG_SIZE_MAX;

        if png_decompress_chunk(
            png_ptr,
            length,
            keyword_length + 2,
            &mut uncompressed_length,
            1, /*terminate*/
        ) == Z_STREAM_END
        {
            let mut text: png_text = core::mem::zeroed();

            if (*png_ptr).read_buffer.is_null() {
                errmsg = c"Read failure in png_handle_zTXt".as_ptr();
            } else {
                /* It worked. */
                buffer = (*png_ptr).read_buffer;
                *buffer.offset((uncompressed_length + (keyword_length + 2) as size_t) as isize) = 0;

                text.compression = PNG_TEXT_COMPRESSION_zTXt;
                text.key = buffer as png_charp;
                text.text = buffer.offset((keyword_length + 2) as isize) as png_charp;
                text.text_length = uncompressed_length;
                text.itxt_length = 0;
                text.lang = ptr::null_mut();
                text.lang_key = ptr::null_mut();

                if png_set_text_2(png_ptr, info_ptr, &text, 1) == 0 {
                    return handled_ok;
                }

                errmsg = c"out of memory".as_ptr();
            }
        } else {
            errmsg = (*png_ptr).zstream.msg;
        }
    }

    png_chunk_benign_error(png_ptr, errmsg);
    handled_error
}

pub(crate) unsafe fn png_handle_iTXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut errmsg: png_const_charp = ptr::null();
    let mut buffer: png_bytep;
    let mut prefix_length: png_uint_32;

    if (*png_ptr).user_chunk_cache_max != 0 {
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            return handled_error;
        }

        (*png_ptr).user_chunk_cache_max -= 1;
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"no space in chunk cache".as_ptr());
            return handled_error;
        }
    }

    buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* First the keyword. */
    prefix_length = 0;
    while prefix_length < length && *buffer.offset(prefix_length as isize) != 0 {
        prefix_length += 1;
    }

    /* Perform a basic check on the keyword length here. */
    if prefix_length > 79 || prefix_length < 1 {
        errmsg = c"bad keyword".as_ptr();
    }
    /* Expect keyword, compression flag, compression type, language, translated
     * keyword (both may be empty but are 0 terminated) then the text.
     */
    else if prefix_length + 5 > length {
        errmsg = c"truncated".as_ptr();
    } else if *buffer.offset((prefix_length + 1) as isize) == 0
        || (*buffer.offset((prefix_length + 1) as isize) == 1
            && *buffer.offset((prefix_length + 2) as isize) == PNG_COMPRESSION_TYPE_BASE as png_byte)
    {
        let compressed: c_int = (*buffer.offset((prefix_length + 1) as isize) != 0) as c_int;
        let language_offset: png_uint_32;
        let translated_keyword_offset: png_uint_32;
        let mut uncompressed_length: png_alloc_size_t = 0;

        /* Now the language tag */
        prefix_length += 3;
        language_offset = prefix_length;

        while prefix_length < length && *buffer.offset(prefix_length as isize) != 0 {
            prefix_length += 1;
        }

        /* WARNING: the length may be invalid here, this is checked below. */
        prefix_length += 1;
        translated_keyword_offset = prefix_length;

        while prefix_length < length && *buffer.offset(prefix_length as isize) != 0 {
            prefix_length += 1;
        }

        /* prefix_length should now be at the trailing '\0' of the translated
         * keyword, but it may already be over the end.
         */
        prefix_length += 1;

        if compressed == 0 && prefix_length <= length {
            uncompressed_length = (length - prefix_length) as png_alloc_size_t;
        } else if compressed != 0 && prefix_length < length {
            uncompressed_length = PNG_SIZE_MAX;

            if png_decompress_chunk(
                png_ptr,
                length,
                prefix_length,
                &mut uncompressed_length,
                1, /*terminate*/
            ) == Z_STREAM_END
            {
                buffer = (*png_ptr).read_buffer;
            } else {
                errmsg = (*png_ptr).zstream.msg;
            }
        } else {
            errmsg = c"truncated".as_ptr();
        }

        if errmsg.is_null() {
            let mut text: png_text = core::mem::zeroed();

            *buffer.offset((uncompressed_length + prefix_length as size_t) as isize) = 0;

            if compressed == 0 {
                text.compression = PNG_ITXT_COMPRESSION_NONE;
            } else {
                text.compression = PNG_ITXT_COMPRESSION_zTXt;
            }

            text.key = buffer as png_charp;
            text.lang = (buffer as png_charp).offset(language_offset as isize);
            text.lang_key = (buffer as png_charp).offset(translated_keyword_offset as isize);
            text.text = (buffer as png_charp).offset(prefix_length as isize);
            text.text_length = 0;
            text.itxt_length = uncompressed_length;

            if png_set_text_2(png_ptr, info_ptr, &text, 1) == 0 {
                return handled_ok;
            }

            errmsg = c"out of memory".as_ptr();
        }
    } else {
        errmsg = c"bad compression info".as_ptr();
    }

    if !errmsg.is_null() {
        png_chunk_benign_error(png_ptr, errmsg);
    }
    handled_error
}

/* Utility function for png_handle_unknown; set up png_ptr::unknown_chunk */
pub(crate) unsafe fn png_cache_unknown_chunk(
    png_ptr: png_structrp,
    length: png_uint_32,
) -> c_int {
    let limit: png_alloc_size_t = (*png_ptr).user_chunk_malloc_max;

    if !(*png_ptr).unknown_chunk.data.is_null() {
        png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
        (*png_ptr).unknown_chunk.data = ptr::null_mut();
    }

    if length as png_alloc_size_t <= limit {
        png_cstring_from_chunk(
            (*png_ptr).unknown_chunk.name.as_mut_ptr() as *mut c_char,
            (*png_ptr).chunk_name,
        );
        /* The following is safe because of the PNG_SIZE_MAX init above */
        (*png_ptr).unknown_chunk.size = length as size_t /*SAFE*/;
        /* 'mode' is a flag array, only the bottom four bits matter here */
        (*png_ptr).unknown_chunk.location = (*png_ptr).mode as png_byte /*SAFE*/;

        if length == 0 {
            (*png_ptr).unknown_chunk.data = ptr::null_mut();
        } else {
            /* Do a 'warn' here - it is handled below. */
            (*png_ptr).unknown_chunk.data =
                png_malloc_warn(png_ptr, length as png_alloc_size_t) as png_bytep;
        }
    }

    if (*png_ptr).unknown_chunk.data.is_null() && length > 0 {
        /* This is benign because we clean up correctly */
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, c"unknown chunk exceeds memory limits".as_ptr());
        0
    } else {
        if length > 0 {
            png_crc_read(png_ptr, (*png_ptr).unknown_chunk.data, length);
        }
        png_crc_finish(png_ptr, 0);
        1
    }
}
