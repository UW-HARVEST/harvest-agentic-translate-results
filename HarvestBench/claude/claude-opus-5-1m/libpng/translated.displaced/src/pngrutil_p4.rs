// pngrutil.c - utilities to read a PNG file
//
// Chunk readers: sPLT, tRNS, bKGD, cICP, cLLI, mDCV, eXIf, hIST, pHYs, oFFs.
//
// This code is released under the libpng license.
// For conditions of distribution and use, see the disclaimer
// and license in png.h

use crate::*;

/* Note: this does not properly handle chunks that are > 64K under DOS */
unsafe extern "C" fn png_handle_sPLT(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buffer: png_bytep;
    let mut entry_start: png_bytep;
    let mut new_palette: png_sPLT_t = core::mem::zeroed();
    let mut pp: png_sPLT_entryp;
    let mut data_length: png_uint_32;
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

        (*png_ptr).user_chunk_cache_max = (*png_ptr).user_chunk_cache_max.wrapping_sub(1);

        if (*png_ptr).user_chunk_cache_max == 1 {
            png_warning(png_ptr, cstr!("No space in chunk cache for sPLT"));
            png_crc_finish(png_ptr, length);
            return handled_error;
        }
    }

    buffer = png_read_buffer(png_ptr, length.wrapping_add(1) as png_alloc_size_t);
    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr!("out of memory"));
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
        png_warning(png_ptr, cstr!("malformed sPLT chunk"));
        return handled_error;
    }

    new_palette.depth = *entry_start;
    entry_start = entry_start.add(1);
    entry_size = if new_palette.depth == 8 { 6 } else { 10 };
    /* This must fit in a png_uint_32 because it is derived from the original
     * chunk data length.
     */
    data_length =
        length.wrapping_sub(((entry_start as isize) - (buffer as isize)) as png_uint_32);

    /* Integrity-check the data length */
    if (data_length % (entry_size as c_uint)) != 0 {
        png_warning(png_ptr, cstr!("sPLT chunk has bad length"));
        return handled_error;
    }

    dl = (data_length / (entry_size as c_uint)) as png_uint_32;
    max_dl = PNG_SIZE_MAX / core::mem::size_of::<png_sPLT_entry>();

    if (dl as usize) > max_dl {
        png_warning(png_ptr, cstr!("sPLT chunk too long"));
        return handled_error;
    }

    new_palette.nentries = (data_length / (entry_size as c_uint)) as png_int_32;

    new_palette.entries = png_malloc_warn(
        png_ptr,
        (new_palette.nentries as png_alloc_size_t) * core::mem::size_of::<png_sPLT_entry>(),
    ) as png_sPLT_entryp;

    if new_palette.entries.is_null() {
        png_warning(png_ptr, cstr!("sPLT chunk requires too much memory"));
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

    png_set_sPLT(png_ptr, info_ptr, &new_palette as *const png_sPLT_t, 1);

    png_free(png_ptr, new_palette.entries as png_voidp);
    handled_ok
}

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
            png_chunk_benign_error(png_ptr, cstr!("invalid"));
            return handled_error;
        }

        png_crc_read(png_ptr, buf.as_mut_ptr(), 2);
        (*png_ptr).num_trans = 1;
        (*png_ptr).trans_color.gray = png_get_uint_16(buf.as_ptr());
    } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB {
        let mut buf: [png_byte; 6] = [0; 6];

        if length != 6 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr!("invalid"));
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
            png_chunk_benign_error(png_ptr, cstr!("out of place"));
            return handled_error;
        }

        if length > (*png_ptr).num_palette as c_uint
            || length > PNG_MAX_PALETTE_LENGTH as c_uint
            || length == 0
        {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr!("invalid"));
            return handled_error;
        }

        png_crc_read(png_ptr, readbuf.as_mut_ptr(), length);
        (*png_ptr).num_trans = length as png_uint_16;
    } else {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr!("invalid with alpha channel"));
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
        &(*png_ptr).trans_color as *const png_color_16,
    );
    handled_ok
}

