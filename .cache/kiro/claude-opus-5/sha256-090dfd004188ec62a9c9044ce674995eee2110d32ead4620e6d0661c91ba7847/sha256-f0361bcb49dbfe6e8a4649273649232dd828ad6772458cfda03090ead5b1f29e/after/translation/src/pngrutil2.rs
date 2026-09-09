//! Translation of c_src/src/pngrutil.c lines 901..2263
use crate::prelude::*;

/* pngrutil.c line 23: #define LZ77Min (2U+5U+4U) */
const LZ77Min: c_uint = 2 + 5 + 4;

pub unsafe extern "C" fn png_handle_IHDR(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buf: [png_byte; 13] = [0; 13];
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
        PNG_COLOR_TYPE_RGB => {
            (*png_ptr).channels = 3;
        }
        PNG_COLOR_TYPE_GRAY_ALPHA => {
            (*png_ptr).channels = 2;
        }
        PNG_COLOR_TYPE_RGB_ALPHA => {
            (*png_ptr).channels = 4;
        }
        /* default / PNG_COLOR_TYPE_GRAY / PNG_COLOR_TYPE_PALETTE */
        _ => {
            (*png_ptr).channels = 1;
        }
    }

    /* Set up other useful info */
    (*png_ptr).pixel_depth =
        ((*png_ptr).bit_depth as c_int * (*png_ptr).channels as c_int) as png_byte;
    (*png_ptr).rowbytes = PNG_ROWBYTES((*png_ptr).pixel_depth as usize, (*png_ptr).width as usize);

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
pub unsafe extern "C" fn png_handle_PLTE(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut errmsg: png_const_charp = core::ptr::null();

    if ((*png_ptr).mode & PNG_HAVE_PLTE) != 0 {
        errmsg = cstr(b"duplicate\0");
    } else if ((*png_ptr).mode & PNG_HAVE_IDAT) != 0 {
        errmsg = cstr(b"out of place\0");
    } else if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0 {
        errmsg = cstr(b"ignored in grayscale PNG\0");
    } else if length > (3 * PNG_MAX_PALETTE_LENGTH) as png_uint_32 || (length % 3) != 0 {
        errmsg = cstr(b"invalid\0");
    } else if (*png_ptr).color_type as c_int != PNG_COLOR_TYPE_PALETTE
        && (png_file_has_chunk(png_ptr, PNG_INDEX_tRNS)
            || png_file_has_chunk(png_ptr, PNG_INDEX_bKGD))
    {
        errmsg = cstr(b"out of place\0");
    } else {
        /* If the palette has 256 or fewer entries but is too large for the bit
         * depth we don't issue an error to preserve the behavior of previous
         * libpng versions. We silently truncate the unused extra palette entries
         * here.
         */
        let max_palette_length: c_uint = if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
        {
            1u32 << (*png_ptr).bit_depth
        } else {
            PNG_MAX_PALETTE_LENGTH as c_uint
        };

        /* The cast is safe because 'length' is less than
         * 3*PNG_MAX_PALETTE_LENGTH
         */
        let num: c_uint = if length > 3u32 * max_palette_length {
            max_palette_length
        } else {
            (length as c_uint) / 3u32
        };

        let mut i: c_uint;
        let mut j: c_uint;
        let mut buf: [png_byte; (3 * PNG_MAX_PALETTE_LENGTH) as usize] =
            [0; (3 * PNG_MAX_PALETTE_LENGTH) as usize];
        let mut palette: [png_color; PNG_MAX_PALETTE_LENGTH as usize] =
            [png_color::default(); PNG_MAX_PALETTE_LENGTH as usize];

        /* Read the chunk into the buffer then read to the end of the chunk. */
        png_crc_read(png_ptr, buf.as_mut_ptr(), num * 3u32);
        png_crc_finish_critical(
            png_ptr,
            length - 3u32 * num,
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

pub unsafe extern "C" fn png_handle_IEND(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    (*png_ptr).mode |= PNG_AFTER_IDAT | PNG_HAVE_IEND;

    if length != 0 {
        png_chunk_benign_error(png_ptr, cstr(b"invalid\0"));
    }

    png_crc_finish_critical(png_ptr, length, 1 /*handle as ancillary*/);

    let _ = info_ptr;
    handled_ok
}

pub unsafe extern "C" fn png_handle_gAMA(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let ugamma: png_uint_32;
    let mut buf: [png_byte; 4] = [0; 4];

    png_crc_read(png_ptr, buf.as_mut_ptr(), 4);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    ugamma = png_get_uint_32(buf.as_ptr());

    if ugamma > PNG_UINT_31_MAX {
        png_chunk_benign_error(png_ptr, cstr(b"invalid\0"));
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

pub unsafe extern "C" fn png_handle_sBIT(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let truelen: c_uint;
    let mut i: c_uint;
    let sample_depth: png_byte;
    let mut buf: [png_byte; 4] = [0; 4];

    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        truelen = 3;
        sample_depth = 8;
    } else {
        truelen = (*png_ptr).channels as c_uint;
        sample_depth = (*png_ptr).bit_depth;
    }

    if length != truelen {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr(b"bad length\0"));
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
            png_chunk_benign_error(png_ptr, cstr(b"invalid\0"));
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

    png_set_sBIT(png_ptr, info_ptr, &mut (*png_ptr).sig_bit);
    handled_ok
}

pub unsafe extern "C" fn png_get_int_32_checked(
    buf: png_const_bytep,
    error: *mut c_int,
) -> png_int_32 {
    let mut uval: png_uint_32 = png_get_uint_32(buf);
    if (uval & 0x80000000) == 0 {
        /* non-negative */
        return uval as png_int_32;
    }

    uval = (uval ^ 0xffffffff).wrapping_add(1); /* 2's complement: -x = ~x+1 */
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

pub unsafe extern "C" fn png_handle_cHRM(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut error: c_int = 0;
    let mut xy: png_xy = png_xy::default();
    let mut buf: [png_byte; 32] = [0; 32];

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
        png_chunk_benign_error(png_ptr, cstr(b"invalid\0"));
        return handled_error;
    }

    /* png_set_cHRM may complain about some of the values but this doesn't matter
     * because it was a cHRM and it did have vaguely (if, perhaps, ridiculous)
     * values.  Ridiculosity will be checked if the values are used later.
     */
    png_set_cHRM_fixed(
        png_ptr, info_ptr, xy.whitex, xy.whitey, xy.redx, xy.redy, xy.greenx, xy.greeny, xy.bluex,
        xy.bluey,
    );

    /* We only use 'chromaticities' for RGB to gray */
    /* There is no need to check sRGB here, cICP is NYI and iCCP is not
     * supported so just check mDCV.
     */
    if !png_file_has_chunk(png_ptr, PNG_INDEX_mDCV) {
        (*png_ptr).chromaticities = xy;
    }

    let _ = length;
    handled_ok
}

pub unsafe extern "C" fn png_handle_sRGB(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut intent: png_byte = 0;

    png_crc_read(png_ptr, &mut intent, 1);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* This checks the range of the "rendering intent" because it is specified in
     * the PNG spec itself; the "reserved" values will result in the chunk not
     * being accepted, just as they do with the various "reserved" values in
     * IHDR.
     */
    if intent > 3
    /*PNGv3 spec*/
    {
        png_chunk_benign_error(png_ptr, cstr(b"invalid\0"));
        return handled_error;
    }

    png_set_sRGB(png_ptr, info_ptr, intent as c_int);
    /* NOTE: png_struct::chromaticities is not set here because the RGB to gray
     * coefficients are known without a need for the chromaticities.
     */

    /* PNGv3: chunk precedence for gamma is cICP, [iCCP], sRGB, gAMA.  iCCP is
     * not supported by libpng so the only requirement is to check for cICP
     * setting the gamma (this is NYI, but this check is safe.)
     */
    if !png_file_has_chunk(png_ptr, PNG_INDEX_cICP) || (*png_ptr).chunk_gamma == 0 {
        (*png_ptr).chunk_gamma = PNG_GAMMA_sRGB_INVERSE;
    }

    let _ = length;
    handled_ok
}

pub unsafe extern "C" fn png_handle_iCCP(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    mut length: png_uint_32,
) -> png_handle_result_code
/* Note: this does not properly handle profiles that are > 64K under DOS */
{
    let mut errmsg: png_const_charp = core::ptr::null(); /* error message output, or no error */
    let mut finished: c_int = 0; /* crc checked */

    /* PNGv3: allow PNG files with both sRGB and iCCP because the PNG spec only
     * ever said that there "should" be only one, not "shall" and the PNGv3
     * colour chunk precedence rules give a handling for this case anyway.
     */
    {
        let mut read_length: uInt;
        let mut keyword_length: uInt;
        let mut keyword: [c_char; 81] = [0; 81];

        /* Find the keyword; the keyword plus separator and compression method
         * bytes can be at most 81 characters long.
         */
        read_length = 81; /* maximum */
        if read_length > length {
            read_length = length as uInt /*SAFE*/;
        }

        png_crc_read(png_ptr, keyword.as_mut_ptr() as png_bytep, read_length);
        length -= read_length;

        if length < LZ77Min {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr(b"too short\0"));
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
            /* We only understand '0' compression - deflate - so if we get a
             * different value we can't safely decode the chunk.
             */
            if keyword_length + 1 < read_length
                && keyword[(keyword_length + 1) as usize] as c_int == PNG_COMPRESSION_TYPE_BASE
            {
                read_length -= keyword_length + 2;

                if png_inflate_claim(png_ptr, png_iCCP) == Z_OK {
                    let mut profile_header: [u8; 132] = [0; 132];
                    let mut local_buffer: [u8; PNG_INFLATE_BUF_SIZE] = [0; PNG_INFLATE_BUF_SIZE];
                    let mut size: png_alloc_size_t =
                        core::mem::size_of_val(&profile_header) as png_alloc_size_t;

                    (*png_ptr).zstream.next_in =
                        (keyword.as_mut_ptr() as *mut u8).add((keyword_length + 2) as usize);
                    (*png_ptr).zstream.avail_in = read_length;
                    png_inflate_read(
                        png_ptr,
                        local_buffer.as_mut_ptr(),
                        core::mem::size_of_val(&local_buffer) as uInt,
                        &mut length,
                        profile_header.as_mut_ptr(),
                        &mut size,
                        0, /*finish: don't, because the output is too small*/
                    );

                    if size == 0 {
                        /* We have the ICC profile header; do the basic header checks.
                         */
                        let profile_length: png_uint_32 = png_get_uint_32(profile_header.as_ptr());

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
                                /* Now read the tag table; a variable size buffer is
                                 * needed at this point, allocate one for the whole
                                 * profile.  The header check has already validated
                                 * that none of this stuff will overflow.
                                 */
                                let tag_count: png_uint_32 =
                                    png_get_uint_32(profile_header.as_ptr().add(128));
                                let profile: png_bytep =
                                    png_read_buffer(png_ptr, profile_length as png_alloc_size_t);

                                if !profile.is_null() {
                                    memcpy(
                                        profile as *mut c_void,
                                        profile_header.as_ptr() as *const c_void,
                                        core::mem::size_of_val(&profile_header),
                                    );

                                    size = (12u32 * tag_count) as png_alloc_size_t;

                                    png_inflate_read(
                                        png_ptr,
                                        local_buffer.as_mut_ptr(),
                                        core::mem::size_of_val(&local_buffer) as uInt,
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
                                            /* The profile has been validated for basic
                                             * security issues, so read the whole thing in.
                                             */
                                            size = (profile_length
                                                - core::mem::size_of_val(&profile_header)
                                                    as png_uint_32
                                                - 12u32 * tag_count)
                                                as png_alloc_size_t;

                                            png_inflate_read(
                                                png_ptr,
                                                local_buffer.as_mut_ptr(),
                                                core::mem::size_of_val(&local_buffer) as uInt,
                                                &mut length,
                                                profile
                                                    .add(core::mem::size_of_val(&profile_header))
                                                    .add((12u32 * tag_count) as usize),
                                                &mut size,
                                                1, /*finish*/
                                            );

                                            if length > 0
                                                && ((*png_ptr).flags & PNG_FLAG_BENIGN_ERRORS_WARN)
                                                    == 0
                                            {
                                                errmsg = cstr(b"extra compressed data\0");
                                            }
                                            /* But otherwise allow extra data: */
                                            else if size == 0 {
                                                if length > 0 {
                                                    /* This can be handled completely, so
                                                     * keep going.
                                                     */
                                                    png_chunk_warning(
                                                        png_ptr,
                                                        cstr(b"extra compressed data\0"),
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
                                                            (keyword_length + 1) as usize,
                                                        );
                                                        (*info_ptr).iccp_proflen = profile_length;
                                                        (*info_ptr).iccp_profile = profile;
                                                        (*png_ptr).read_buffer =
                                                            core::ptr::null_mut(); /*steal*/
                                                        (*info_ptr).free_me |= PNG_FREE_ICCP;
                                                        (*info_ptr).valid |= PNG_INFO_iCCP;
                                                    } else {
                                                        errmsg = cstr(b"out of memory\0");
                                                    }
                                                }

                                                /* else the profile remains in the read
                                                 * buffer which gets reused for subsequent
                                                 * chunks.
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
                                    errmsg = cstr(b"out of memory\0");
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
                errmsg = cstr(b"bad compression method\0"); /* or missing */
            }
        } else {
            errmsg = cstr(b"bad keyword\0");
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

pub unsafe extern "C" fn png_handle_sPLT(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code
/* Note: this does not properly handle chunks that are > 64K under DOS */
{
    let buffer: png_bytep;
    let mut entry_start: png_bytep;
    let mut new_palette: png_sPLT_t = png_sPLT_t {
        name: core::ptr::null_mut(),
        depth: 0,
        entries: core::ptr::null_mut(),
        nentries: 0,
    };
    let mut pp: png_sPLT_entryp;
    let data_length: png_uint_32;
    let entry_size: c_int;
    let mut i: c_int;
    let skip: png_uint_32 = 0;
    let dl: png_uint_32;
    let max_dl: usize;

    if (*png_ptr).user_chunk_cache_max != 0 {
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_crc_finish(png_ptr, length);
            return handled_error;
        }

        (*png_ptr).user_chunk_cache_max -= 1;
        if (*png_ptr).user_chunk_cache_max == 1 {
            png_warning(png_ptr, cstr(b"No space in chunk cache for sPLT\0"));
            png_crc_finish(png_ptr, length);
            return handled_error;
        }
    }

    buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);
    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr(b"out of memory\0"));
        return handled_error;
    }

    /* WARNING: this may break if size_t is less than 32 bits; it is assumed
     * that the PNG_MAX_MALLOC_64K test is enabled in this case, but this is a
     * potential breakage point if the types in pngconf.h aren't exactly right.
     */
    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, skip) != 0 {
        return handled_error;
    }

    *buffer.add(length as usize) = 0;

    entry_start = buffer;
    while *entry_start != 0 {
        /* Empty loop to find end of name */
        entry_start = entry_start.add(1);
    }

    entry_start = entry_start.add(1);

    /* A sample depth should follow the separator, and we should be on it  */
    if length < 2u32 || entry_start > buffer.add((length - 2u32) as usize) {
        png_warning(png_ptr, cstr(b"malformed sPLT chunk\0"));
        return handled_error;
    }

    new_palette.depth = *entry_start;
    entry_start = entry_start.add(1);
    entry_size = if new_palette.depth == 8 { 6 } else { 10 };
    /* This must fit in a png_uint_32 because it is derived from the original
     * chunk data length.
     */
    data_length = length - (entry_start.offset_from(buffer) as png_uint_32);

    /* Integrity-check the data length */
    if (data_length % (entry_size as c_uint)) != 0 {
        png_warning(png_ptr, cstr(b"sPLT chunk has bad length\0"));
        return handled_error;
    }

    dl = data_length / (entry_size as c_uint);
    max_dl = PNG_SIZE_MAX / core::mem::size_of::<png_sPLT_entry>();

    if dl as usize > max_dl {
        png_warning(png_ptr, cstr(b"sPLT chunk too long\0"));
        return handled_error;
    }

    new_palette.nentries = (data_length / (entry_size as c_uint)) as png_int_32;

    new_palette.entries = png_malloc_warn(
        png_ptr,
        (new_palette.nentries as png_alloc_size_t) * core::mem::size_of::<png_sPLT_entry>(),
    ) as png_sPLT_entryp;

    if new_palette.entries.is_null() {
        png_warning(png_ptr, cstr(b"sPLT chunk requires too much memory\0"));
        return handled_error;
    }

    i = 0;
    while i < new_palette.nentries {
        pp = new_palette.entries.add(i as usize);

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

pub unsafe extern "C" fn png_handle_tRNS(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut readbuf: [png_byte; PNG_MAX_PALETTE_LENGTH as usize] =
        [0; PNG_MAX_PALETTE_LENGTH as usize];

    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY {
        let mut buf: [png_byte; 2] = [0; 2];

        if length != 2 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr(b"invalid\0"));
            return handled_error;
        }

        png_crc_read(png_ptr, buf.as_mut_ptr(), 2);
        (*png_ptr).num_trans = 1;
        (*png_ptr).trans_color.gray = png_get_uint_16(buf.as_ptr());
    } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB {
        let mut buf: [png_byte; 6] = [0; 6];

        if length != 6 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr(b"invalid\0"));
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
            png_chunk_benign_error(png_ptr, cstr(b"out of place\0"));
            return handled_error;
        }

        if length > (*png_ptr).num_palette as c_uint
            || length > PNG_MAX_PALETTE_LENGTH as c_uint
            || length == 0
        {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr(b"invalid\0"));
            return handled_error;
        }

        png_crc_read(png_ptr, readbuf.as_mut_ptr(), length);
        (*png_ptr).num_trans = length as png_uint_16;
    } else {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr(b"invalid with alpha channel\0"));
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
        &mut (*png_ptr).trans_color,
    );
    handled_ok
}

pub unsafe extern "C" fn png_handle_bKGD(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let truelen: c_uint;
    let mut buf: [png_byte; 6] = [0; 6];
    let mut background: png_color_16 = png_color_16::default();

    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        if ((*png_ptr).mode & PNG_HAVE_PLTE) == 0 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr(b"out of place\0"));
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
        png_chunk_benign_error(png_ptr, cstr(b"invalid\0"));
        return handled_error;
    }

    png_crc_read(png_ptr, buf.as_mut_ptr(), truelen);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* We convert the index value into RGB components so that we can allow
     * arbitrary RGB values for background when we have transparency, and
     * so it is easy to determine the RGB values of the background color
     * from the info_ptr struct.
     */
    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        background.index = buf[0];

        if !info_ptr.is_null() && (*info_ptr).num_palette != 0 {
            if buf[0] as png_uint_16 >= (*info_ptr).num_palette {
                png_chunk_benign_error(png_ptr, cstr(b"invalid index\0"));
                return handled_error;
            }

            background.red = (*(*png_ptr).palette.add(buf[0] as usize)).red as png_uint_16;
            background.green = (*(*png_ptr).palette.add(buf[0] as usize)).green as png_uint_16;
            background.blue = (*(*png_ptr).palette.add(buf[0] as usize)).blue as png_uint_16;
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
                png_chunk_benign_error(png_ptr, cstr(b"invalid gray level\0"));
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
                png_chunk_benign_error(png_ptr, cstr(b"invalid color\0"));
                return handled_error;
            }
        }

        background.index = 0;
        background.red = png_get_uint_16(buf.as_ptr());
        background.green = png_get_uint_16(buf.as_ptr().add(2));
        background.blue = png_get_uint_16(buf.as_ptr().add(4));
        background.gray = 0;
    }

    png_set_bKGD(png_ptr, info_ptr, &mut background);
    handled_ok
}

