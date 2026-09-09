//! Translation of c_src/src/pngwutil.c lines 1449..2780
use crate::prelude::*;

/* Write the cICP data */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_cICP(
    png_ptr: png_structrp,
    colour_primaries: png_byte,
    transfer_function: png_byte,
    matrix_coefficients: png_byte,
    video_full_range_flag: png_byte,
) {
    let mut buf: [png_byte; 4] = [0; 4];

    png_write_chunk_header(png_ptr, png_cICP, 4);

    buf[0] = colour_primaries;
    buf[1] = transfer_function;
    buf[2] = matrix_coefficients;
    buf[3] = video_full_range_flag;
    png_write_chunk_data(png_ptr, buf.as_ptr(), 4);

    png_write_chunk_end(png_ptr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_cLLI_fixed(
    png_ptr: png_structrp,
    maxCLL: png_uint_32,
    maxFALL: png_uint_32,
) {
    let mut buf: [png_byte; 8] = [0; 8];

    png_save_uint_32(buf.as_mut_ptr(), maxCLL);
    png_save_uint_32(buf.as_mut_ptr().add(4), maxFALL);

    png_write_complete_chunk(png_ptr, png_cLLI, buf.as_ptr(), 8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_mDCV_fixed(
    png_ptr: png_structrp,
    red_x: png_uint_16,
    red_y: png_uint_16,
    green_x: png_uint_16,
    green_y: png_uint_16,
    blue_x: png_uint_16,
    blue_y: png_uint_16,
    white_x: png_uint_16,
    white_y: png_uint_16,
    maxDL: png_uint_32,
    minDL: png_uint_32,
) {
    let mut buf: [png_byte; 24] = [0; 24];

    png_save_uint_16(buf.as_mut_ptr().add(0), red_x as c_uint);
    png_save_uint_16(buf.as_mut_ptr().add(2), red_y as c_uint);
    png_save_uint_16(buf.as_mut_ptr().add(4), green_x as c_uint);
    png_save_uint_16(buf.as_mut_ptr().add(6), green_y as c_uint);
    png_save_uint_16(buf.as_mut_ptr().add(8), blue_x as c_uint);
    png_save_uint_16(buf.as_mut_ptr().add(10), blue_y as c_uint);
    png_save_uint_16(buf.as_mut_ptr().add(12), white_x as c_uint);
    png_save_uint_16(buf.as_mut_ptr().add(14), white_y as c_uint);
    png_save_uint_32(buf.as_mut_ptr().add(16), maxDL);
    png_save_uint_32(buf.as_mut_ptr().add(20), minDL);

    png_write_complete_chunk(png_ptr, png_mDCV, buf.as_ptr(), 24);
}

/* Write the Exif data */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_eXIf(png_ptr: png_structrp, exif: png_bytep, num_exif: c_int) {
    let mut i: c_int;
    let mut buf: [png_byte; 1] = [0; 1];

    png_write_chunk_header(png_ptr, png_eXIf, num_exif as png_uint_32);

    i = 0;
    while i < num_exif {
        buf[0] = *exif.add(i as usize);
        png_write_chunk_data(png_ptr, buf.as_ptr(), 1);
        i += 1;
    }

    png_write_chunk_end(png_ptr);
}

/* Write the histogram */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_hIST(
    png_ptr: png_structrp,
    hist: png_const_uint_16p,
    num_hist: c_int,
) {
    let mut i: c_int;
    let mut buf: [png_byte; 3] = [0; 3];

    if num_hist > (*png_ptr).num_palette as c_int {
        png_warning(
            png_ptr,
            cstr(b"Invalid number of histogram entries specified\0"),
        );
        return;
    }

    png_write_chunk_header(png_ptr, png_hIST, (num_hist * 2) as png_uint_32);

    i = 0;
    while i < num_hist {
        png_save_uint_16(buf.as_mut_ptr(), *hist.add(i as usize) as c_uint);
        png_write_chunk_data(png_ptr, buf.as_ptr(), 2);
        i += 1;
    }

    png_write_chunk_end(png_ptr);
}

/* Write a tEXt chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_tEXt(
    png_ptr: png_structrp,
    key: png_const_charp,
    text: png_const_charp,
    mut text_len: usize,
) {
    let key_len: png_uint_32;
    let mut new_key: [png_byte; 80] = [0; 80];

    key_len = png_check_keyword(png_ptr, key, new_key.as_mut_ptr());

    if key_len == 0 {
        png_error(png_ptr, cstr(b"tEXt: invalid keyword\0"));
    }

    if text.is_null() || *text == b'\0' as c_char {
        text_len = 0;
    } else {
        text_len = strlen(text);
    }

    if text_len > (PNG_UINT_31_MAX - (key_len + 1)) as usize {
        png_error(png_ptr, cstr(b"tEXt: text too long\0"));
    }

    /* Make sure we include the 0 after the key */
    png_write_chunk_header(
        png_ptr,
        png_tEXt,
        (key_len as usize + text_len + 1) as png_uint_32,
    );
    /*
     * We leave it to the application to meet PNG-1.0 requirements on the
     * contents of the text.
     */
    png_write_chunk_data(png_ptr, new_key.as_ptr(), (key_len + 1) as usize);

    if text_len != 0 {
        png_write_chunk_data(png_ptr, text as png_const_bytep, text_len);
    }

    png_write_chunk_end(png_ptr);
}

/* Write a compressed text chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_zTXt(
    png_ptr: png_structrp,
    key: png_const_charp,
    text: png_const_charp,
    compression: c_int,
) {
    let mut key_len: png_uint_32;
    let mut new_key: [png_byte; 81] = [0; 81];
    let mut comp: compression_state = core::mem::zeroed();

    if compression == PNG_TEXT_COMPRESSION_NONE {
        png_write_tEXt(png_ptr, key, text, 0);
        return;
    }

    if compression != PNG_TEXT_COMPRESSION_zTXt {
        png_error(png_ptr, cstr(b"zTXt: invalid compression type\0"));
    }

    key_len = png_check_keyword(png_ptr, key, new_key.as_mut_ptr());

    if key_len == 0 {
        png_error(png_ptr, cstr(b"zTXt: invalid keyword\0"));
    }

    /* Add the compression method and 1 for the keyword separator. */
    key_len += 1;
    new_key[key_len as usize] = PNG_COMPRESSION_TYPE_BASE as png_byte;
    key_len += 1;

    /* Compute the compressed data; do it now for the length */
    png_text_compress_init(
        &mut comp,
        text as png_const_bytep,
        if text.is_null() { 0 } else { strlen(text) },
    );

    if png_text_compress(png_ptr, png_zTXt, &mut comp, key_len) != Z_OK {
        png_error(png_ptr, (*png_ptr).zstream.msg);
    }

    /* Write start of chunk */
    png_write_chunk_header(png_ptr, png_zTXt, key_len + comp.output_len);

    /* Write key */
    png_write_chunk_data(png_ptr, new_key.as_ptr(), key_len as usize);

    /* Write the compressed data */
    png_write_compressed_data_out(png_ptr, &mut comp);

    /* Close the chunk */
    png_write_chunk_end(png_ptr);
}

/* Write an iTXt chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_iTXt(
    png_ptr: png_structrp,
    mut compression: c_int,
    key: png_const_charp,
    mut lang: png_const_charp,
    mut lang_key: png_const_charp,
    mut text: png_const_charp,
) {
    let mut key_len: png_uint_32;
    let mut prefix_len: png_uint_32;
    let lang_len: usize;
    let lang_key_len: usize;
    let mut new_key: [png_byte; 82] = [0; 82];
    let mut comp: compression_state = core::mem::zeroed();

    key_len = png_check_keyword(png_ptr, key, new_key.as_mut_ptr());

    if key_len == 0 {
        png_error(png_ptr, cstr(b"iTXt: invalid keyword\0"));
    }

    /* Set the compression flag */
    match compression {
        PNG_ITXT_COMPRESSION_NONE | PNG_TEXT_COMPRESSION_NONE => {
            key_len += 1;
            new_key[key_len as usize] = 0; /* no compression */
            compression = 0;
        }

        PNG_TEXT_COMPRESSION_zTXt | PNG_ITXT_COMPRESSION_zTXt => {
            key_len += 1;
            new_key[key_len as usize] = 1; /* compressed */
            compression = 1;
        }

        _ => {
            png_error(png_ptr, cstr(b"iTXt: invalid compression\0"));
        }
    }

    key_len += 1;
    new_key[key_len as usize] = PNG_COMPRESSION_TYPE_BASE as png_byte;
    key_len += 1; /* for the keyword separator */

    /* We leave it to the application to meet PNG-1.0 requirements on the
     * contents of the text.
     */
    if lang.is_null() {
        lang = cstr(b"\0");
    } /* empty language is valid */
    lang_len = strlen(lang) + 1;
    if lang_key.is_null() {
        lang_key = cstr(b"\0");
    } /* may be empty */
    lang_key_len = strlen(lang_key) + 1;
    if text.is_null() {
        text = cstr(b"\0");
    } /* may be empty */

    prefix_len = key_len;
    if lang_len > (PNG_UINT_31_MAX - prefix_len) as usize {
        prefix_len = PNG_UINT_31_MAX;
    } else {
        prefix_len = (prefix_len as usize + lang_len) as png_uint_32;
    }

    if lang_key_len > (PNG_UINT_31_MAX - prefix_len) as usize {
        prefix_len = PNG_UINT_31_MAX;
    } else {
        prefix_len = (prefix_len as usize + lang_key_len) as png_uint_32;
    }

    png_text_compress_init(&mut comp, text as png_const_bytep, strlen(text));

    if compression != 0 {
        if png_text_compress(png_ptr, png_iTXt, &mut comp, prefix_len) != Z_OK {
            png_error(png_ptr, (*png_ptr).zstream.msg);
        }
    } else {
        if comp.input_len > (PNG_UINT_31_MAX - prefix_len) as usize {
            png_error(png_ptr, cstr(b"iTXt: uncompressed text too long\0"));
        }

        /* So the string will fit in a chunk: */
        comp.output_len = comp.input_len as png_uint_32;
    }

    png_write_chunk_header(png_ptr, png_iTXt, comp.output_len + prefix_len);

    png_write_chunk_data(png_ptr, new_key.as_ptr(), key_len as usize);

    png_write_chunk_data(png_ptr, lang as png_const_bytep, lang_len);

    png_write_chunk_data(png_ptr, lang_key as png_const_bytep, lang_key_len);

    if compression != 0 {
        png_write_compressed_data_out(png_ptr, &mut comp);
    } else {
        png_write_chunk_data(png_ptr, text as png_const_bytep, comp.output_len as usize);
    }

    png_write_chunk_end(png_ptr);
}

/* Write the oFFs chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_oFFs(
    png_ptr: png_structrp,
    x_offset: png_int_32,
    y_offset: png_int_32,
    unit_type: c_int,
) {
    let mut buf: [png_byte; 9] = [0; 9];

    if unit_type >= PNG_OFFSET_LAST {
        png_warning(png_ptr, cstr(b"Unrecognized unit type for oFFs chunk\0"));
    }

    png_save_int_32(buf.as_mut_ptr(), x_offset);
    png_save_int_32(buf.as_mut_ptr().add(4), y_offset);
    buf[8] = unit_type as png_byte;

    png_write_complete_chunk(png_ptr, png_oFFs, buf.as_ptr(), 9);
}

/* Write the pCAL chunk (described in the PNG extensions document) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_pCAL(
    png_ptr: png_structrp,
    purpose: png_charp,
    X0: png_int_32,
    X1: png_int_32,
    type_: c_int,
    nparams: c_int,
    units: png_const_charp,
    params: png_charpp,
) {
    let mut purpose_len: png_uint_32;
    let units_len: usize;
    let mut total_len: usize;
    let params_len: *mut usize;
    let mut buf: [png_byte; 10] = [0; 10];
    let mut new_purpose: [png_byte; 80] = [0; 80];
    let mut i: c_int;

    if type_ >= PNG_EQUATION_LAST {
        png_error(
            png_ptr,
            cstr(b"Unrecognized equation type for pCAL chunk\0"),
        );
    }

    purpose_len = png_check_keyword(png_ptr, purpose, new_purpose.as_mut_ptr());

    if purpose_len == 0 {
        png_error(png_ptr, cstr(b"pCAL: invalid keyword\0"));
    }

    purpose_len += 1; /* terminator */

    units_len = strlen(units) + (if nparams == 0 { 0 } else { 1 });
    total_len = purpose_len as usize + units_len + 10;

    params_len = png_malloc(
        png_ptr,
        (nparams as png_alloc_size_t) * core::mem::size_of::<usize>(),
    ) as *mut usize;

    /* Find the length of each parameter, making sure we don't count the
     * null terminator for the last parameter.
     */
    i = 0;
    while i < nparams {
        *params_len.add(i as usize) =
            strlen(*params.add(i as usize)) + (if i == nparams - 1 { 0 } else { 1 });
        total_len += *params_len.add(i as usize);
        i += 1;
    }

    png_write_chunk_header(png_ptr, png_pCAL, total_len as png_uint_32);
    png_write_chunk_data(png_ptr, new_purpose.as_ptr(), purpose_len as usize);
    png_save_int_32(buf.as_mut_ptr(), X0);
    png_save_int_32(buf.as_mut_ptr().add(4), X1);
    buf[8] = type_ as png_byte;
    buf[9] = nparams as png_byte;
    png_write_chunk_data(png_ptr, buf.as_ptr(), 10);
    png_write_chunk_data(png_ptr, units as png_const_bytep, units_len);

    i = 0;
    while i < nparams {
        png_write_chunk_data(
            png_ptr,
            *params.add(i as usize) as png_const_bytep,
            *params_len.add(i as usize),
        );
        i += 1;
    }

    png_free(png_ptr, params_len as png_voidp);
    png_write_chunk_end(png_ptr);
}

