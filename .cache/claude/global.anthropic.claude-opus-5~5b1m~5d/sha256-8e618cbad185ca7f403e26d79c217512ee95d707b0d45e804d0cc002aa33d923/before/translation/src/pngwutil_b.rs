//! pngwutil.c lines 1074-1928: ancillary chunk writing (IEND, gAMA, sRGB,
//! iCCP, sPLT, sBIT, cHRM, tRNS, bKGD, cICP, cLLI, mDCV, eXIf, hIST, tEXt,
//! zTXt, iTXt, oFFs, pCAL, sCAL, pHYs, tIME).
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

/* Write an IEND chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_IEND(png_ptr: png_structrp) {
    png_write_complete_chunk(png_ptr, png_IEND, core::ptr::null(), 0);
    (*png_ptr).mode |= PNG_HAVE_IEND;
}

/* Write a gAMA chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_gAMA_fixed(
    png_ptr: png_structrp,
    file_gamma: png_fixed_point,
) {
    let mut buf: [png_byte; 4] = [0; 4];

    /* file_gamma is saved in 1/100,000ths */
    png_save_uint_32(buf.as_mut_ptr(), file_gamma as png_uint_32);
    png_write_complete_chunk(png_ptr, png_gAMA, buf.as_ptr(), 4);
}

/* Write a sRGB chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_sRGB(png_ptr: png_structrp, srgb_intent: c_int) {
    let mut buf: [png_byte; 1] = [0; 1];

    if srgb_intent >= PNG_sRGB_INTENT_LAST {
        png_warning(png_ptr, c"Invalid sRGB rendering intent specified".as_ptr());
    }

    buf[0] = srgb_intent as png_byte;
    png_write_complete_chunk(png_ptr, png_sRGB, buf.as_ptr(), 1);
}

/* Write an iCCP chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_iCCP(
    png_ptr: png_structrp,
    name: png_const_charp,
    profile: png_const_bytep,
    profile_len: png_uint_32,
) {
    let mut name_len: png_uint_32;
    let mut new_name: [png_byte; 81] = [0; 81]; /* 1 byte for the compression byte */
    let mut comp: compression_state = compression_state::default();
    let temp: png_uint_32;

    /* These are all internal problems: the profile should have been checked
     * before when it was stored.
     */
    if profile.is_null() {
        png_error(png_ptr, c"No profile for iCCP chunk".as_ptr()); /* internal error */
    }

    if profile_len < 132 {
        png_error(png_ptr, c"ICC profile too short".as_ptr());
    }

    if png_get_uint_32(profile) != profile_len {
        png_error(png_ptr, c"Incorrect data in iCCP".as_ptr());
    }

    temp = (*profile.add(8)) as png_uint_32;
    if temp > 3 && (profile_len & 0x03) != 0 {
        png_error(
            png_ptr,
            c"ICC profile length invalid (not a multiple of 4)".as_ptr(),
        );
    }

    {
        let embedded_profile_len: png_uint_32 = png_get_uint_32(profile);

        if profile_len != embedded_profile_len {
            png_error(png_ptr, c"Profile length does not match profile".as_ptr());
        }
    }

    name_len = png_check_keyword(png_ptr, name, new_name.as_mut_ptr());

    if name_len == 0 {
        png_error(png_ptr, c"iCCP: invalid keyword".as_ptr());
    }

    name_len += 1;
    new_name[name_len as usize] = PNG_COMPRESSION_TYPE_BASE as png_byte;

    /* Make sure we include the NULL after the name and the compression type */
    name_len += 1;

    png_text_compress_init(&mut comp, profile, profile_len as png_alloc_size_t);

    /* Allow for keyword terminator and compression byte */
    if png_text_compress(png_ptr, png_iCCP, &mut comp, name_len) != Z_OK {
        png_error(png_ptr, (*png_ptr).zstream.msg);
    }

    png_write_chunk_header(png_ptr, png_iCCP, name_len + comp.output_len);

    png_write_chunk_data(png_ptr, new_name.as_ptr(), name_len as usize);

    png_write_compressed_data_out(png_ptr, &mut comp);

    png_write_chunk_end(png_ptr);
}

