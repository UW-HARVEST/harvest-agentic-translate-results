/* pngwutil.c lines 1074..1448 */

/* Write an IEND chunk */
/* png_write_IEND */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_IEND(png_ptr: png_structrp) {
    png_write_complete_chunk(png_ptr, png_IEND, core::ptr::null(), 0);
    (*png_ptr).mode |= PNG_HAVE_IEND;
}

/* Write a gAMA chunk */
/* png_write_gAMA_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_gAMA_fixed(png_ptr: png_structrp, file_gamma: png_fixed_point) {
    let mut buf: [png_byte; 4] = [0; 4];

    /* file_gamma is saved in 1/100,000ths */
    png_save_uint_32(buf.as_mut_ptr(), file_gamma as png_uint_32);
    png_write_complete_chunk(png_ptr, png_gAMA, buf.as_ptr(), 4);
}

/* Write a sRGB chunk */
/* png_write_sRGB */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_sRGB(png_ptr: png_structrp, srgb_intent: c_int) {
    let mut buf: [png_byte; 1] = [0; 1];

    if srgb_intent >= PNG_sRGB_INTENT_LAST {
        png_warning(
            png_ptr,
            b"Invalid sRGB rendering intent specified\0".as_ptr() as png_const_charp,
        );
    }

    buf[0] = srgb_intent as png_byte;
    png_write_complete_chunk(png_ptr, png_sRGB, buf.as_ptr(), 1);
}

/* Write an iCCP chunk */
/* png_write_iCCP */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_iCCP(
    png_ptr: png_structrp,
    name: png_const_charp,
    profile: png_const_bytep,
    profile_len: png_uint_32,
) {
    let mut name_len: png_uint_32;
    let mut new_name: [png_byte; 81] = [0; 81]; /* 1 byte for the compression byte */
    let mut comp: compression_state = core::mem::zeroed();
    let temp: png_uint_32;

    /* These are all internal problems: the profile should have been checked
     * before when it was stored.
     */
    if profile == core::ptr::null() {
        png_error(
            png_ptr,
            b"No profile for iCCP chunk\0".as_ptr() as png_const_charp,
        ); /* internal error */
    }

    if profile_len < 132 {
        png_error(
            png_ptr,
            b"ICC profile too short\0".as_ptr() as png_const_charp,
        );
    }

    if PNG_get_uint_32(profile) != profile_len {
        png_error(
            png_ptr,
            b"Incorrect data in iCCP\0".as_ptr() as png_const_charp,
        );
    }

    temp = *profile.add(8) as png_uint_32;
    if temp > 3 && (profile_len & 0x03) != 0 {
        png_error(
            png_ptr,
            b"ICC profile length invalid (not a multiple of 4)\0".as_ptr() as png_const_charp,
        );
    }

    {
        let embedded_profile_len: png_uint_32 = PNG_get_uint_32(profile);

        if profile_len != embedded_profile_len {
            png_error(
                png_ptr,
                b"Profile length does not match profile\0".as_ptr() as png_const_charp,
            );
        }
    }

    name_len = png_check_keyword(png_ptr, name, new_name.as_mut_ptr());

    if name_len == 0 {
        png_error(
            png_ptr,
            b"iCCP: invalid keyword\0".as_ptr() as png_const_charp,
        );
    }

    name_len = name_len.wrapping_add(1);
    new_name[name_len as usize] = PNG_COMPRESSION_TYPE_BASE as png_byte;

    /* Make sure we include the NULL after the name and the compression type */
    name_len = name_len.wrapping_add(1);

    png_text_compress_init(&mut comp, profile, profile_len as png_alloc_size_t);

    /* Allow for keyword terminator and compression byte */
    if png_text_compress(png_ptr, png_iCCP, &mut comp, name_len) != Z_OK {
        png_error(png_ptr, (*png_ptr).zstream.msg);
    }

    png_write_chunk_header(png_ptr, png_iCCP, name_len.wrapping_add(comp.output_len));

    png_write_chunk_data(png_ptr, new_name.as_ptr(), name_len as usize);

    png_write_compressed_data_out(png_ptr, &mut comp);

    png_write_chunk_end(png_ptr);
}

