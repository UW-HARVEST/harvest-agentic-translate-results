/* pngrutil.c lines 1687..2262 */

/* png_handle_tRNS */
unsafe extern "C" fn png_handle_tRNS(
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
            png_chunk_benign_error(png_ptr, b"invalid\0".as_ptr() as png_const_charp);
            return handled_error;
        }

        png_crc_read(png_ptr, buf.as_mut_ptr(), 2);
        (*png_ptr).num_trans = 1;
        (*png_ptr).trans_color.gray = PNG_get_uint_16(buf.as_ptr());
    } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB {
        let mut buf: [png_byte; 6] = [0; 6];

        if length != 6 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, b"invalid\0".as_ptr() as png_const_charp);
            return handled_error;
        }

        png_crc_read(png_ptr, buf.as_mut_ptr(), length);
        (*png_ptr).num_trans = 1;
        (*png_ptr).trans_color.red = PNG_get_uint_16(buf.as_ptr());
        (*png_ptr).trans_color.green = PNG_get_uint_16(buf.as_ptr().add(2));
        (*png_ptr).trans_color.blue = PNG_get_uint_16(buf.as_ptr().add(4));
    } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        if ((*png_ptr).mode & PNG_HAVE_PLTE) == 0 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, b"out of place\0".as_ptr() as png_const_charp);
            return handled_error;
        }

        if length > (*png_ptr).num_palette as c_uint
            || length > PNG_MAX_PALETTE_LENGTH as c_uint
            || length == 0
        {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, b"invalid\0".as_ptr() as png_const_charp);
            return handled_error;
        }

        png_crc_read(png_ptr, readbuf.as_mut_ptr(), length);
        (*png_ptr).num_trans = length as png_uint_16;
    } else {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(
            png_ptr,
            b"invalid with alpha channel\0".as_ptr() as png_const_charp,
        );
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
        core::ptr::addr_of!((*png_ptr).trans_color),
    );
    handled_ok
}

/* png_handle_bKGD */
unsafe extern "C" fn png_handle_bKGD(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let truelen: c_uint;
    let mut buf: [png_byte; 6] = [0; 6];
    let mut background: png_color_16 = Default::default();

    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        if ((*png_ptr).mode & PNG_HAVE_PLTE) == 0 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, b"out of place\0".as_ptr() as png_const_charp);
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
        png_chunk_benign_error(png_ptr, b"invalid\0".as_ptr() as png_const_charp);
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

        if info_ptr != core::ptr::null_mut() && (*info_ptr).num_palette != 0 {
            if buf[0] as c_int >= (*info_ptr).num_palette as c_int {
                png_chunk_benign_error(png_ptr, b"invalid index\0".as_ptr() as png_const_charp);
                return handled_error;
            }

            background.red = (*(*png_ptr).palette.add(buf[0] as usize)).red as png_uint_16;
            background.green = (*(*png_ptr).palette.add(buf[0] as usize)).green as png_uint_16;
            background.blue = (*(*png_ptr).palette.add(buf[0] as usize)).blue as png_uint_16;
        } else {
            background.blue = 0;
            background.green = 0;
            background.red = 0;
        }

        background.gray = 0;
    }
    /* GRAY */
    else if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0 {
        if (*png_ptr).bit_depth as c_int <= 8 {
            if buf[0] != 0 || buf[1] as c_uint >= (1i32 << (*png_ptr).bit_depth as c_int) as c_uint {
                png_chunk_benign_error(png_ptr, b"invalid gray level\0".as_ptr() as png_const_charp);
                return handled_error;
            }
        }

        background.index = 0;
        background.gray = PNG_get_uint_16(buf.as_ptr());
        background.blue = background.gray;
        background.green = background.gray;
        background.red = background.gray;
    } else {
        if (*png_ptr).bit_depth as c_int <= 8 {
            if buf[0] != 0 || buf[2] != 0 || buf[4] != 0 {
                png_chunk_benign_error(png_ptr, b"invalid color\0".as_ptr() as png_const_charp);
                return handled_error;
            }
        }

        background.index = 0;
        background.red = PNG_get_uint_16(buf.as_ptr());
        background.green = PNG_get_uint_16(buf.as_ptr().add(2));
        background.blue = PNG_get_uint_16(buf.as_ptr().add(4));
        background.gray = 0;
    }

    png_set_bKGD(png_ptr, info_ptr, &background);
    handled_ok
}

/* png_handle_cICP */
unsafe extern "C" fn png_handle_cICP(
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
    /* PNG_READ_RGB_TO_GRAY_SUPPORTED */
    if !png_file_has_chunk(png_ptr, PNG_INDEX_mDCV) {
        /* TODO: png_ptr->chromaticities = chromaticities; */
    }

    /* PNG_READ_GAMMA_SUPPORTED */
    /* PNGv3: chunk precedence for gamma is cICP, [iCCP], sRGB, gAMA.  cICP is
     * at the head so simply set the gamma if it can be determined.  If not
     * chunk_gamma remains unchanged; sRGB and gAMA handling check it for
     * being zero.
     */
    /* TODO: set png_struct::chunk_gamma when possible */

    handled_ok
}