/* Write the sCAL chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_sCAL_s(
    png_ptr: png_structrp,
    unit: c_int,
    width: png_const_charp,
    height: png_const_charp,
) {
    let mut buf: [png_byte; 64] = [0; 64];
    let wlen: usize;
    let hlen: usize;
    let total_len: usize;

    wlen = strlen(width);
    hlen = strlen(height);
    total_len = wlen + hlen + 2;

    if total_len > 64 {
        png_warning(png_ptr, cstr(b"Can't write sCAL (buffer too small)\0"));
        return;
    }

    buf[0] = unit as png_byte;
    memcpy(
        buf.as_mut_ptr().add(1) as *mut c_void,
        width as *const c_void,
        wlen + 1,
    ); /* Append the '\0' here */
    memcpy(
        buf.as_mut_ptr().add(wlen + 2) as *mut c_void,
        height as *const c_void,
        hlen,
    ); /* Do NOT append the '\0' here */

    png_write_complete_chunk(png_ptr, png_sCAL, buf.as_ptr(), total_len);
}

/* Write the pHYs chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_pHYs(
    png_ptr: png_structrp,
    x_pixels_per_unit: png_uint_32,
    y_pixels_per_unit: png_uint_32,
    unit_type: c_int,
) {
    let mut buf: [png_byte; 9] = [0; 9];

    if unit_type >= PNG_RESOLUTION_LAST {
        png_warning(png_ptr, cstr(b"Unrecognized unit type for pHYs chunk\0"));
    }

    png_save_uint_32(buf.as_mut_ptr(), x_pixels_per_unit);
    png_save_uint_32(buf.as_mut_ptr().add(4), y_pixels_per_unit);
    buf[8] = unit_type as png_byte;

    png_write_complete_chunk(png_ptr, png_pHYs, buf.as_ptr(), 9);
}

/* Write the tIME chunk.  Use either png_convert_from_struct_tm()
 * or png_convert_from_time_t(), or fill in the structure yourself.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_tIME(png_ptr: png_structrp, mod_time: png_const_timep) {
    let mut buf: [png_byte; 7] = [0; 7];

    if (*mod_time).month > 12
        || (*mod_time).month < 1
        || (*mod_time).day > 31
        || (*mod_time).day < 1
        || (*mod_time).hour > 23
        || (*mod_time).second > 60
    {
        png_warning(png_ptr, cstr(b"Invalid time specified for tIME chunk\0"));
        return;
    }

    png_save_uint_16(buf.as_mut_ptr(), (*mod_time).year as c_uint);
    buf[2] = (*mod_time).month;
    buf[3] = (*mod_time).day;
    buf[4] = (*mod_time).hour;
    buf[5] = (*mod_time).minute;
    buf[6] = (*mod_time).second;

    png_write_complete_chunk(png_ptr, png_tIME, buf.as_ptr(), 7);
}

/* Initializes the row writing capability of libpng */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_start_row(png_ptr: png_structrp) {
    let buf_size: png_alloc_size_t;
    let usr_pixel_depth: c_int;

    let mut filters: png_byte;

    usr_pixel_depth = (*png_ptr).usr_channels as c_int * (*png_ptr).usr_bit_depth as c_int;
    buf_size = PNG_ROWBYTES(usr_pixel_depth as usize, (*png_ptr).width as usize) + 1;

    /* 1.5.6: added to allow checking in the row write code. */
    (*png_ptr).transformed_pixel_depth = (*png_ptr).pixel_depth;
    (*png_ptr).maximum_pixel_depth = usr_pixel_depth as png_byte;

    /* Set up row buffer */
    (*png_ptr).row_buf = png_malloc(png_ptr, buf_size) as png_bytep;

    *(*png_ptr).row_buf.add(0) = PNG_FILTER_VALUE_NONE as png_byte;

    filters = (*png_ptr).do_filter;

    if (*png_ptr).height == 1 {
        filters &= (0xff & !(PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH)) as png_byte;
    }

    if (*png_ptr).width == 1 {
        filters &= (0xff & !(PNG_FILTER_SUB | PNG_FILTER_AVG | PNG_FILTER_PAETH)) as png_byte;
    }

    if filters == 0 {
        filters = PNG_FILTER_NONE as png_byte;
    }

    (*png_ptr).do_filter = filters;

    if (filters as c_int & (PNG_FILTER_SUB | PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH))
        != 0
        && (*png_ptr).try_row.is_null()
    {
        let mut num_filters: c_int = 0;

        (*png_ptr).try_row = png_malloc(png_ptr, buf_size) as png_bytep;

        if filters as c_int & PNG_FILTER_SUB != 0 {
            num_filters += 1;
        }

        if filters as c_int & PNG_FILTER_UP != 0 {
            num_filters += 1;
        }

        if filters as c_int & PNG_FILTER_AVG != 0 {
            num_filters += 1;
        }

        if filters as c_int & PNG_FILTER_PAETH != 0 {
            num_filters += 1;
        }

        if num_filters > 1 {
            (*png_ptr).tst_row = png_malloc(png_ptr, buf_size) as png_bytep;
        }
    }

    /* We only need to keep the previous row if we are using one of the following
     * filters.
     */
    if (filters as c_int & (PNG_FILTER_AVG | PNG_FILTER_UP | PNG_FILTER_PAETH)) != 0 {
        (*png_ptr).prev_row = png_calloc(png_ptr, buf_size) as png_bytep;
    }

    /* If interlaced, we need to set up width and height of pass */
    if (*png_ptr).interlaced != 0 {
        if ((*png_ptr).transformations & PNG_INTERLACE) == 0 {
            (*png_ptr).num_rows = ((*png_ptr).height + png_pass_yinc[0] as png_uint_32
                - 1
                - png_pass_ystart[0] as png_uint_32)
                / png_pass_yinc[0] as png_uint_32;

            (*png_ptr).usr_width = ((*png_ptr).width + png_pass_inc[0] as png_uint_32
                - 1
                - png_pass_start[0] as png_uint_32)
                / png_pass_inc[0] as png_uint_32;
        } else {
            (*png_ptr).num_rows = (*png_ptr).height;
            (*png_ptr).usr_width = (*png_ptr).width;
        }
    } else {
        (*png_ptr).num_rows = (*png_ptr).height;
        (*png_ptr).usr_width = (*png_ptr).width;
    }
}