pub unsafe extern "C" fn png_handle_cICP(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buf: [png_byte; 4] = [0; 4];

    png_crc_read(png_ptr, buf.as_mut_ptr(), 4);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    png_set_cICP(png_ptr, info_ptr, buf[0], buf[1], buf[2], buf[3]);

    /* We only use 'chromaticities' for RGB to gray */
    if !png_file_has_chunk(png_ptr, PNG_INDEX_mDCV) {
        /* TODO: png_ptr->chromaticities = chromaticities; */
    }

    /* PNGv3: chunk precedence for gamma is cICP, [iCCP], sRGB, gAMA.  cICP is
     * at the head so simply set the gamma if it can be determined.  If not
     * chunk_gamma remains unchanged; sRGB and gAMA handling check it for
     * being zero.
     */
    /* TODO: set png_struct::chunk_gamma when possible */

    let _ = length;
    handled_ok
}

pub unsafe extern "C" fn png_handle_cLLI(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buf: [png_byte; 8] = [0; 8];

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

pub unsafe extern "C" fn png_handle_mDCV(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut chromaticities: png_xy = png_xy::default();
    let mut buf: [png_byte; 24] = [0; 24];

    png_crc_read(png_ptr, buf.as_mut_ptr(), 24);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* The error checking happens here, this puts it in just one place.  The
     * odd /50000 scaling factor makes it more difficult but the (x.y) values are
     * only two bytes so a <<1 is safe.
     */
    chromaticities.redx = ((png_get_uint_16(buf.as_ptr().add(0)) as c_int) << 1) as png_fixed_point; /* red x */
    chromaticities.redy = ((png_get_uint_16(buf.as_ptr().add(2)) as c_int) << 1) as png_fixed_point; /* red y */
    chromaticities.greenx =
        ((png_get_uint_16(buf.as_ptr().add(4)) as c_int) << 1) as png_fixed_point; /* green x */
    chromaticities.greeny =
        ((png_get_uint_16(buf.as_ptr().add(6)) as c_int) << 1) as png_fixed_point; /* green y */
    chromaticities.bluex =
        ((png_get_uint_16(buf.as_ptr().add(8)) as c_int) << 1) as png_fixed_point; /* blue x */
    chromaticities.bluey =
        ((png_get_uint_16(buf.as_ptr().add(10)) as c_int) << 1) as png_fixed_point; /* blue y */
    chromaticities.whitex =
        ((png_get_uint_16(buf.as_ptr().add(12)) as c_int) << 1) as png_fixed_point; /* white x */
    chromaticities.whitey =
        ((png_get_uint_16(buf.as_ptr().add(14)) as c_int) << 1) as png_fixed_point; /* white y */

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
        png_get_uint_32(buf.as_ptr().add(20)),
    ); /* minimum perceivable luminance */

    /* We only use 'chromaticities' for RGB to gray */
    (*png_ptr).chromaticities = chromaticities;

    let _ = length;
    handled_ok
}