/* Write a sPLT chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_sPLT(png_ptr: png_structrp, spalette: png_const_sPLT_tp) {
    let name_len: png_uint_32;
    let mut new_name: [png_byte; 80] = [0; 80];
    let mut entrybuf: [png_byte; 10] = [0; 10];
    let entry_size: usize = if (*spalette).depth == 8 { 6 } else { 10 };
    let palette_size: usize = entry_size * ((*spalette).nentries as usize);
    let mut ep: png_sPLT_entryp;

    name_len = png_check_keyword(png_ptr, (*spalette).name, new_name.as_mut_ptr());

    if name_len == 0 {
        png_error(png_ptr, c"sPLT: invalid keyword".as_ptr());
    }

    /* Make sure we include the NULL after the name */
    png_write_chunk_header(
        png_ptr,
        png_sPLT,
        (name_len as usize + 2 + palette_size) as png_uint_32,
    );

    png_write_chunk_data(
        png_ptr,
        new_name.as_ptr() as png_const_bytep,
        (name_len + 1) as usize,
    );

    png_write_chunk_data(png_ptr, core::ptr::addr_of!((*spalette).depth), 1);

    /* Loop through each palette entry, writing appropriately */
    ep = (*spalette).entries;
    while ep < (*spalette).entries.offset((*spalette).nentries as isize) {
        if (*spalette).depth == 8 {
            entrybuf[0] = (*ep).red as png_byte;
            entrybuf[1] = (*ep).green as png_byte;
            entrybuf[2] = (*ep).blue as png_byte;
            entrybuf[3] = (*ep).alpha as png_byte;
            png_save_uint_16(entrybuf.as_mut_ptr().add(4), (*ep).frequency as c_uint);
        } else {
            png_save_uint_16(entrybuf.as_mut_ptr().add(0), (*ep).red as c_uint);
            png_save_uint_16(entrybuf.as_mut_ptr().add(2), (*ep).green as c_uint);
            png_save_uint_16(entrybuf.as_mut_ptr().add(4), (*ep).blue as c_uint);
            png_save_uint_16(entrybuf.as_mut_ptr().add(6), (*ep).alpha as c_uint);
            png_save_uint_16(entrybuf.as_mut_ptr().add(8), (*ep).frequency as c_uint);
        }

        png_write_chunk_data(png_ptr, entrybuf.as_ptr(), entry_size);

        ep = ep.add(1);
    }

    png_write_chunk_end(png_ptr);
}

/* Write the sBIT chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_sBIT(
    png_ptr: png_structrp,
    sbit: png_const_color_8p,
    color_type: c_int,
) {
    let mut buf: [png_byte; 4] = [0; 4];
    let mut size: usize;

    /* Make sure we don't depend upon the order of PNG_COLOR_8 */
    if (color_type & PNG_COLOR_MASK_COLOR) != 0 {
        let maxbits: png_byte;

        maxbits = (if color_type == PNG_COLOR_TYPE_PALETTE {
            8
        } else {
            (*png_ptr).usr_bit_depth as c_int
        }) as png_byte;

        if (*sbit).red == 0
            || (*sbit).red > maxbits
            || (*sbit).green == 0
            || (*sbit).green > maxbits
            || (*sbit).blue == 0
            || (*sbit).blue > maxbits
        {
            png_warning(png_ptr, c"Invalid sBIT depth specified".as_ptr());
            return;
        }

        buf[0] = (*sbit).red;
        buf[1] = (*sbit).green;
        buf[2] = (*sbit).blue;
        size = 3;
    } else {
        if (*sbit).gray == 0 || (*sbit).gray > (*png_ptr).usr_bit_depth {
            png_warning(png_ptr, c"Invalid sBIT depth specified".as_ptr());
            return;
        }

        buf[0] = (*sbit).gray;
        size = 1;
    }

    if (color_type & PNG_COLOR_MASK_ALPHA) != 0 {
        if (*sbit).alpha == 0 || (*sbit).alpha > (*png_ptr).usr_bit_depth {
            png_warning(png_ptr, c"Invalid sBIT depth specified".as_ptr());
            return;
        }

        buf[size] = (*sbit).alpha;
        size += 1;
    }

    png_write_complete_chunk(png_ptr, png_sBIT, buf.as_ptr(), size);
}