/* Internal use only.  Called when finished processing a row of data. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_finish_row(png_ptr: png_structrp) {
    /* Next row */
    (*png_ptr).row_number += 1;

    /* See if we are done */
    if (*png_ptr).row_number < (*png_ptr).num_rows {
        return;
    }

    /* If interlaced, go to next pass */
    if (*png_ptr).interlaced != 0 {
        (*png_ptr).row_number = 0;
        if ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
            (*png_ptr).pass += 1;
        } else {
            /* Loop until we find a non-zero width or height pass */
            loop {
                (*png_ptr).pass += 1;

                if (*png_ptr).pass >= 7 {
                    break;
                }

                (*png_ptr).usr_width = ((*png_ptr).width
                    + png_pass_inc[(*png_ptr).pass as usize] as png_uint_32
                    - 1
                    - png_pass_start[(*png_ptr).pass as usize] as png_uint_32)
                    / png_pass_inc[(*png_ptr).pass as usize] as png_uint_32;

                (*png_ptr).num_rows = ((*png_ptr).height
                    + png_pass_yinc[(*png_ptr).pass as usize] as png_uint_32
                    - 1
                    - png_pass_ystart[(*png_ptr).pass as usize] as png_uint_32)
                    / png_pass_yinc[(*png_ptr).pass as usize] as png_uint_32;

                if ((*png_ptr).transformations & PNG_INTERLACE) != 0 {
                    break;
                }

                if !((*png_ptr).usr_width == 0 || (*png_ptr).num_rows == 0) {
                    break;
                }
            }
        }

        /* Reset the row above the image for the next pass */
        if (*png_ptr).pass < 7 {
            if !(*png_ptr).prev_row.is_null() {
                memset(
                    (*png_ptr).prev_row as *mut c_void,
                    0,
                    PNG_ROWBYTES(
                        (*png_ptr).usr_channels as usize * (*png_ptr).usr_bit_depth as usize,
                        (*png_ptr).width as usize,
                    ) + 1,
                );
            }

            return;
        }
    }

    /* If we get here, we've just written the last row, so we need
    to flush the compressor */
    png_compress_IDAT(png_ptr, core::ptr::null(), 0, Z_FINISH);
}