pub unsafe extern "C" fn png_handle_eXIf(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let buffer: png_bytep;

    buffer = png_read_buffer(png_ptr, length as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr(b"out of memory\0"));
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* PNGv3: the code used to check the byte order mark at the start for MM or
     * II, however PNGv3 states that the first 4 bytes should be checked.
     * The caller ensures that there are four bytes available.
     */
    {
        let header: png_uint_32 = png_get_uint_32(buffer);

        /* These numbers are copied from the PNGv3 spec: */
        if header != 0x49492A00 && header != 0x4D4D002A {
            png_chunk_benign_error(png_ptr, cstr(b"invalid\0"));
            return handled_error;
        }
    }

    png_set_eXIf_1(png_ptr, info_ptr, length, buffer);
    handled_ok
}

pub unsafe extern "C" fn png_handle_hIST(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let num: c_uint;
    let mut i: c_uint;
    let mut readbuf: [png_uint_16; PNG_MAX_PALETTE_LENGTH as usize] =
        [0; PNG_MAX_PALETTE_LENGTH as usize];

    /* This cast is safe because the chunk definition limits the length to a
     * maximum of 1024 bytes.
     */
    num = (length as c_uint) / 2;

    if length != num * 2
        || num != (*png_ptr).num_palette as c_uint
        || num > PNG_MAX_PALETTE_LENGTH as c_uint
    {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr(b"invalid\0"));
        return handled_error;
    }

    i = 0;
    while i < num {
        let mut buf: [png_byte; 2] = [0; 2];

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

pub unsafe extern "C" fn png_handle_pHYs(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buf: [png_byte; 9] = [0; 9];
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

pub unsafe extern "C" fn png_handle_oFFs(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buf: [png_byte; 9] = [0; 9];
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
pub unsafe extern "C" fn png_handle_pCAL(
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
    let units: *mut png_byte;
    let params: png_charpp;
    let mut i: c_int;

    buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr(b"out of memory\0"));
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    *buffer.add(length as usize) = 0; /* Null terminate the last string */

    buf = buffer;
    while *buf != 0 {
        /* Empty loop */
        buf = buf.add(1);
    }

    endptr = buffer.add(length as usize);

    /* We need to have at least 12 bytes after the purpose string
     * in order to get the parameter information.
     */
    if endptr.offset_from(buf) <= 12 {
        png_chunk_benign_error(png_ptr, cstr(b"invalid\0"));
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
    if (type_ as c_int == PNG_EQUATION_LINEAR && nparams as c_int != 2)
        || (type_ as c_int == PNG_EQUATION_BASE_E && nparams as c_int != 3)
        || (type_ as c_int == PNG_EQUATION_ARBITRARY && nparams as c_int != 3)
        || (type_ as c_int == PNG_EQUATION_HYPERBOLIC && nparams as c_int != 4)
    {
        png_chunk_benign_error(png_ptr, cstr(b"invalid parameter count\0"));
        return handled_error;
    } else if type_ as c_int >= PNG_EQUATION_LAST {
        png_chunk_benign_error(png_ptr, cstr(b"unrecognized equation type\0"));
    }

    buf = units;
    while *buf != 0 {
        /* Empty loop to move past the units string. */
        buf = buf.add(1);
    }

    params = png_malloc_warn(
        png_ptr,
        (nparams as usize) * core::mem::size_of::<png_charp>(),
    ) as png_charpp;

    if params.is_null() {
        png_chunk_benign_error(png_ptr, cstr(b"out of memory\0"));
        return handled_error;
    }

    /* Get pointers to the start of each parameter string. */
    i = 0;
    while i < nparams as c_int {
        buf = buf.add(1); /* Skip the null string terminator from previous parameter. */

        *params.add(i as usize) = buf as png_charp;
        while buf <= endptr && *buf != 0 {
            /* Empty loop to move past each parameter string */
            buf = buf.add(1);
        }

        /* Make sure we haven't run out of data yet */
        if buf > endptr {
            png_free(png_ptr, params as png_voidp);
            png_chunk_benign_error(png_ptr, cstr(b"invalid data\0"));
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