/* Write the cHRM chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_cHRM_fixed(png_ptr: png_structrp, xy: *const png_xy) {
    let mut buf: [png_byte; 32] = [0; 32];

    /* Each value is saved in 1/100,000ths */
    png_save_int_32(buf.as_mut_ptr(), (*xy).whitex);
    png_save_int_32(buf.as_mut_ptr().add(4), (*xy).whitey);

    png_save_int_32(buf.as_mut_ptr().add(8), (*xy).redx);
    png_save_int_32(buf.as_mut_ptr().add(12), (*xy).redy);

    png_save_int_32(buf.as_mut_ptr().add(16), (*xy).greenx);
    png_save_int_32(buf.as_mut_ptr().add(20), (*xy).greeny);

    png_save_int_32(buf.as_mut_ptr().add(24), (*xy).bluex);
    png_save_int_32(buf.as_mut_ptr().add(28), (*xy).bluey);

    png_write_complete_chunk(png_ptr, png_cHRM, buf.as_ptr(), 32);
}

/* Write the tRNS chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_tRNS(
    png_ptr: png_structrp,
    trans_alpha: png_const_bytep,
    tran: png_const_color_16p,
    num_trans: c_int,
    color_type: c_int,
) {
    let mut buf: [png_byte; 6] = [0; 6];

    if color_type == PNG_COLOR_TYPE_PALETTE {
        if num_trans <= 0 || num_trans > (*png_ptr).num_palette as c_int {
            png_app_warning(
                png_ptr,
                c"Invalid number of transparent colors specified".as_ptr(),
            );
            return;
        }

        /* Write the chunk out as it is */
        png_write_complete_chunk(png_ptr, png_tRNS, trans_alpha, num_trans as usize);
    } else if color_type == PNG_COLOR_TYPE_GRAY {
        /* One 16-bit value */
        if (*tran).gray as c_int >= (1 << (*png_ptr).bit_depth as c_int) {
            png_app_warning(
                png_ptr,
                c"Ignoring attempt to write tRNS chunk out-of-range for bit_depth".as_ptr(),
            );

            return;
        }

        png_save_uint_16(buf.as_mut_ptr(), (*tran).gray as c_uint);
        png_write_complete_chunk(png_ptr, png_tRNS, buf.as_ptr(), 2);
    } else if color_type == PNG_COLOR_TYPE_RGB {
        /* Three 16-bit values */
        png_save_uint_16(buf.as_mut_ptr(), (*tran).red as c_uint);
        png_save_uint_16(buf.as_mut_ptr().add(2), (*tran).green as c_uint);
        png_save_uint_16(buf.as_mut_ptr().add(4), (*tran).blue as c_uint);
        if (*png_ptr).bit_depth == 8 && (buf[0] | buf[2] | buf[4]) != 0 {
            png_app_warning(
                png_ptr,
                c"Ignoring attempt to write 16-bit tRNS chunk when bit_depth is 8".as_ptr(),
            );
            return;
        }

        png_write_complete_chunk(png_ptr, png_tRNS, buf.as_ptr(), 6);
    } else {
        png_app_warning(png_ptr, c"Can't write tRNS with an alpha channel".as_ptr());
    }
}