/* Pick out the correct pixels for the interlace pass.
 * The basic idea here is to go through the row with a source
 * pointer and a destination pointer (sp and dp), and copy the
 * correct pixels for the pass.  As the row gets compacted,
 * sp will always be >= dp, so we should never overwrite anything.
 * See the default: case for the easiest code to understand.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_write_interlace(
    row_info: png_row_infop,
    row: png_bytep,
    pass: c_int,
) {
    /* We don't have to do anything on the last pass (6) */
    if pass < 6 {
        /* Each pixel depth is handled separately */
        match (*row_info).pixel_depth {
            1 => {
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut shift: c_uint;
                let mut d: c_int;
                let mut value: c_int;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                dp = row;
                d = 0;
                shift = 7;

                i = png_pass_start[pass as usize] as png_uint_32;
                while i < row_width {
                    sp = row.add((i >> 3) as usize);
                    value = (*sp as c_int >> (7 - (i & 0x07) as c_int)) & 0x01;
                    d |= value << shift;

                    if shift == 0 {
                        shift = 7;
                        *dp = d as png_byte;
                        dp = dp.add(1);
                        d = 0;
                    } else {
                        shift -= 1;
                    }

                    i += png_pass_inc[pass as usize] as png_uint_32;
                }
                if shift != 7 {
                    *dp = d as png_byte;
                }
            }

            2 => {
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut shift: c_uint;
                let mut d: c_int;
                let mut value: c_int;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                dp = row;
                shift = 6;
                d = 0;

                i = png_pass_start[pass as usize] as png_uint_32;
                while i < row_width {
                    sp = row.add((i >> 2) as usize);
                    value = (*sp as c_int >> ((3 - (i & 0x03) as c_int) << 1)) & 0x03;
                    d |= value << shift;

                    if shift == 0 {
                        shift = 6;
                        *dp = d as png_byte;
                        dp = dp.add(1);
                        d = 0;
                    } else {
                        shift -= 2;
                    }

                    i += png_pass_inc[pass as usize] as png_uint_32;
                }
                if shift != 6 {
                    *dp = d as png_byte;
                }
            }

            4 => {
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut shift: c_uint;
                let mut d: c_int;
                let mut value: c_int;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;

                dp = row;
                shift = 4;
                d = 0;
                i = png_pass_start[pass as usize] as png_uint_32;
                while i < row_width {
                    sp = row.add((i >> 1) as usize);
                    value = (*sp as c_int >> ((1 - (i & 0x01) as c_int) << 2)) & 0x0f;
                    d |= value << shift;

                    if shift == 0 {
                        shift = 4;
                        *dp = d as png_byte;
                        dp = dp.add(1);
                        d = 0;
                    } else {
                        shift -= 4;
                    }

                    i += png_pass_inc[pass as usize] as png_uint_32;
                }
                if shift != 4 {
                    *dp = d as png_byte;
                }
            }

            _ => {
                let mut sp: png_bytep;
                let mut dp: png_bytep;
                let mut i: png_uint_32;
                let row_width: png_uint_32 = (*row_info).width;
                let pixel_bytes: usize;

                /* Start at the beginning */
                dp = row;

                /* Find out how many bytes each pixel takes up */
                pixel_bytes = ((*row_info).pixel_depth >> 3) as usize;

                /* Loop through the row, only looking at the pixels that matter */
                i = png_pass_start[pass as usize] as png_uint_32;
                while i < row_width {
                    /* Find out where the original pixel is */
                    sp = row.add(i as usize * pixel_bytes);

                    /* Move the pixel */
                    if dp != sp {
                        memcpy(dp as *mut c_void, sp as *const c_void, pixel_bytes);
                    }

                    /* Next pixel */
                    dp = dp.add(pixel_bytes);

                    i += png_pass_inc[pass as usize] as png_uint_32;
                }
            }
        }
        /* Set new row width */
        (*row_info).width = ((*row_info).width + png_pass_inc[pass as usize] as png_uint_32
            - 1
            - png_pass_start[pass as usize] as png_uint_32)
            / png_pass_inc[pass as usize] as png_uint_32;

        (*row_info).rowbytes =
            PNG_ROWBYTES((*row_info).pixel_depth as usize, (*row_info).width as usize);
    }
}