/* png_handle_cLLI */
unsafe extern "C" fn png_handle_cLLI(
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
        PNG_get_uint_32(buf.as_ptr()),
        PNG_get_uint_32(buf.as_ptr().add(4)),
    );
    handled_ok
}

/* png_handle_mDCV */
unsafe extern "C" fn png_handle_mDCV(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut chromaticities: png_xy = Default::default();
    let mut buf: [png_byte; 24] = [0; 24];

    png_crc_read(png_ptr, buf.as_mut_ptr(), 24);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    /* The error checking happens here, this puts it in just one place.  The
     * odd /50000 scaling factor makes it more difficult but the (x.y) values are
     * only two bytes so a <<1 is safe.
     *
     * WARNING: the PNG specification defines the cHRM chunk to **start** with
     * the white point (x,y).  The W3C PNG v3 specification puts the white point
     * **after* R,G,B.  The x,y values in mDCV are also scaled by 50,000 and
     * stored in just two bytes, whereas those in cHRM are scaled by 100,000 and
     * stored in four bytes.  This is very, very confusing.  These APIs remove
     * the confusion by copying the existing, well established, API.
     */
    /* red x */
    chromaticities.redx = ((PNG_get_uint_16(buf.as_ptr().add(0)) as c_int) << 1) as png_fixed_point;
    /* red y */
    chromaticities.redy = ((PNG_get_uint_16(buf.as_ptr().add(2)) as c_int) << 1) as png_fixed_point;
    /* green x */
    chromaticities.greenx =
        ((PNG_get_uint_16(buf.as_ptr().add(4)) as c_int) << 1) as png_fixed_point;
    /* green y */
    chromaticities.greeny =
        ((PNG_get_uint_16(buf.as_ptr().add(6)) as c_int) << 1) as png_fixed_point;
    /* blue x */
    chromaticities.bluex = ((PNG_get_uint_16(buf.as_ptr().add(8)) as c_int) << 1) as png_fixed_point;
    /* blue y */
    chromaticities.bluey =
        ((PNG_get_uint_16(buf.as_ptr().add(10)) as c_int) << 1) as png_fixed_point;
    /* white x */
    chromaticities.whitex =
        ((PNG_get_uint_16(buf.as_ptr().add(12)) as c_int) << 1) as png_fixed_point;
    /* white y */
    chromaticities.whitey =
        ((PNG_get_uint_16(buf.as_ptr().add(14)) as c_int) << 1) as png_fixed_point;

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
        PNG_get_uint_32(buf.as_ptr().add(16)), /* peak luminance */
        PNG_get_uint_32(buf.as_ptr().add(20)),
    ); /* minimum perceivable luminance */

    /* We only use 'chromaticities' for RGB to gray */
    /* PNG_READ_RGB_TO_GRAY_SUPPORTED */
    (*png_ptr).chromaticities = chromaticities;

    handled_ok
}

/* png_handle_eXIf */
unsafe extern "C" fn png_handle_eXIf(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let buffer: png_bytep;

    buffer = png_read_buffer(png_ptr, length as png_alloc_size_t);

    if buffer == core::ptr::null_mut() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, b"out of memory\0".as_ptr() as png_const_charp);
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
        let header: png_uint_32 = PNG_get_uint_32(buffer);

        /* These numbers are copied from the PNGv3 spec: */
        if header != 0x49492A00 && header != 0x4D4D002A {
            png_chunk_benign_error(png_ptr, b"invalid\0".as_ptr() as png_const_charp);
            return handled_error;
        }
    }

    png_set_eXIf_1(png_ptr, info_ptr, length, buffer);
    handled_ok
}

/* png_handle_hIST */
unsafe extern "C" fn png_handle_hIST(
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
     *
     * TODO: maybe use png_uint_32 anyway, not unsigned int, to reduce the
     * casts.
     */
    num = (length as c_uint) / 2;

    if length != num.wrapping_mul(2)
        || num != (*png_ptr).num_palette as c_uint
        || num > PNG_MAX_PALETTE_LENGTH as c_uint
    {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, b"invalid\0".as_ptr() as png_const_charp);
        return handled_error;
    }

    i = 0;
    while i < num {
        let mut buf: [png_byte; 2] = [0; 2];

        png_crc_read(png_ptr, buf.as_mut_ptr(), 2);
        readbuf[i as usize] = PNG_get_uint_16(buf.as_ptr());

        i += 1;
    }

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    png_set_hIST(png_ptr, info_ptr, readbuf.as_ptr());
    handled_ok
}

