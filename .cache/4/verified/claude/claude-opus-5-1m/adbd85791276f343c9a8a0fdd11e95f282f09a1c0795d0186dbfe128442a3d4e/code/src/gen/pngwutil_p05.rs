/* pngwutil.c lines 1449..1928 */

/* Write the cICP data */
/* png_write_cICP */
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

/* png_write_cLLI_fixed */
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

/* png_write_mDCV_fixed */
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
/* png_write_eXIf */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_eXIf(png_ptr: png_structrp, exif: png_bytep, num_exif: c_int) {
    let mut i: c_int;
    let mut buf: [png_byte; 1] = [0; 1];

    png_write_chunk_header(png_ptr, png_eXIf, num_exif as png_uint_32);

    i = 0;
    while i < num_exif {
        buf[0] = *exif.offset(i as isize);
        png_write_chunk_data(png_ptr, buf.as_ptr(), 1);

        i += 1;
    }

    png_write_chunk_end(png_ptr);
}

/* Write the histogram */
/* png_write_hIST */
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
            b"Invalid number of histogram entries specified\0".as_ptr() as png_const_charp,
        );
        return;
    }

    png_write_chunk_header(png_ptr, png_hIST, (num_hist * 2) as png_uint_32);

    i = 0;
    while i < num_hist {
        png_save_uint_16(buf.as_mut_ptr(), *hist.offset(i as isize) as c_uint);
        png_write_chunk_data(png_ptr, buf.as_ptr(), 2);

        i += 1;
    }

    png_write_chunk_end(png_ptr);
}

/* Write a tEXt chunk */
/* png_write_tEXt */
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
        png_error(
            png_ptr,
            b"tEXt: invalid keyword\0".as_ptr() as png_const_charp,
        );
    }

    if text == core::ptr::null() || *text == 0 {
        text_len = 0;
    } else {
        text_len = strlen(text);
    }

    if text_len > (PNG_UINT_31_MAX.wrapping_sub(key_len.wrapping_add(1))) as usize {
        png_error(png_ptr, b"tEXt: text too long\0".as_ptr() as png_const_charp);
    }

    /* Make sure we include the 0 after the key */
    png_write_chunk_header(
        png_ptr,
        png_tEXt,
        (key_len as usize).wrapping_add(text_len).wrapping_add(1) as png_uint_32, /*checked above*/
    );
    /*
     * We leave it to the application to meet PNG-1.0 requirements on the
     * contents of the text.  PNG-1.0 through PNG-1.2 discourage the use of
     * any non-Latin-1 characters except for NEWLINE.  ISO PNG will forbid them.
     * The NUL character is forbidden by PNG-1.0 through PNG-1.2 and ISO PNG.
     */
    png_write_chunk_data(png_ptr, new_key.as_ptr(), key_len.wrapping_add(1) as usize);

    if text_len != 0 {
        png_write_chunk_data(png_ptr, text as png_const_bytep, text_len);
    }

    png_write_chunk_end(png_ptr);
}

/* Write a compressed text chunk */
/* png_write_zTXt */
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
        png_error(
            png_ptr,
            b"zTXt: invalid compression type\0".as_ptr() as png_const_charp,
        );
    }

    key_len = png_check_keyword(png_ptr, key, new_key.as_mut_ptr());

    if key_len == 0 {
        png_error(
            png_ptr,
            b"zTXt: invalid keyword\0".as_ptr() as png_const_charp,
        );
    }

    /* Add the compression method and 1 for the keyword separator. */
    key_len = key_len.wrapping_add(1);
    new_key[key_len as usize] = PNG_COMPRESSION_TYPE_BASE as png_byte;
    key_len = key_len.wrapping_add(1);

    /* Compute the compressed data; do it now for the length */
    png_text_compress_init(
        &mut comp,
        text as png_const_bytep,
        if text == core::ptr::null() {
            0
        } else {
            strlen(text)
        },
    );

    if png_text_compress(png_ptr, png_zTXt, &mut comp, key_len) != Z_OK {
        png_error(png_ptr, (*png_ptr).zstream.msg);
    }

    /* Write start of chunk */
    png_write_chunk_header(png_ptr, png_zTXt, key_len.wrapping_add(comp.output_len));

    /* Write key */
    png_write_chunk_data(png_ptr, new_key.as_ptr(), key_len as usize);

    /* Write the compressed data */
    png_write_compressed_data_out(png_ptr, &mut comp);

    /* Close the chunk */
    png_write_chunk_end(png_ptr);
}

