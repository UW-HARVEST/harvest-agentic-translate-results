// pngrutil.c - utilities to read a PNG file
//
// This file contains routines that are only called from within
// libpng itself during the course of reading an image.
//
// Chunk 3: png_handle_IHDR .. png_handle_iCCP

use crate::*;

/* CHUNK HANDLING */
/* Read and check the IDHR chunk */
unsafe extern "C" fn png_handle_IHDR(
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

        /* default: invalid, png_set_IHDR calls png_error
         * PNG_COLOR_TYPE_GRAY, PNG_COLOR_TYPE_PALETTE
         */
        _ => {
            (*png_ptr).channels = 1;
        }
    }

    /* Set up other useful info */
    (*png_ptr).pixel_depth = ((*png_ptr).bit_depth as c_int * (*png_ptr).channels as c_int) as png_byte;
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

    handled_ok
}

/* Read and check the palette */
/* TODO: there are several obvious errors in this code when handling
 * out-of-place chunks and there is much over-complexity caused by trying to
 * patch up the problems.
 */
unsafe extern "C" fn png_handle_PLTE(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut errmsg: png_const_charp = core::ptr::null();

    /* 1.6.47: consistency.  This used to be especially treated as a critical
     * error even in an image which is not colour mapped, there isn't a good
     * justification for treating some errors here one way and others another so
     * everything uses the same logic.
     */
    if ((*png_ptr).mode & PNG_HAVE_PLTE) != 0 {
        errmsg = cstr!("duplicate");
    } else if ((*png_ptr).mode & PNG_HAVE_IDAT) != 0 {
        errmsg = cstr!("out of place");
    } else if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0 {
        errmsg = cstr!("ignored in grayscale PNG");
    } else if length > 3 * PNG_MAX_PALETTE_LENGTH as png_uint_32 || (length % 3) != 0 {
        errmsg = cstr!("invalid");
    }
    /* This drops PLTE in favour of tRNS or bKGD because both of those chunks
     * can have an effect on the rendering of the image whereas PLTE only matters
     * in the case of an 8-bit display with a decoder which controls the palette.
     *
     * The alternative here is to ignore the error and store the palette anyway;
     * destroying the tRNS will definitely cause problems.
     *
     * NOTE: the case of PNG_COLOR_TYPE_PALETTE need not be considered because
     * the png_handle_ routines for the three 'after PLTE' chunks tRNS, bKGD and
     * hIST all check for a preceding PLTE in these cases.
     */
    else if (*png_ptr).color_type as c_int != PNG_COLOR_TYPE_PALETTE
        && (png_file_has_chunk(png_ptr, PNG_INDEX_tRNS)
            || png_file_has_chunk(png_ptr, PNG_INDEX_bKGD))
    {
        errmsg = cstr!("out of place");
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
            length as c_uint / 3u32
        };

        let mut i: c_uint;
        let mut j: c_uint;
        let mut buf: [png_byte; 3 * PNG_MAX_PALETTE_LENGTH as usize] =
            [0; 3 * PNG_MAX_PALETTE_LENGTH as usize];
        let mut palette: [png_color; PNG_MAX_PALETTE_LENGTH as usize] = [png_color {
            red: 0,
            green: 0,
            blue: 0,
        }; PNG_MAX_PALETTE_LENGTH as usize];

        /* Read the chunk into the buffer then read to the end of the chunk. */
        png_crc_read(png_ptr, buf.as_mut_ptr(), num * 3u32);
        png_crc_finish_critical(
            png_ptr,
            length - 3u32 * num,
            /* Handle as ancillary if PLTE is optional: */
            ((*png_ptr).color_type as c_int != PNG_COLOR_TYPE_PALETTE) as c_int,
        );

        i = 0u32;
        j = 0u32;
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
    } else
    /* not critical to this image */
    {
        png_crc_finish_critical(png_ptr, length, 1 /*handle as ancillary*/);
        png_chunk_benign_error(png_ptr, errmsg);
    }

    /* Because PNG_UNUSED(errmsg) does not work if all the uses are compiled out
     * (this does happen).
     */
    if !errmsg.is_null() {
        handled_error
    } else {
        handled_error
    }
}