unsafe extern "C" fn png_handle_bKGD(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let truelen: c_uint;
    let mut buf: [png_byte; 6] = [0; 6];
    let mut background: png_color_16 = core::mem::zeroed();

    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        if ((*png_ptr).mode & PNG_HAVE_PLTE) == 0 {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, cstr!("out of place"));
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
        png_chunk_benign_error(png_ptr, cstr!("invalid"));
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
            if buf[0] as c_int >= (*info_ptr).num_palette as c_int {
                png_chunk_benign_error(png_ptr, cstr!("invalid index"));
                return handled_error;
            }

            background.red = (*(*png_ptr).palette.offset(buf[0] as isize)).red as png_uint_16;
            background.green = (*(*png_ptr).palette.offset(buf[0] as isize)).green as png_uint_16;
            background.blue = (*(*png_ptr).palette.offset(buf[0] as isize)).blue as png_uint_16;
        } else {
            background.blue = 0;
            background.green = background.blue;
            background.red = background.green;
        }

        background.gray = 0;
    } else if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0 {
        /* GRAY */
        if (*png_ptr).bit_depth <= 8 {
            if buf[0] != 0 || (buf[1] as c_uint) >= ((1 << (*png_ptr).bit_depth) as c_uint) {
                png_chunk_benign_error(png_ptr, cstr!("invalid gray level"));
                return handled_error;
            }
        }

        background.index = 0;
        background.gray = png_get_uint_16(buf.as_ptr());
        background.blue = background.gray;
        background.green = background.blue;
        background.red = background.green;
    } else {
        if (*png_ptr).bit_depth <= 8 {
            if buf[0] != 0 || buf[2] != 0 || buf[4] != 0 {
                png_chunk_benign_error(png_ptr, cstr!("invalid color"));
                return handled_error;
            }
        }

        background.index = 0;
        background.red = png_get_uint_16(buf.as_ptr());
        background.green = png_get_uint_16(buf.as_ptr().add(2));
        background.blue = png_get_uint_16(buf.as_ptr().add(4));
        background.gray = 0;
    }

    png_set_bKGD(png_ptr, info_ptr, &background as *const png_color_16);
    handled_ok
}

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

    if !png_file_has_chunk(png_ptr, PNG_INDEX_mDCV) {
        /* TODO: png_ptr->chromaticities = chromaticities; */
    }

    /* PNGv3: chunk precedence for gamma is cICP, [iCCP], sRGB, gAMA.  cICP is
     * at the head so simply set the gamma if it can be determined.  If not
     * chunk_gamma remains unchanged; sRGB and gAMA handling check it for
     * being zero.
     */
    /* TODO: set png_struct::chunk_gamma when possible */

    handled_ok
}

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
        png_get_uint_32(buf.as_ptr()),
        png_get_uint_32(buf.as_ptr().add(4)),
    );
    handled_ok
}

unsafe extern "C" fn png_handle_mDCV(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut chromaticities: png_xy = core::mem::zeroed();
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
    chromaticities.redx = (png_get_uint_16(buf.as_ptr().add(0)) as c_int) << 1; /* red x */
    chromaticities.redy = (png_get_uint_16(buf.as_ptr().add(2)) as c_int) << 1; /* red y */
    chromaticities.greenx = (png_get_uint_16(buf.as_ptr().add(4)) as c_int) << 1; /* green x */
    chromaticities.greeny = (png_get_uint_16(buf.as_ptr().add(6)) as c_int) << 1; /* green y */
    chromaticities.bluex = (png_get_uint_16(buf.as_ptr().add(8)) as c_int) << 1; /* blue x */
    chromaticities.bluey = (png_get_uint_16(buf.as_ptr().add(10)) as c_int) << 1; /* blue y */
    chromaticities.whitex = (png_get_uint_16(buf.as_ptr().add(12)) as c_int) << 1; /* white x */
    chromaticities.whitey = (png_get_uint_16(buf.as_ptr().add(14)) as c_int) << 1; /* white y */

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
        png_get_uint_32(buf.as_ptr().add(16)),  /* peak luminance */
        png_get_uint_32(buf.as_ptr().add(20)),
    ); /* minimum perceivable luminance */

    /* We only use 'chromaticities' for RGB to gray */

    (*png_ptr).chromaticities = chromaticities;

    handled_ok
}

unsafe extern "C" fn png_handle_eXIf(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    let mut buffer: png_bytep = core::ptr::null_mut();

    buffer = png_read_buffer(png_ptr, length as png_alloc_size_t);

    if buffer.is_null() {
        png_crc_finish(png_ptr, length);
        png_chunk_benign_error(png_ptr, cstr!("out of memory"));
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
            png_chunk_benign_error(png_ptr, cstr!("invalid"));
            return handled_error;
        }
    }

    png_set_eXIf_1(png_ptr, info_ptr, length, buffer);
    handled_ok
}

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
        png_chunk_benign_error(png_ptr, cstr!("invalid"));
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

    res_x = png_get_uint_32(buf.as_ptr());
    res_y = png_get_uint_32(buf.as_ptr().add(4));
    unit_type = buf[8] as c_int;
    png_set_pHYs(png_ptr, info_ptr, res_x, res_y, unit_type);
    handled_ok
}

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

    offset_x = png_get_int_32(buf.as_ptr());
    offset_y = png_get_int_32(buf.as_ptr().add(4));
    unit_type = buf[8] as c_int;
    png_set_oFFs(png_ptr, info_ptr, offset_x, offset_y, unit_type);
    handled_ok
}