/* Write the background chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_bKGD(
    png_ptr: png_structrp,
    back: png_const_color_16p,
    color_type: c_int,
) {
    let mut buf: [png_byte; 6] = [0; 6];

    if color_type == PNG_COLOR_TYPE_PALETTE {
        if ((*png_ptr).num_palette != 0
            || ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_EMPTY_PLTE) == 0)
            && (*back).index as c_int >= (*png_ptr).num_palette as c_int
        {
            png_warning(png_ptr, c"Invalid background palette index".as_ptr());
            return;
        }

        buf[0] = (*back).index;
        png_write_complete_chunk(png_ptr, png_bKGD, buf.as_ptr(), 1);
    } else if (color_type & PNG_COLOR_MASK_COLOR) != 0 {
        png_save_uint_16(buf.as_mut_ptr(), (*back).red as c_uint);
        png_save_uint_16(buf.as_mut_ptr().add(2), (*back).green as c_uint);
        png_save_uint_16(buf.as_mut_ptr().add(4), (*back).blue as c_uint);
        if (*png_ptr).bit_depth == 8 && (buf[0] | buf[2] | buf[4]) != 0 {
            png_warning(
                png_ptr,
                c"Ignoring attempt to write 16-bit bKGD chunk when bit_depth is 8".as_ptr(),
            );

            return;
        }

        png_write_complete_chunk(png_ptr, png_bKGD, buf.as_ptr(), 6);
    } else {
        if (*back).gray as c_int >= (1 << (*png_ptr).bit_depth as c_int) {
            png_warning(
                png_ptr,
                c"Ignoring attempt to write bKGD chunk out-of-range for bit_depth".as_ptr(),
            );

            return;
        }

        png_save_uint_16(buf.as_mut_ptr(), (*back).gray as c_uint);
        png_write_complete_chunk(png_ptr, png_bKGD, buf.as_ptr(), 2);
    }
}

/* Write the cICP data */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_cICP(
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
pub unsafe extern "C-unwind" fn png_write_cLLI_fixed(
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
pub unsafe extern "C-unwind" fn png_write_mDCV_fixed(
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
pub unsafe extern "C-unwind" fn png_write_eXIf(
    png_ptr: png_structrp,
    exif: png_bytep,
    num_exif: c_int,
) {
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
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_hIST(
    png_ptr: png_structrp,
    hist: png_const_uint_16p,
    num_hist: c_int,
) {
    let mut i: c_int;
    let mut buf: [png_byte; 3] = [0; 3];

    if num_hist > (*png_ptr).num_palette as c_int {
        png_warning(
            png_ptr,
            c"Invalid number of histogram entries specified".as_ptr(),
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
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_tEXt(
    png_ptr: png_structrp,
    key: png_const_charp,
    text: png_const_charp,
    text_len_in: usize,
) {
    let key_len: png_uint_32;
    let mut new_key: [png_byte; 80] = [0; 80];
    let mut text_len: usize = text_len_in;

    key_len = png_check_keyword(png_ptr, key, new_key.as_mut_ptr());

    if key_len == 0 {
        png_error(png_ptr, c"tEXt: invalid keyword".as_ptr());
    }

    if text.is_null() || *text == 0 {
        text_len = 0;
    } else {
        text_len = strlen(text);
    }

    if text_len > (PNG_UINT_31_MAX - (key_len + 1)) as usize {
        png_error(png_ptr, c"tEXt: text too long".as_ptr());
    }

    /* Make sure we include the 0 after the key */
    png_write_chunk_header(
        png_ptr,
        png_tEXt,
        (key_len as usize + text_len + 1) as png_uint_32, /*checked above*/
    );
    /*
     * We leave it to the application to meet PNG-1.0 requirements on the
     * contents of the text.  PNG-1.0 through PNG-1.2 discourage the use of
     * any non-Latin-1 characters except for NEWLINE.  ISO PNG will forbid them.
     * The NUL character is forbidden by PNG-1.0 through PNG-1.2 and ISO PNG.
     */
    png_write_chunk_data(png_ptr, new_key.as_ptr(), (key_len + 1) as usize);

    if text_len != 0 {
        png_write_chunk_data(png_ptr, text as png_const_bytep, text_len);
    }

    png_write_chunk_end(png_ptr);
}

/* Write a compressed text chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_zTXt(
    png_ptr: png_structrp,
    key: png_const_charp,
    text: png_const_charp,
    compression: c_int,
) {
    let mut key_len: png_uint_32;
    let mut new_key: [png_byte; 81] = [0; 81];
    let mut comp: compression_state = compression_state::default();

    if compression == PNG_TEXT_COMPRESSION_NONE {
        png_write_tEXt(png_ptr, key, text, 0);
        return;
    }

    if compression != PNG_TEXT_COMPRESSION_zTXt {
        png_error(png_ptr, c"zTXt: invalid compression type".as_ptr());
    }

    key_len = png_check_keyword(png_ptr, key, new_key.as_mut_ptr());

    if key_len == 0 {
        png_error(png_ptr, c"zTXt: invalid keyword".as_ptr());
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
pub unsafe extern "C-unwind" fn png_write_iTXt(
    png_ptr: png_structrp,
    compression_in: c_int,
    key: png_const_charp,
    lang_in: png_const_charp,
    lang_key_in: png_const_charp,
    text_in: png_const_charp,
) {
    let mut key_len: png_uint_32;
    let mut prefix_len: png_uint_32;
    let lang_len: usize;
    let lang_key_len: usize;
    let mut new_key: [png_byte; 82] = [0; 82];
    let mut comp: compression_state = compression_state::default();
    let mut compression: c_int = compression_in;
    let mut lang: png_const_charp = lang_in;
    let mut lang_key: png_const_charp = lang_key_in;
    let mut text: png_const_charp = text_in;

    key_len = png_check_keyword(png_ptr, key, new_key.as_mut_ptr());

    if key_len == 0 {
        png_error(png_ptr, c"iTXt: invalid keyword".as_ptr());
    }

    /* Set the compression flag */
    match compression {
        PNG_ITXT_COMPRESSION_NONE | PNG_TEXT_COMPRESSION_NONE => {
            key_len += 1;
            new_key[key_len as usize] = 0;
            compression = 0; /* no compression */
        }

        PNG_TEXT_COMPRESSION_zTXt | PNG_ITXT_COMPRESSION_zTXt => {
            key_len += 1;
            new_key[key_len as usize] = 1;
            compression = 1; /* compressed */
        }

        _ => {
            png_error(png_ptr, c"iTXt: invalid compression".as_ptr());
        }
    }

    key_len += 1;
    new_key[key_len as usize] = PNG_COMPRESSION_TYPE_BASE as png_byte;
    key_len += 1; /* for the keyword separator */

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
    if lang.is_null() {
        lang = c"".as_ptr(); /* empty language is valid */
    }
    lang_len = strlen(lang) + 1;
    if lang_key.is_null() {
        lang_key = c"".as_ptr(); /* may be empty */
    }
    lang_key_len = strlen(lang_key) + 1;
    if text.is_null() {
        text = c"".as_ptr(); /* may be empty */
    }

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
            png_error(png_ptr, c"iTXt: uncompressed text too long".as_ptr());
        }

        /* So the string will fit in a chunk: */
        comp.output_len = comp.input_len as png_uint_32; /*SAFE*/
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
pub unsafe extern "C-unwind" fn png_write_oFFs(
    png_ptr: png_structrp,
    x_offset: png_int_32,
    y_offset: png_int_32,
    unit_type: c_int,
) {
    let mut buf: [png_byte; 9] = [0; 9];

    if unit_type >= PNG_OFFSET_LAST {
        png_warning(png_ptr, c"Unrecognized unit type for oFFs chunk".as_ptr());
    }

    png_save_int_32(buf.as_mut_ptr(), x_offset);
    png_save_int_32(buf.as_mut_ptr().add(4), y_offset);
    buf[8] = unit_type as png_byte;

    png_write_complete_chunk(png_ptr, png_oFFs, buf.as_ptr(), 9);
}

/* Write the pCAL chunk (described in the PNG extensions document) */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_pCAL(
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
            c"Unrecognized equation type for pCAL chunk".as_ptr(),
        );
    }

    purpose_len = png_check_keyword(png_ptr, purpose, new_purpose.as_mut_ptr());

    if purpose_len == 0 {
        png_error(png_ptr, c"pCAL: invalid keyword".as_ptr());
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
        *params_len.offset(i as isize) = strlen(*params.offset(i as isize))
            + (if i == nparams - 1 { 0 } else { 1 });
        total_len += *params_len.offset(i as isize);
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
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_sCAL_s(
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
        png_warning(png_ptr, c"Can't write sCAL (buffer too small)".as_ptr());
        return;
    }

    buf[0] = unit as png_byte;
    memcpy(
        buf.as_mut_ptr().add(1) as *mut u8,
        width as *const u8,
        wlen + 1,
    ); /* Append the '\0' here */
    memcpy(
        buf.as_mut_ptr().add(wlen + 2) as *mut u8,
        height as *const u8,
        hlen,
    ); /* Do NOT append the '\0' here */

    png_write_complete_chunk(png_ptr, png_sCAL, buf.as_ptr(), total_len);
}

/* Write the pHYs chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_pHYs(
    png_ptr: png_structrp,
    x_pixels_per_unit: png_uint_32,
    y_pixels_per_unit: png_uint_32,
    unit_type: c_int,
) {
    let mut buf: [png_byte; 9] = [0; 9];

    if unit_type >= PNG_RESOLUTION_LAST {
        png_warning(png_ptr, c"Unrecognized unit type for pHYs chunk".as_ptr());
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
pub unsafe extern "C-unwind" fn png_write_tIME(png_ptr: png_structrp, mod_time: png_const_timep) {
    let mut buf: [png_byte; 7] = [0; 7];

    if (*mod_time).month as c_int > 12
        || ((*mod_time).month as c_int) < 1
        || (*mod_time).day as c_int > 31
        || ((*mod_time).day as c_int) < 1
        || (*mod_time).hour as c_int > 23
        || (*mod_time).second as c_int > 60
    {
        png_warning(png_ptr, c"Invalid time specified for tIME chunk".as_ptr());
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