/* On read the IDAT chunk is always handled specially, even if marked for
 * unknown handling (this is allowed), so:
 *
 * #define png_handle_IDAT NULL
 */

unsafe extern "C" fn png_handle_IEND(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    (*png_ptr).mode |= PNG_AFTER_IDAT | PNG_HAVE_IEND;

    if length != 0 {
        png_chunk_benign_error(png_ptr, cstr!("invalid"));
    }

    png_crc_finish_critical(png_ptr, length, 1 /*handle as ancillary*/);

    handled_ok
}

unsafe extern "C" fn png_handle_gAMA(
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
        png_chunk_benign_error(png_ptr, cstr!("invalid"));
        return handled_error;
    }

    png_set_gAMA_fixed(png_ptr, info_ptr, ugamma as png_fixed_point /*SAFE*/);

    /* PNGv3: chunk precedence for gamma is cICP, [iCCP], sRGB, gAMA.  gAMA is
     * at the end of the chain so simply check for an unset value.
     */
    if (*png_ptr).chunk_gamma == 0 {
        (*png_ptr).chunk_gamma = ugamma as png_fixed_point /*SAFE*/;
    }

    handled_ok
}

unsafe extern "C" fn png_handle_sBIT(
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
        png_chunk_benign_error(png_ptr, cstr!("bad length"));
        return handled_error;
    }

    buf[3] = sample_depth;
    buf[2] = buf[3];
    buf[1] = buf[2];
    buf[0] = buf[1];
    png_crc_read(png_ptr, buf.as_mut_ptr(), truelen);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    i = 0;
    while i < truelen {
        if buf[i as usize] == 0 || buf[i as usize] > sample_depth {
            png_chunk_benign_error(png_ptr, cstr!("invalid"));
            return handled_error;
        }
        i += 1;
    }

    if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        (*png_ptr).sig_bit.red = buf[0];
        (*png_ptr).sig_bit.green = buf[1];
        (*png_ptr).sig_bit.blue = buf[2];
        (*png_ptr).sig_bit.alpha = buf[3];
    } else
    /* grayscale */
    {
        (*png_ptr).sig_bit.gray = buf[0];
        (*png_ptr).sig_bit.red = buf[0];
        (*png_ptr).sig_bit.green = buf[0];
        (*png_ptr).sig_bit.blue = buf[0];
        (*png_ptr).sig_bit.alpha = buf[1];
    }

    png_set_sBIT(
        png_ptr,
        info_ptr,
        core::ptr::addr_of!((*png_ptr).sig_bit) as png_const_color_8p,
    );
    handled_ok
}

unsafe extern "C" fn png_get_int_32_checked(buf: png_const_bytep, error: *mut c_int) -> png_int_32 {
    let mut uval: png_uint_32 = png_get_uint_32(buf);
    if (uval & 0x80000000) == 0
    /* non-negative */
    {
        return uval as png_int_32;
    }

    uval = (uval ^ 0xffffffff).wrapping_add(1); /* 2's complement: -x = ~x+1 */
    if (uval & 0x80000000) == 0
    /* no overflow */
    {
        return (uval as png_int_32).wrapping_neg();
    }

    /* This version of png_get_int_32 has a way of returning the error to the
     * caller, so:
     */
    *error = 1;
    0 /* Safe */
}