/* Write an iTXt chunk */
/* png_write_iTXt */
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
        png_error(
            png_ptr,
            b"iTXt: invalid keyword\0".as_ptr() as png_const_charp,
        );
    }

    /* Set the compression flag */
    match compression {
        PNG_ITXT_COMPRESSION_NONE | PNG_TEXT_COMPRESSION_NONE => {
            key_len = key_len.wrapping_add(1);
            new_key[key_len as usize] = 0;
            compression = 0; /* no compression */
        }

        PNG_TEXT_COMPRESSION_zTXt | PNG_ITXT_COMPRESSION_zTXt => {
            key_len = key_len.wrapping_add(1);
            new_key[key_len as usize] = 1;
            compression = 1; /* compressed */
        }

        _ => {
            png_error(
                png_ptr,
                b"iTXt: invalid compression\0".as_ptr() as png_const_charp,
            );
        }
    }

    key_len = key_len.wrapping_add(1);
    new_key[key_len as usize] = PNG_COMPRESSION_TYPE_BASE as png_byte;
    key_len = key_len.wrapping_add(1); /* for the keyword separator */

    /* We leave it to the application to meet PNG-1.0 requirements on the
     * contents of the text.  PNG-1.0 through PNG-1.2 discourage the use of
     * any non-Latin-1 characters except for NEWLINE.  ISO PNG, however,
     * specifies that the text is UTF-8 and this really doesn't require any
     * checking.
     *
     * The NUL character is forbidden by PNG-1.0 through PNG-1.2 and ISO PNG.
     *
     * TODO: validate the language tag correctly (see the spec.)
     */
    if lang == core::ptr::null() {
        lang = b"\0".as_ptr() as png_const_charp; /* empty language is valid */
    }
    lang_len = strlen(lang).wrapping_add(1);
    if lang_key == core::ptr::null() {
        lang_key = b"\0".as_ptr() as png_const_charp; /* may be empty */
    }
    lang_key_len = strlen(lang_key).wrapping_add(1);
    if text == core::ptr::null() {
        text = b"\0".as_ptr() as png_const_charp; /* may be empty */
    }

    prefix_len = key_len;
    if lang_len > PNG_UINT_31_MAX.wrapping_sub(prefix_len) as usize {
        prefix_len = PNG_UINT_31_MAX;
    } else {
        prefix_len = (prefix_len as usize).wrapping_add(lang_len) as png_uint_32;
    }

    if lang_key_len > PNG_UINT_31_MAX.wrapping_sub(prefix_len) as usize {
        prefix_len = PNG_UINT_31_MAX;
    } else {
        prefix_len = (prefix_len as usize).wrapping_add(lang_key_len) as png_uint_32;
    }

    png_text_compress_init(&mut comp, text as png_const_bytep, strlen(text));

    if compression != 0 {
        if png_text_compress(png_ptr, png_iTXt, &mut comp, prefix_len) != Z_OK {
            png_error(png_ptr, (*png_ptr).zstream.msg);
        }
    } else {
        if comp.input_len > PNG_UINT_31_MAX.wrapping_sub(prefix_len) as usize {
            png_error(
                png_ptr,
                b"iTXt: uncompressed text too long\0".as_ptr() as png_const_charp,
            );
        }

        /* So the string will fit in a chunk: */
        comp.output_len = comp.input_len as png_uint_32; /*SAFE*/
    }

    png_write_chunk_header(
        png_ptr,
        png_iTXt,
        comp.output_len.wrapping_add(prefix_len),
    );

    png_write_chunk_data(png_ptr, new_key.as_ptr(), key_len as usize);

    png_write_chunk_data(png_ptr, lang as png_const_bytep, lang_len);

    png_write_chunk_data(png_ptr, lang_key as png_const_bytep, lang_key_len);

    if compression != 0 {
        png_write_compressed_data_out(png_ptr, &mut comp);
    } else {
        png_write_chunk_data(
            png_ptr,
            text as png_const_bytep,
            comp.output_len as usize,
        );
    }

    png_write_chunk_end(png_ptr);
}

