/* pngrutil.c lines 901..1326 */

/* CHUNK HANDLING */
/* Read and check the IDHR chunk */
/* png_handle_IHDR */
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
    /* PNG_MNG_FEATURES_SUPPORTED */
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

    handled_ok
}

/* Read and check the palette */
/* TODO: there are several obvious errors in this code when handling
 * out-of-place chunks and there is much over-complexity caused by trying to
 * patch up the problems.
 */
/* png_handle_PLTE */
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
        errmsg = b"duplicate\0".as_ptr() as png_const_charp;
    } else if ((*png_ptr).mode & PNG_HAVE_IDAT) != 0 {
        errmsg = b"out of place\0".as_ptr() as png_const_charp;
    } else if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0 {
        errmsg = b"ignored in grayscale PNG\0".as_ptr() as png_const_charp;
    } else if length > 3 * PNG_MAX_PALETTE_LENGTH as png_uint_32 || (length % 3) != 0 {
        errmsg = b"invalid\0".as_ptr() as png_const_charp;
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
        errmsg = b"out of place\0".as_ptr() as png_const_charp;
    } else {
        /* If the palette has 256 or fewer entries but is too large for the bit
         * depth we don't issue an error to preserve the behavior of previous
         * libpng versions. We silently truncate the unused extra palette entries
         * here.
         */
        let max_palette_length: c_uint = if (*png_ptr).color_type as c_int
            == PNG_COLOR_TYPE_PALETTE
        {
            1u32 << (*png_ptr).bit_depth as c_uint
        } else {
            PNG_MAX_PALETTE_LENGTH as c_uint
        };

        /* The cast is safe because 'length' is less than
         * 3*PNG_MAX_PALETTE_LENGTH
         */
        let num: c_uint = if length > 3u32.wrapping_mul(max_palette_length) {
            max_palette_length
        } else {
            (length as c_uint) / 3u32
        };

        let mut i: c_uint;
        let mut j: c_uint;
        let mut buf: [png_byte; 3 * (PNG_MAX_PALETTE_LENGTH as usize)] =
            [0; 3 * (PNG_MAX_PALETTE_LENGTH as usize)];
        let mut palette: [png_color; PNG_MAX_PALETTE_LENGTH as usize] =
            [Default::default(); PNG_MAX_PALETTE_LENGTH as usize];

        /* Read the chunk into the buffer then read to the end of the chunk. */
        png_crc_read(png_ptr, buf.as_mut_ptr(), num.wrapping_mul(3u32));
        png_crc_finish_critical(
            png_ptr,
            length.wrapping_sub(3u32.wrapping_mul(num)),
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
    }
    /* not critical to this image */
    else {
        png_crc_finish_critical(png_ptr, length, 1 /*handle as ancillary*/);
        png_chunk_benign_error(png_ptr, errmsg);
    }

    /* Because PNG_UNUSED(errmsg) does not work if all the uses are compiled out
     * (this does happen).
     */
    if errmsg != core::ptr::null() {
        handled_error
    } else {
        handled_error
    }
}

/* On read the IDAT chunk is always handled specially, even if marked for
 * unknown handling (this is allowed), so:
 */
/* #define png_handle_IDAT NULL */

/* png_handle_IEND */
unsafe extern "C" fn png_handle_IEND(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    (*png_ptr).mode |= PNG_AFTER_IDAT | PNG_HAVE_IEND;

    if length != 0 {
        png_chunk_benign_error(png_ptr, b"invalid\0".as_ptr() as png_const_charp);
    }

    png_crc_finish_critical(png_ptr, length, 1 /*handle as ancillary*/);

    handled_ok
}