pub unsafe fn png_setup_sub_row(
    png_ptr: png_structrp,
    bpp: png_uint_32,
    row_bytes: usize,
    lmins: usize,
) -> usize {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut lp: png_bytep;
    let mut i: usize;
    let mut sum: usize = 0;
    let mut v: c_uint;

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_SUB as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    while i < bpp as usize {
        *dp = *rp;
        v = *dp as c_uint;
        sum += if v < 128 {
            v as usize
        } else {
            (256 - v) as usize
        };
        i += 1;
        rp = rp.add(1);
        dp = dp.add(1);
    }

    lp = (*png_ptr).row_buf.add(1);
    while i < row_bytes {
        *dp = ((*rp as c_int - *lp as c_int) & 0xff) as png_byte;
        v = *dp as c_uint;
        sum += if v < 128 {
            v as usize
        } else {
            (256 - v) as usize
        };

        if sum > lmins {
            /* We are already worse, don't continue. */
            break;
        }
        i += 1;
        rp = rp.add(1);
        lp = lp.add(1);
        dp = dp.add(1);
    }

    sum
}

pub unsafe fn png_setup_sub_row_only(png_ptr: png_structrp, bpp: png_uint_32, row_bytes: usize) {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut lp: png_bytep;
    let mut i: usize;

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_SUB as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    while i < bpp as usize {
        *dp = *rp;
        i += 1;
        rp = rp.add(1);
        dp = dp.add(1);
    }

    lp = (*png_ptr).row_buf.add(1);
    while i < row_bytes {
        *dp = ((*rp as c_int - *lp as c_int) & 0xff) as png_byte;
        i += 1;
        rp = rp.add(1);
        lp = lp.add(1);
        dp = dp.add(1);
    }
}