/* Write a sPLT chunk */
/* png_write_sPLT */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_sPLT(png_ptr: png_structrp, spalette: png_const_sPLT_tp) {
    let name_len: png_uint_32;
    let mut new_name: [png_byte; 80] = [0; 80];
    let mut entrybuf: [png_byte; 10] = [0; 10];
    let entry_size: usize = if (*spalette).depth as c_int == 8 { 6 } else { 10 };
    let palette_size: usize = entry_size.wrapping_mul((*spalette).nentries as usize);
    let mut ep: png_sPLT_entryp;

    name_len = png_check_keyword(png_ptr, (*spalette).name, new_name.as_mut_ptr());

    if name_len == 0 {
        png_error(
            png_ptr,
            b"sPLT: invalid keyword\0".as_ptr() as png_const_charp,
        );
    }

    /* Make sure we include the NULL after the name */
    png_write_chunk_header(
        png_ptr,
        png_sPLT,
        (name_len as usize)
            .wrapping_add(2)
            .wrapping_add(palette_size) as png_uint_32,
    );

    png_write_chunk_data(
        png_ptr,
        new_name.as_mut_ptr() as png_const_bytep,
        name_len.wrapping_add(1) as usize,
    );

    png_write_chunk_data(png_ptr, core::ptr::addr_of!((*spalette).depth), 1);

    /* Loop through each palette entry, writing appropriately */
    ep = (*spalette).entries;
    while ep < (*spalette).entries.offset((*spalette).nentries as isize) {
        if (*spalette).depth as c_int == 8 {
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
/* png_write_sBIT */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_sBIT(
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

        if (*sbit).red as c_int == 0
            || (*sbit).red as c_int > maxbits as c_int
            || (*sbit).green as c_int == 0
            || (*sbit).green as c_int > maxbits as c_int
            || (*sbit).blue as c_int == 0
            || (*sbit).blue as c_int > maxbits as c_int
        {
            png_warning(
                png_ptr,
                b"Invalid sBIT depth specified\0".as_ptr() as png_const_charp,
            );
            return;
        }

        buf[0] = (*sbit).red;
        buf[1] = (*sbit).green;
        buf[2] = (*sbit).blue;
        size = 3;
    } else {
        if (*sbit).gray as c_int == 0 || (*sbit).gray as c_int > (*png_ptr).usr_bit_depth as c_int {
            png_warning(
                png_ptr,
                b"Invalid sBIT depth specified\0".as_ptr() as png_const_charp,
            );
            return;
        }

        buf[0] = (*sbit).gray;
        size = 1;
    }

    if (color_type & PNG_COLOR_MASK_ALPHA) != 0 {
        if (*sbit).alpha as c_int == 0 || (*sbit).alpha as c_int > (*png_ptr).usr_bit_depth as c_int
        {
            png_warning(
                png_ptr,
                b"Invalid sBIT depth specified\0".as_ptr() as png_const_charp,
            );
            return;
        }

        buf[size] = (*sbit).alpha;
        size += 1;
    }

    png_write_complete_chunk(png_ptr, png_sBIT, buf.as_ptr(), size);
}

/* Write the cHRM chunk */
/* png_write_cHRM_fixed */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_cHRM_fixed(png_ptr: png_structrp, xy: *const png_xy) {
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
/* png_write_tRNS */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_tRNS(
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
                b"Invalid number of transparent colors specified\0".as_ptr() as png_const_charp,
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
                b"Ignoring attempt to write tRNS chunk out-of-range for bit_depth\0".as_ptr()
                    as png_const_charp,
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
        if (*png_ptr).bit_depth as c_int == 8
            && (buf[0] as c_int | buf[2] as c_int | buf[4] as c_int) != 0
        {
            png_app_warning(
                png_ptr,
                b"Ignoring attempt to write 16-bit tRNS chunk when bit_depth is 8\0".as_ptr()
                    as png_const_charp,
            );
            return;
        }

        png_write_complete_chunk(png_ptr, png_tRNS, buf.as_ptr(), 6);
    } else {
        png_app_warning(
            png_ptr,
            b"Can't write tRNS with an alpha channel\0".as_ptr() as png_const_charp,
        );
    }
}

/* Write the background chunk */
/* png_write_bKGD */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_bKGD(
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
            png_warning(
                png_ptr,
                b"Invalid background palette index\0".as_ptr() as png_const_charp,
            );
            return;
        }

        buf[0] = (*back).index;
        png_write_complete_chunk(png_ptr, png_bKGD, buf.as_ptr(), 1);
    } else if (color_type & PNG_COLOR_MASK_COLOR) != 0 {
        png_save_uint_16(buf.as_mut_ptr(), (*back).red as c_uint);
        png_save_uint_16(buf.as_mut_ptr().add(2), (*back).green as c_uint);
        png_save_uint_16(buf.as_mut_ptr().add(4), (*back).blue as c_uint);
        if (*png_ptr).bit_depth as c_int == 8
            && (buf[0] as c_int | buf[2] as c_int | buf[4] as c_int) != 0
        {
            png_warning(
                png_ptr,
                b"Ignoring attempt to write 16-bit bKGD chunk when bit_depth is 8\0".as_ptr()
                    as png_const_charp,
            );

            return;
        }

        png_write_complete_chunk(png_ptr, png_bKGD, buf.as_ptr(), 6);
    } else {
        if (*back).gray as c_int >= (1 << (*png_ptr).bit_depth as c_int) {
            png_warning(
                png_ptr,
                b"Ignoring attempt to write bKGD chunk out-of-range for bit_depth\0".as_ptr()
                    as png_const_charp,
            );

            return;
        }

        png_save_uint_16(buf.as_mut_ptr(), (*back).gray as c_uint);
        png_write_complete_chunk(png_ptr, png_bKGD, buf.as_ptr(), 2);
    }
}