/* png_handle_pHYs */
unsafe extern "C" fn png_handle_pHYs(
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

    res_x = PNG_get_uint_32(buf.as_ptr());
    res_y = PNG_get_uint_32(buf.as_ptr().add(4));
    unit_type = buf[8] as c_int;
    png_set_pHYs(png_ptr, info_ptr, res_x, res_y, unit_type);
    handled_ok
}

/* png_handle_oFFs */
unsafe extern "C" fn png_handle_oFFs(
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

    offset_x = PNG_get_int_32(buf.as_ptr());
    offset_y = PNG_get_int_32(buf.as_ptr().add(4));
    unit_type = buf[8] as c_int;
    png_set_oFFs(png_ptr, info_ptr, offset_x, offset_y, unit_type);
    handled_ok
}

/* Read the pCAL chunk (described in the PNG Extensions document) */
/* png_handle_pCAL */
unsafe extern "C" fn png_handle_pCAL(
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

    buffer = png_read_buffer(png_ptr, length.wrapping_add(1) as png_alloc_size_t);

    if buffer == core::ptr::null_mut() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, b"out of memory\0".as_ptr() as png_const_charp);
        return handled_error;
    }

    png_crc_read(png_ptr, buffer, length);

    if png_crc_finish(png_ptr, 0) != 0 {
        return handled_error;
    }

    *buffer.add(length as usize) = 0; /* Null terminate the last string */

    /* Finding end of pCAL purpose string */
    buf = buffer;
    while *buf != 0
    /* Empty loop */
    {
        buf = buf.add(1);
    }

    endptr = buffer.add(length as usize);

    /* We need to have at least 12 bytes after the purpose string
     * in order to get the parameter information.
     */
    if endptr.offset_from(buf) <= 12 {
        png_chunk_benign_error(png_ptr, b"invalid\0".as_ptr() as png_const_charp);
        return handled_error;
    }

    /* Reading pCAL X0, X1, type, nparams, and units */
    X0 = PNG_get_int_32((buf as png_bytep).add(1));
    X1 = PNG_get_int_32((buf as png_bytep).add(5));
    type_ = *buf.add(9);
    nparams = *buf.add(10);
    units = buf.add(11);

    /* Checking pCAL equation type and number of parameters */
    /* Check that we have the right number of parameters for known
     * equation types.
     */
    if (type_ as c_int == PNG_EQUATION_LINEAR && nparams as c_int != 2)
        || (type_ as c_int == PNG_EQUATION_BASE_E && nparams as c_int != 3)
        || (type_ as c_int == PNG_EQUATION_ARBITRARY && nparams as c_int != 3)
        || (type_ as c_int == PNG_EQUATION_HYPERBOLIC && nparams as c_int != 4)
    {
        png_chunk_benign_error(png_ptr, b"invalid parameter count\0".as_ptr() as png_const_charp);
        return handled_error;
    } else if type_ as c_int >= PNG_EQUATION_LAST {
        png_chunk_benign_error(
            png_ptr,
            b"unrecognized equation type\0".as_ptr() as png_const_charp,
        );
    }

    buf = units;
    while *buf != 0
    /* Empty loop to move past the units string. */
    {
        buf = buf.add(1);
    }

    /* Allocating pCAL parameters array */

    params = png_malloc_warn(
        png_ptr,
        (nparams as usize).wrapping_mul(core::mem::size_of::<png_charp>()),
    ) as png_charpp;

    if params == core::ptr::null_mut() {
        png_chunk_benign_error(png_ptr, b"out of memory\0".as_ptr() as png_const_charp);
        return handled_error;
    }

    /* Get pointers to the start of each parameter string. */
    i = 0;
    while i < nparams as c_int {
        buf = buf.add(1); /* Skip the null string terminator from previous parameter. */

        *params.offset(i as isize) = buf as png_charp;
        while buf <= endptr && *buf != 0
        /* Empty loop to move past each parameter string */
        {
            buf = buf.add(1);
        }

        /* Make sure we haven't run out of data yet */
        if buf > endptr {
            png_free(png_ptr, params as png_voidp);
            png_chunk_benign_error(png_ptr, b"invalid data\0".as_ptr() as png_const_charp);
            return handled_error;
        }

        i += 1;
    }

    png_set_pCAL(
        png_ptr,
        info_ptr,
        buffer as png_const_charp,
        X0,
        X1,
        type_ as c_int,
        nparams as c_int,
        units as png_const_charp,
        params,
    );

    /* TODO: BUG: png_set_pCAL calls png_chunk_report which, in this case, calls
     * png_benign_error and that can error out.
     *
     * png_read_buffer needs to be allocated with space for both nparams and the
     * parameter strings.  Not hard to do.
     */
    png_free(png_ptr, params as png_voidp);
    handled_ok
}