pub unsafe fn png_setup_up_row(png_ptr: png_structrp, row_bytes: usize, lmins: usize) -> usize {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut i: usize;
    let mut sum: usize = 0;
    let mut v: c_uint;

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_UP as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < row_bytes {
        *dp = ((*rp as c_int - *pp as c_int) & 0xff) as png_byte;
        v = *dp as c_uint;
        sum += if v < 128 {
            v as usize
        } else {
            (256 - v) as usize
        };

        if sum > lmins {
            /* We are already worse, don't continue. */
            break;
        }
        i += 1;
        rp = rp.add(1);
        pp = pp.add(1);
        dp = dp.add(1);
    }

    sum
}

pub unsafe fn png_setup_up_row_only(png_ptr: png_structrp, row_bytes: usize) {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut i: usize;

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_UP as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < row_bytes {
        *dp = ((*rp as c_int - *pp as c_int) & 0xff) as png_byte;
        i += 1;
        rp = rp.add(1);
        pp = pp.add(1);
        dp = dp.add(1);
    }
}

pub unsafe fn png_setup_avg_row(
    png_ptr: png_structrp,
    bpp: png_uint_32,
    row_bytes: usize,
    lmins: usize,
) -> usize {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut lp: png_bytep;
    let mut i: png_uint_32;
    let mut sum: usize = 0;
    let mut v: c_uint;

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_AVG as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < bpp {
        *dp = ((*rp as c_int - (*pp as c_int / 2)) & 0xff) as png_byte;
        v = *dp as c_uint;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);

        sum += if v < 128 {
            v as usize
        } else {
            (256 - v) as usize
        };
        i += 1;
    }

    lp = (*png_ptr).row_buf.add(1);
    while (i as usize) < row_bytes {
        *dp = ((*rp as c_int - ((*pp as c_int + *lp as c_int) / 2)) & 0xff) as png_byte;
        v = *dp as c_uint;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);
        lp = lp.add(1);

        sum += if v < 128 {
            v as usize
        } else {
            (256 - v) as usize
        };

        if sum > lmins {
            /* We are already worse, don't continue. */
            break;
        }
        i += 1;
    }

    sum
}