/* Write the oFFs chunk */
/* png_write_oFFs */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_oFFs(
    png_ptr: png_structrp,
    x_offset: png_int_32,
    y_offset: png_int_32,
    unit_type: c_int,
) {
    let mut buf: [png_byte; 9] = [0; 9];

    if unit_type >= PNG_OFFSET_LAST {
        png_warning(
            png_ptr,
            b"Unrecognized unit type for oFFs chunk\0".as_ptr() as png_const_charp,
        );
    }

    png_save_int_32(buf.as_mut_ptr(), x_offset);
    png_save_int_32(buf.as_mut_ptr().add(4), y_offset);
    buf[8] = unit_type as png_byte;

    png_write_complete_chunk(png_ptr, png_oFFs, buf.as_ptr(), 9);
}

/* Write the pCAL chunk (described in the PNG extensions document) */
/* png_write_pCAL */
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
            b"Unrecognized equation type for pCAL chunk\0".as_ptr() as png_const_charp,
        );
    }

    purpose_len = png_check_keyword(png_ptr, purpose, new_purpose.as_mut_ptr());

    if purpose_len == 0 {
        png_error(
            png_ptr,
            b"pCAL: invalid keyword\0".as_ptr() as png_const_charp,
        );
    }

    purpose_len = purpose_len.wrapping_add(1); /* terminator */

    units_len = strlen(units).wrapping_add(if nparams == 0 { 0 } else { 1 });
    total_len = (purpose_len as usize)
        .wrapping_add(units_len)
        .wrapping_add(10);

    params_len = png_malloc(
        png_ptr,
        (nparams as png_alloc_size_t)
            .wrapping_mul(core::mem::size_of::<usize>() as png_alloc_size_t)
            as png_alloc_size_t,
    ) as *mut usize;

    /* Find the length of each parameter, making sure we don't count the
     * null terminator for the last parameter.
     */
    i = 0;
    while i < nparams {
        *params_len.offset(i as isize) = strlen(*params.offset(i as isize))
            .wrapping_add(if i == nparams - 1 { 0 } else { 1 });
        total_len = total_len.wrapping_add(*params_len.offset(i as isize));

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
            *params.offset(i as isize) as png_const_bytep,
            *params_len.offset(i as isize),
        );

        i += 1;
    }

    png_free(png_ptr, params_len as png_voidp);
    png_write_chunk_end(png_ptr);
}

/* Write the sCAL chunk */
/* png_write_sCAL_s */
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
    total_len = wlen.wrapping_add(hlen).wrapping_add(2);

    if total_len > 64 {
        png_warning(
            png_ptr,
            b"Can't write sCAL (buffer too small)\0".as_ptr() as png_const_charp,
        );
        return;
    }

    buf[0] = unit as png_byte;
    memcpy(
        buf.as_mut_ptr().add(1) as *mut c_void,
        width as *const c_void,
        wlen.wrapping_add(1),
    ); /* Append the '\0' here */
    memcpy(
        buf.as_mut_ptr().add(wlen.wrapping_add(2)) as *mut c_void,
        height as *const c_void,
        hlen,
    ); /* Do NOT append the '\0' here */

    png_write_complete_chunk(png_ptr, png_sCAL, buf.as_ptr(), total_len);
}

/* Write the pHYs chunk */
/* png_write_pHYs */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_pHYs(
    png_ptr: png_structrp,
    x_pixels_per_unit: png_uint_32,
    y_pixels_per_unit: png_uint_32,
    unit_type: c_int,
) {
    let mut buf: [png_byte; 9] = [0; 9];

    if unit_type >= PNG_RESOLUTION_LAST {
        png_warning(
            png_ptr,
            b"Unrecognized unit type for pHYs chunk\0".as_ptr() as png_const_charp,
        );
    }

    png_save_uint_32(buf.as_mut_ptr(), x_pixels_per_unit);
    png_save_uint_32(buf.as_mut_ptr().add(4), y_pixels_per_unit);
    buf[8] = unit_type as png_byte;

    png_write_complete_chunk(png_ptr, png_pHYs, buf.as_ptr(), 9);
}

/* Write the tIME chunk.  Use either png_convert_from_struct_tm()
 * or png_convert_from_time_t(), or fill in the structure yourself.
 */
/* png_write_tIME */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_tIME(png_ptr: png_structrp, mod_time: png_const_timep) {
    let mut buf: [png_byte; 7] = [0; 7];

    if (*mod_time).month as c_int > 12
        || ((*mod_time).month as c_int) < 1
        || (*mod_time).day as c_int > 31
        || ((*mod_time).day as c_int) < 1
        || (*mod_time).hour as c_int > 23
        || (*mod_time).second as c_int > 60
    {
        png_warning(
            png_ptr,
            b"Invalid time specified for tIME chunk\0".as_ptr() as png_const_charp,
        );
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