/* png_handle_gAMA */
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

    ugamma = PNG_get_uint_32(buf.as_ptr());

    if ugamma > PNG_UINT_31_MAX {
        png_chunk_benign_error(png_ptr, b"invalid\0".as_ptr() as png_const_charp);
        return handled_error;
    }

    png_set_gAMA_fixed(png_ptr, info_ptr, ugamma as png_fixed_point /*SAFE*/);

    /* PNG_READ_GAMMA_SUPPORTED */
    /* PNGv3: chunk precedence for gamma is cICP, [iCCP], sRGB, gAMA.  gAMA is
     * at the end of the chain so simply check for an unset value.
     */
    if (*png_ptr).chunk_gamma == 0 {
        (*png_ptr).chunk_gamma = ugamma as png_fixed_point /*SAFE*/;
    }

    handled_ok
}

/* png_handle_sBIT */
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
        png_chunk_benign_error(png_ptr, b"bad length\0".as_ptr() as png_const_charp);
        return handled_error;
    }

    buf[3] = sample_depth;
    buf[2] = sample_depth;
    buf[1] = sample_depth;
    buf[0] = sample_depth;
    png_crc_read(png_ptr, buf.as_mut_ptr(), truelen);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    i = 0;
    while i < truelen {
        if buf[i as usize] == 0 || buf[i as usize] > sample_depth {
            png_chunk_benign_error(png_ptr, b"invalid\0".as_ptr() as png_const_charp);
            return handled_error;
        }

        i += 1;
    }

    if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
        (*png_ptr).sig_bit.red = buf[0];
        (*png_ptr).sig_bit.green = buf[1];
        (*png_ptr).sig_bit.blue = buf[2];
        (*png_ptr).sig_bit.alpha = buf[3];
    }
    /* grayscale */
    else {
        (*png_ptr).sig_bit.gray = buf[0];
        (*png_ptr).sig_bit.red = buf[0];
        (*png_ptr).sig_bit.green = buf[0];
        (*png_ptr).sig_bit.blue = buf[0];
        (*png_ptr).sig_bit.alpha = buf[1];
    }

    png_set_sBIT(png_ptr, info_ptr, core::ptr::addr_of!((*png_ptr).sig_bit));
    handled_ok
}

/* png_get_int_32_checked */
unsafe fn png_get_int_32_checked(buf: png_const_bytep, error: *mut c_int) -> png_int_32 {
    let mut uval: png_uint_32 = PNG_get_uint_32(buf);
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

/* png_handle_cHRM */
unsafe extern "C" fn png_handle_cHRM(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut error: c_int = 0;
    let mut xy: png_xy = Default::default();
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
        png_chunk_benign_error(png_ptr, b"invalid\0".as_ptr() as png_const_charp);
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
    /* PNG_READ_RGB_TO_GRAY_SUPPORTED */
    /* There is no need to check sRGB here, cICP is NYI and iCCP is not
     * supported so just check mDCV.
     */
    if !png_file_has_chunk(png_ptr, PNG_INDEX_mDCV) {
        (*png_ptr).chromaticities = xy;
    }

    handled_ok
}

/* png_handle_sRGB */
unsafe extern "C" fn png_handle_sRGB(
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
    if intent as c_int > 3
    /*PNGv3 spec*/
    {
        png_chunk_benign_error(png_ptr, b"invalid\0".as_ptr() as png_const_charp);
        return handled_error;
    }

    png_set_sRGB(png_ptr, info_ptr, intent as c_int);
    /* NOTE: png_struct::chromaticities is not set here because the RGB to gray
     * coefficients are known without a need for the chromaticities.
     */

    /* PNG_READ_GAMMA_SUPPORTED */
    /* PNGv3: chunk precedence for gamma is cICP, [iCCP], sRGB, gAMA.  iCCP is
     * not supported by libpng so the only requirement is to check for cICP
     * setting the gamma (this is NYI, but this check is safe.)
     */
    if !png_file_has_chunk(png_ptr, PNG_INDEX_cICP) || (*png_ptr).chunk_gamma == 0 {
        (*png_ptr).chunk_gamma = PNG_GAMMA_sRGB_INVERSE;
    }

    handled_ok
}