pub unsafe fn png_setup_avg_row_only(png_ptr: png_structrp, bpp: png_uint_32, row_bytes: usize) {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut lp: png_bytep;
    let mut i: png_uint_32;

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_AVG as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < bpp {
        *dp = ((*rp as c_int - (*pp as c_int / 2)) & 0xff) as png_byte;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);
        i += 1;
    }

    lp = (*png_ptr).row_buf.add(1);
    while (i as usize) < row_bytes {
        *dp = ((*rp as c_int - ((*pp as c_int + *lp as c_int) / 2)) & 0xff) as png_byte;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);
        lp = lp.add(1);
        i += 1;
    }
}

pub unsafe fn png_setup_paeth_row(
    png_ptr: png_structrp,
    bpp: png_uint_32,
    row_bytes: usize,
    lmins: usize,
) -> usize {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut cp: png_bytep;
    let mut lp: png_bytep;
    let mut i: usize;
    let mut sum: usize = 0;
    let mut v: c_uint;

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_PAETH as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < bpp as usize {
        *dp = ((*rp as c_int - *pp as c_int) & 0xff) as png_byte;
        v = *dp as c_uint;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);

        sum += if v < 128 {
            v as usize
        } else {
            (256 - v) as usize
        };
        i += 1;
    }

    lp = (*png_ptr).row_buf.add(1);
    cp = (*png_ptr).prev_row.add(1);
    while i < row_bytes {
        let a: c_int;
        let b: c_int;
        let c: c_int;
        let pa: c_int;
        let pb: c_int;
        let mut pc: c_int;
        let mut p: c_int;

        b = *pp as c_int;
        pp = pp.add(1);
        c = *cp as c_int;
        cp = cp.add(1);
        a = *lp as c_int;
        lp = lp.add(1);

        p = b - c;
        pc = a - c;

        pa = if p < 0 { -p } else { p };
        pb = if pc < 0 { -pc } else { pc };
        pc = if (p + pc) < 0 { -(p + pc) } else { p + pc };

        p = if pa <= pb && pa <= pc {
            a
        } else if pb <= pc {
            b
        } else {
            c
        };

        *dp = ((*rp as c_int - p) & 0xff) as png_byte;
        v = *dp as c_uint;
        dp = dp.add(1);
        rp = rp.add(1);

        sum += if v < 128 {
            v as usize
        } else {
            (256 - v) as usize
        };

        if sum > lmins {
            /* We are already worse, don't continue. */
            break;
        }
        i += 1;
    }

    sum
}