unsafe extern "C" fn png_handle_cHRM(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut error: c_int = 0;
    let mut xy: png_xy = core::mem::zeroed();
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
        png_chunk_benign_error(png_ptr, cstr!("invalid"));
        return handled_error;
    }

    /* png_set_cHRM may complain about some of the values but this doesn't matter
     * because it was a cHRM and it did have vaguely (if, perhaps, ridiculous)
     * values.  Ridiculosity will be checked if the values are used later.
     */
    png_set_cHRM_fixed(
        png_ptr,
        info_ptr,
        xy.whitex,
        xy.whitey,
        xy.redx,
        xy.redy,
        xy.greenx,
        xy.greeny,
        xy.bluex,
        xy.bluey,
    );

    /* We only use 'chromaticities' for RGB to gray */

    /* There is no need to check sRGB here, cICP is NYI and iCCP is not
     * supported so just check mDCV.
     */
    if !png_file_has_chunk(png_ptr, PNG_INDEX_mDCV) {
        (*png_ptr).chromaticities = xy;
    }

    handled_ok
}

unsafe extern "C" fn png_handle_sRGB(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut intent: png_byte = 0;

    png_crc_read(png_ptr, &mut intent as *mut png_byte, 1);

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
        png_chunk_benign_error(png_ptr, cstr!("invalid"));
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

    handled_ok
}

unsafe extern "C" fn png_handle_iCCP(
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

        if length < (2u32 + 5u32 + 4u32)
        /* LZ77Min */
        {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr!("too short"));
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
                    let mut profile_header: [png_byte; 132] = [0; 132];
                    let mut local_buffer: [png_byte; PNG_INFLATE_BUF_SIZE] =
                        [0; PNG_INFLATE_BUF_SIZE];
                    let mut size: png_alloc_size_t = core::mem::size_of_val(&profile_header);

                    (*png_ptr).zstream.next_in = (keyword.as_mut_ptr() as *mut Bytef)
                        .add((keyword_length + 2) as usize) as *const Bytef;
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

                                    size = 12u32.wrapping_mul(tag_count) as png_alloc_size_t;

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
                                            size = (profile_length as png_alloc_size_t)
                                                .wrapping_sub(core::mem::size_of_val(
                                                    &profile_header,
                                                ))
                                                .wrapping_sub(
                                                    12u32.wrapping_mul(tag_count)
                                                        as png_alloc_size_t,
                                                );

                                            png_inflate_read(
                                                png_ptr,
                                                local_buffer.as_mut_ptr(),
                                                core::mem::size_of_val(&local_buffer) as uInt,
                                                &mut length,
                                                profile
                                                    .add(core::mem::size_of_val(&profile_header))
                                                    .add(12u32.wrapping_mul(tag_count) as usize),
                                                &mut size,
                                                1, /*finish*/
                                            );

                                            if length > 0
                                                && ((*png_ptr).flags
                                                    & PNG_FLAG_BENIGN_ERRORS_WARN)
                                                    == 0
                                            {
                                                errmsg = cstr!("extra compressed data");
                                            }
                                            /* But otherwise allow extra data: */
                                            else if size == 0 {
                                                if length > 0 {
                                                    /* This can be handled completely, so
                                                     * keep going.
                                                     */
                                                    png_chunk_warning(
                                                        png_ptr,
                                                        cstr!("extra compressed data"),
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
                                                    ) as *mut c_char;
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
                                                        errmsg = cstr!("out of memory");
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
                                    } else
                                    /* profile truncated */
                                    {
                                        errmsg = (*png_ptr).zstream.msg;
                                    }
                                } else {
                                    errmsg = cstr!("out of memory");
                                }
                            }

                            /* else png_icc_check_header output an error */
                        }

                        /* else png_icc_check_length output an error */
                    } else
                    /* profile truncated */
                    {
                        errmsg = (*png_ptr).zstream.msg;
                    }

                    /* Release the stream */
                    (*png_ptr).zowner = 0;
                } else
                /* png_inflate_claim failed */
                {
                    errmsg = (*png_ptr).zstream.msg;
                }
            } else {
                errmsg = cstr!("bad compression method"); /* or missing */
            }
        } else {
            errmsg = cstr!("bad keyword");
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