pub unsafe fn png_setup_paeth_row_only(png_ptr: png_structrp, bpp: png_uint_32, row_bytes: usize) {
    let mut rp: png_bytep;
    let mut dp: png_bytep;
    let mut pp: png_bytep;
    let mut cp: png_bytep;
    let mut lp: png_bytep;
    let mut i: usize;

    *(*png_ptr).try_row.add(0) = PNG_FILTER_VALUE_PAETH as png_byte;

    i = 0;
    rp = (*png_ptr).row_buf.add(1);
    dp = (*png_ptr).try_row.add(1);
    pp = (*png_ptr).prev_row.add(1);
    while i < bpp as usize {
        *dp = ((*rp as c_int - *pp as c_int) & 0xff) as png_byte;
        dp = dp.add(1);
        rp = rp.add(1);
        pp = pp.add(1);
        i += 1;
    }

    lp = (*png_ptr).row_buf.add(1);
    cp = (*png_ptr).prev_row.add(1);
    while i < row_bytes {
        let a: c_int;
        let b: c_int;
        let c: c_int;
        let pa: c_int;
        let pb: c_int;
        let mut pc: c_int;
        let mut p: c_int;

        b = *pp as c_int;
        pp = pp.add(1);
        c = *cp as c_int;
        cp = cp.add(1);
        a = *lp as c_int;
        lp = lp.add(1);

        p = b - c;
        pc = a - c;

        pa = if p < 0 { -p } else { p };
        pb = if pc < 0 { -pc } else { pc };
        pc = if (p + pc) < 0 { -(p + pc) } else { p + pc };

        p = if pa <= pb && pa <= pc {
            a
        } else if pb <= pc {
            b
        } else {
            c
        };

        *dp = ((*rp as c_int - p) & 0xff) as png_byte;
        dp = dp.add(1);
        rp = rp.add(1);
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_find_filter(png_ptr: png_structrp, row_info: png_row_infop) {
    let mut filter_to_do: c_uint = (*png_ptr).do_filter as c_uint;
    let row_buf: png_bytep;
    let mut best_row: png_bytep;
    let bpp: png_uint_32;
    let mut mins: usize;
    let row_bytes: usize = (*row_info).rowbytes;

    /* Find out how many bytes offset each pixel is */
    bpp = ((*row_info).pixel_depth as c_uint + 7) as png_uint_32 >> 3;

    row_buf = (*png_ptr).row_buf;
    mins = PNG_SIZE_MAX - 256/* so we can detect potential overflow of the
                             running sum */;

    /* We don't need to test the 'no filter' case if this is the only filter
     * that has been chosen, as it doesn't actually do anything to the data.
     */
    best_row = (*png_ptr).row_buf;

    if PNG_SIZE_MAX / 128 <= row_bytes {
        /* Overflow can occur in the calculation, just select the lowest set
         * filter.
         */
        filter_to_do &= (0u32.wrapping_sub(filter_to_do)) as c_uint;
    } else if (filter_to_do & PNG_FILTER_NONE as c_uint) != 0
        && filter_to_do != PNG_FILTER_NONE as c_uint
    {
        /* Overflow not possible and multiple filters in the list, including the
         * 'none' filter.
         */
        let mut rp: png_bytep;
        let mut sum: usize = 0;
        let mut i: usize;
        let mut v: c_uint;

        {
            i = 0;
            rp = row_buf.add(1);
            while i < row_bytes {
                v = *rp as c_uint;
                sum += if v < 128 {
                    v as usize
                } else {
                    (256 - v) as usize
                };
                i += 1;
                rp = rp.add(1);
            }
        }

        mins = sum;
    }

    /* Sub filter */
    if filter_to_do == PNG_FILTER_SUB as c_uint
    /* It's the only filter so no testing is needed */
    {
        png_setup_sub_row_only(png_ptr, bpp, row_bytes);
        best_row = (*png_ptr).try_row;
    } else if (filter_to_do & PNG_FILTER_SUB as c_uint) != 0 {
        let sum: usize;
        let lmins: usize = mins;

        sum = png_setup_sub_row(png_ptr, bpp, row_bytes, lmins);

        if sum < mins {
            mins = sum;
            best_row = (*png_ptr).try_row;
            if !(*png_ptr).tst_row.is_null() {
                (*png_ptr).try_row = (*png_ptr).tst_row;
                (*png_ptr).tst_row = best_row;
            }
        }
    }

    /* Up filter */
    if filter_to_do == PNG_FILTER_UP as c_uint {
        png_setup_up_row_only(png_ptr, row_bytes);
        best_row = (*png_ptr).try_row;
    } else if (filter_to_do & PNG_FILTER_UP as c_uint) != 0 {
        let sum: usize;
        let lmins: usize = mins;

        sum = png_setup_up_row(png_ptr, row_bytes, lmins);

        if sum < mins {
            mins = sum;
            best_row = (*png_ptr).try_row;
            if !(*png_ptr).tst_row.is_null() {
                (*png_ptr).try_row = (*png_ptr).tst_row;
                (*png_ptr).tst_row = best_row;
            }
        }
    }

    /* Avg filter */
    if filter_to_do == PNG_FILTER_AVG as c_uint {
        png_setup_avg_row_only(png_ptr, bpp, row_bytes);
        best_row = (*png_ptr).try_row;
    } else if (filter_to_do & PNG_FILTER_AVG as c_uint) != 0 {
        let sum: usize;
        let lmins: usize = mins;

        sum = png_setup_avg_row(png_ptr, bpp, row_bytes, lmins);

        if sum < mins {
            mins = sum;
            best_row = (*png_ptr).try_row;
            if !(*png_ptr).tst_row.is_null() {
                (*png_ptr).try_row = (*png_ptr).tst_row;
                (*png_ptr).tst_row = best_row;
            }
        }
    }

    /* Paeth filter */
    if filter_to_do == PNG_FILTER_PAETH as c_uint {
        png_setup_paeth_row_only(png_ptr, bpp, row_bytes);
        best_row = (*png_ptr).try_row;
    } else if (filter_to_do & PNG_FILTER_PAETH as c_uint) != 0 {
        let sum: usize;
        let lmins: usize = mins;

        sum = png_setup_paeth_row(png_ptr, bpp, row_bytes, lmins);

        if sum < mins {
            best_row = (*png_ptr).try_row;
            if !(*png_ptr).tst_row.is_null() {
                (*png_ptr).try_row = (*png_ptr).tst_row;
                (*png_ptr).tst_row = best_row;
            }
        }
    }

    /* Do the actual writing of the filtered row data from the chosen filter. */
    png_write_filtered_row(png_ptr, best_row, (*row_info).rowbytes + 1);
}

/* Do the actual writing of a previously filtered row. */
pub unsafe fn png_write_filtered_row(
    png_ptr: png_structrp,
    filtered_row: png_bytep,
    full_row_length: usize, /*includes filter byte*/
) {
    png_compress_IDAT(png_ptr, filtered_row, full_row_length, Z_NO_FLUSH);

    /* Swap the current and previous rows */
    if !(*png_ptr).prev_row.is_null() {
        let tptr: png_bytep;

        tptr = (*png_ptr).prev_row;
        (*png_ptr).prev_row = (*png_ptr).row_buf;
        (*png_ptr).row_buf = tptr;
    }

    /* Finish row - updates counters and flushes zlib if last row */
    png_write_finish_row(png_ptr);

    (*png_ptr).flush_rows += 1;

    if (*png_ptr).flush_dist > 0 && (*png_ptr).flush_rows >= (*png_ptr).flush_dist {
        png_write_flush(png_ptr);
    }
}
