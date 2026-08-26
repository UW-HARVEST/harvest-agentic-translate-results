// pngwutil.c - utilities to write a PNG file
//
// This file is part of the translation of libpng.  It contains the functions
// png_write_IHDR() .. png_write_sBIT().

use crate::*;

/* Write the IHDR chunk, and update the png_struct with the necessary
 * information.  Note that the rest of this code depends upon this
 * information being correct.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_IHDR(
    png_ptr: png_structrp,
    width: png_uint_32,
    height: png_uint_32,
    bit_depth: c_int,
    color_type: c_int,
    mut compression_method: c_int,
    mut filter_method: c_int,
    mut interlace_method: c_int,
) {
    let mut buf: [png_byte; 13] = [0; 13]; /* Buffer to store the IHDR info */
    let mut is_invalid_depth: c_int;

    /* Check that we have valid input data from the application info */
    match color_type {
        PNG_COLOR_TYPE_GRAY => {
            match bit_depth {
                1 | 2 | 4 | 8 | 16 => {
                    (*png_ptr).channels = 1;
                }

                _ => {
                    png_error(png_ptr, cstr!("Invalid bit depth for grayscale image"));
                }
            }
        }

        PNG_COLOR_TYPE_RGB => {
            is_invalid_depth = (bit_depth != 8) as c_int;

            is_invalid_depth = (is_invalid_depth != 0 && bit_depth != 16) as c_int;

            if is_invalid_depth != 0 {
                png_error(png_ptr, cstr!("Invalid bit depth for RGB image"));
            }

            (*png_ptr).channels = 3;
        }

        PNG_COLOR_TYPE_PALETTE => {
            match bit_depth {
                1 | 2 | 4 | 8 => {
                    (*png_ptr).channels = 1;
                }

                _ => {
                    png_error(png_ptr, cstr!("Invalid bit depth for paletted image"));
                }
            }
        }

        PNG_COLOR_TYPE_GRAY_ALPHA => {
            is_invalid_depth = (bit_depth != 8) as c_int;

            is_invalid_depth = (is_invalid_depth != 0 && bit_depth != 16) as c_int;

            if is_invalid_depth != 0 {
                png_error(png_ptr, cstr!("Invalid bit depth for grayscale+alpha image"));
            }

            (*png_ptr).channels = 2;
        }

        PNG_COLOR_TYPE_RGB_ALPHA => {
            is_invalid_depth = (bit_depth != 8) as c_int;

            is_invalid_depth = (is_invalid_depth != 0 && bit_depth != 16) as c_int;

            if is_invalid_depth != 0 {
                png_error(png_ptr, cstr!("Invalid bit depth for RGBA image"));
            }

            (*png_ptr).channels = 4;
        }

        _ => {
            png_error(png_ptr, cstr!("Invalid image color type specified"));
        }
    }

    if compression_method != PNG_COMPRESSION_TYPE_BASE {
        png_warning(png_ptr, cstr!("Invalid compression type specified"));
        compression_method = PNG_COMPRESSION_TYPE_BASE;
    }

    /* Write filter_method 64 (intrapixel differencing) only if
     * 1. Libpng was compiled with PNG_MNG_FEATURES_SUPPORTED and
     * 2. Libpng did not write a PNG signature (this filter_method is only
     *    used in PNG datastreams that are embedded in MNG datastreams) and
     * 3. The application called png_permit_mng_features with a mask that
     *    included PNG_FLAG_MNG_FILTER_64 and
     * 4. The filter_method is 64 and
     * 5. The color_type is RGB or RGBA
     */
    if !(((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
        && ((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) == 0
        && (color_type == PNG_COLOR_TYPE_RGB || color_type == PNG_COLOR_TYPE_RGB_ALPHA)
        && (filter_method == PNG_INTRAPIXEL_DIFFERENCING))
        && filter_method != PNG_FILTER_TYPE_BASE
    {
        png_warning(png_ptr, cstr!("Invalid filter type specified"));
        filter_method = PNG_FILTER_TYPE_BASE;
    }

    if interlace_method != PNG_INTERLACE_NONE && interlace_method != PNG_INTERLACE_ADAM7 {
        png_warning(png_ptr, cstr!("Invalid interlace type specified"));
        interlace_method = PNG_INTERLACE_ADAM7;
    }

    /* Save the relevant information */
    (*png_ptr).bit_depth = bit_depth as png_byte;
    (*png_ptr).color_type = color_type as png_byte;
    (*png_ptr).interlaced = interlace_method as png_byte;

    (*png_ptr).filter_type = filter_method as png_byte;

    (*png_ptr).compression_type = compression_method as png_byte;
    (*png_ptr).width = width;
    (*png_ptr).height = height;

    (*png_ptr).pixel_depth = (bit_depth * (*png_ptr).channels as c_int) as png_byte;
    (*png_ptr).rowbytes = PNG_ROWBYTES((*png_ptr).pixel_depth as usize, width as usize);
    /* Set the usr info, so any transformations can modify it */
    (*png_ptr).usr_width = (*png_ptr).width;
    (*png_ptr).usr_bit_depth = (*png_ptr).bit_depth;
    (*png_ptr).usr_channels = (*png_ptr).channels;

    /* Pack the header information into the buffer */
    png_save_uint_32(buf.as_mut_ptr(), width);
    png_save_uint_32(buf.as_mut_ptr().add(4), height);
    buf[8] = bit_depth as png_byte;
    buf[9] = color_type as png_byte;
    buf[10] = compression_method as png_byte;
    buf[11] = filter_method as png_byte;
    buf[12] = interlace_method as png_byte;

    /* Write the chunk */
    png_write_complete_chunk(png_ptr, png_IHDR, buf.as_ptr(), 13);

    if ((*png_ptr).do_filter as c_int) == PNG_NO_FILTERS {
        if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
            || ((*png_ptr).bit_depth as c_int) < 8
        {
            (*png_ptr).do_filter = PNG_FILTER_NONE as png_byte;
        } else {
            (*png_ptr).do_filter = PNG_ALL_FILTERS as png_byte;
        }
    }

    (*png_ptr).mode = PNG_HAVE_IHDR; /* not READY_FOR_ZTXT */
}

/* Write the palette.  We are careful not to trust png_color to be in the
 * correct order for PNG, so people can redefine it to any convenient
 * structure.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_PLTE(
    png_ptr: png_structrp,
    palette: png_const_colorp,
    num_pal: png_uint_32,
) {
    let max_palette_length: png_uint_32;
    let mut i: png_uint_32;
    let mut pal_ptr: png_const_colorp;
    let mut buf: [png_byte; 3] = [0; 3];

    max_palette_length = if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
        ((1 as c_int) << (*png_ptr).bit_depth as c_int) as png_uint_32
    } else {
        PNG_MAX_PALETTE_LENGTH as png_uint_32
    };

    if (((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_EMPTY_PLTE) == 0 && num_pal == 0)
        || num_pal > max_palette_length
    {
        if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            png_error(png_ptr, cstr!("Invalid number of colors in palette"));
        } else {
            png_warning(png_ptr, cstr!("Invalid number of colors in palette"));
            return;
        }
    }

    if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0 {
        png_warning(
            png_ptr,
            cstr!("Ignoring request to write a PLTE chunk in grayscale PNG"),
        );

        return;
    }

    (*png_ptr).num_palette = num_pal as png_uint_16;

    png_write_chunk_header(png_ptr, png_PLTE, num_pal.wrapping_mul(3) as png_uint_32);

    i = 0;
    pal_ptr = palette;
    while i < num_pal {
        buf[0] = (*pal_ptr).red;
        buf[1] = (*pal_ptr).green;
        buf[2] = (*pal_ptr).blue;
        png_write_chunk_data(png_ptr, buf.as_ptr(), 3);

        i += 1;
        pal_ptr = pal_ptr.add(1);
    }

    png_write_chunk_end(png_ptr);
    (*png_ptr).mode |= PNG_HAVE_PLTE;
}

/* This is similar to png_text_compress, above, except that it does not require
 * all of the data at once and, instead of buffering the compressed result,
 * writes it as IDAT chunks.  Unlike png_text_compress it *can* png_error out
 * because it calls the write interface.  As a result it does its own error
 * reporting and does not return an error code.  In the event of error it will
 * just call png_error.  The input data length may exceed 32-bits.  The 'flush'
 * parameter is exactly the same as that to deflate, with the following
 * meanings:
 *
 * Z_NO_FLUSH: normal incremental output of compressed data
 * Z_SYNC_FLUSH: do a SYNC_FLUSH, used by png_write_flush
 * Z_FINISH: this is the end of the input, do a Z_FINISH and clean up
 *
 * The routine manages the acquire and release of the png_ptr->zstream by
 * checking and (at the end) clearing png_ptr->zowner; it does some sanity
 * checks on the 'mode' flags while doing this.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_compress_IDAT(
    png_ptr: png_structrp,
    row_data: png_const_bytep,
    mut row_data_length: png_alloc_size_t,
    flush: c_int,
) {
    if (*png_ptr).zowner != png_IDAT {
        /* First time.   Ensure we have a temporary buffer for compression and
         * trim the buffer list if it has more than one entry to free memory.
         * If 'WRITE_COMPRESSED_TEXT' is not set the list will never have been
         * created at this point, but the check here is quick and safe.
         */
        if (*png_ptr).zbuffer_list.is_null() {
            (*png_ptr).zbuffer_list =
                png_malloc(png_ptr, PNG_COMPRESSION_BUFFER_SIZE(png_ptr)) as png_compression_bufferp;
            (*(*png_ptr).zbuffer_list).next = core::ptr::null_mut();
        } else {
            png_free_buffer_list(png_ptr, &mut (*(*png_ptr).zbuffer_list).next);
        }

        /* It is a terminal error if we can't claim the zstream. */
        if png_deflate_claim(png_ptr, png_IDAT, png_image_size(png_ptr)) != Z_OK {
            png_error(png_ptr, (*png_ptr).zstream.msg);
        }

        /* The output state is maintained in png_ptr->zstream, so it must be
         * initialized here after the claim.
         */
        (*png_ptr).zstream.next_out =
            core::ptr::addr_of_mut!((*(*png_ptr).zbuffer_list).output) as png_bytep;
        (*png_ptr).zstream.avail_out = (*png_ptr).zbuffer_size;
    }

    /* Now loop reading and writing until all the input is consumed or an error
     * terminates the operation.  The _out values are maintained across calls to
     * this function, but the input must be reset each time.
     */
    (*png_ptr).zstream.next_in = row_data;
    (*png_ptr).zstream.avail_in = 0; /* set below */
    loop {
        let ret: c_int;

        /* INPUT: from the row data */
        let mut avail: uInt = uInt::MAX; /* ZLIB_IO_MAX */

        if avail as png_alloc_size_t > row_data_length {
            avail = row_data_length as uInt; /* safe because of the check */
        }

        (*png_ptr).zstream.avail_in = avail;
        row_data_length -= avail as png_alloc_size_t;

        ret = deflate(
            &mut (*png_ptr).zstream,
            if row_data_length > 0 { Z_NO_FLUSH } else { flush },
        );

        /* Include as-yet unconsumed input */
        row_data_length += (*png_ptr).zstream.avail_in as png_alloc_size_t;
        (*png_ptr).zstream.avail_in = 0;

        /* OUTPUT: write complete IDAT chunks when avail_out drops to zero. Note
         * that these two zstream fields are preserved across the calls, therefore
         * there is no need to set these up on entry to the loop.
         */
        if (*png_ptr).zstream.avail_out == 0 {
            let data: png_bytep =
                core::ptr::addr_of_mut!((*(*png_ptr).zbuffer_list).output) as png_bytep;
            let size: uInt = (*png_ptr).zbuffer_size;

            /* Write an IDAT containing the data then reset the buffer.  The
             * first IDAT may need deflate header optimization.
             */
            if ((*png_ptr).mode & PNG_HAVE_IDAT) == 0
                && (*png_ptr).compression_type as c_int == PNG_COMPRESSION_TYPE_BASE
            {
                optimize_cmf(data, png_image_size(png_ptr));
            }

            if size > 0 {
                png_write_complete_chunk(png_ptr, png_IDAT, data, size as usize);
            }
            (*png_ptr).mode |= PNG_HAVE_IDAT;

            (*png_ptr).zstream.next_out = data;
            (*png_ptr).zstream.avail_out = size;

            /* For SYNC_FLUSH or FINISH it is essential to keep calling zlib with
             * the same flush parameter until it has finished output, for NO_FLUSH
             * it doesn't matter.
             */
            if ret == Z_OK && flush != Z_NO_FLUSH {
                continue;
            }
        }

        /* The order of these checks doesn't matter much; it just affects which
         * possible error might be detected if multiple things go wrong at once.
         */
        if ret == Z_OK
        /* most likely return code! */
        {
            /* If all the input has been consumed then just return.  If Z_FINISH
             * was used as the flush parameter something has gone wrong if we get
             * here.
             */
            if row_data_length == 0 {
                if flush == Z_FINISH {
                    png_error(png_ptr, cstr!("Z_OK on Z_FINISH with output space"));
                }

                return;
            }
        } else if ret == Z_STREAM_END && flush == Z_FINISH {
            /* This is the end of the IDAT data; any pending output must be
             * flushed.  For small PNG files we may still be at the beginning.
             */
            let data: png_bytep =
                core::ptr::addr_of_mut!((*(*png_ptr).zbuffer_list).output) as png_bytep;
            let size: uInt = (*png_ptr).zbuffer_size - (*png_ptr).zstream.avail_out;

            if ((*png_ptr).mode & PNG_HAVE_IDAT) == 0
                && (*png_ptr).compression_type as c_int == PNG_COMPRESSION_TYPE_BASE
            {
                optimize_cmf(data, png_image_size(png_ptr));
            }

            if size > 0 {
                png_write_complete_chunk(png_ptr, png_IDAT, data, size as usize);
            }
            (*png_ptr).zstream.avail_out = 0;
            (*png_ptr).zstream.next_out = core::ptr::null_mut();
            (*png_ptr).mode |= PNG_HAVE_IDAT | PNG_AFTER_IDAT;

            (*png_ptr).zowner = 0; /* Release the stream */
            return;
        } else {
            /* This is an error condition. */
            png_zstream_error(png_ptr, ret);
            png_error(png_ptr, (*png_ptr).zstream.msg);
        }
    }
}

/* Write an IEND chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_IEND(png_ptr: png_structrp) {
    png_write_complete_chunk(png_ptr, png_IEND, core::ptr::null(), 0);
    (*png_ptr).mode |= PNG_HAVE_IEND;
}

/* Write a gAMA chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_gAMA_fixed(png_ptr: png_structrp, file_gamma: png_fixed_point) {
    let mut buf: [png_byte; 4] = [0; 4];

    /* file_gamma is saved in 1/100,000ths */
    png_save_uint_32(buf.as_mut_ptr(), file_gamma as png_uint_32);
    png_write_complete_chunk(png_ptr, png_gAMA, buf.as_ptr(), 4);
}

/* Write a sRGB chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_sRGB(png_ptr: png_structrp, intent: c_int) {
    let mut buf: [png_byte; 1] = [0; 1];

    if intent >= PNG_sRGB_INTENT_LAST {
        png_warning(png_ptr, cstr!("Invalid sRGB rendering intent specified"));
    }

    buf[0] = intent as png_byte;
    png_write_complete_chunk(png_ptr, png_sRGB, buf.as_ptr(), 1);
}

/* Write an iCCP chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_iCCP(
    png_ptr: png_structrp,
    name: png_const_charp,
    profile: png_const_bytep,
    proflen: png_uint_32,
) {
    let mut name_len: png_uint_32;
    let mut new_name: [png_byte; 81] = [0; 81]; /* 1 byte for the compression byte */
    let mut comp: compression_state = core::mem::zeroed();
    let temp: png_uint_32;

    /* These are all internal problems: the profile should have been checked
     * before when it was stored.
     */
    if profile.is_null() {
        png_error(png_ptr, cstr!("No profile for iCCP chunk")); /* internal error */
    }

    if proflen < 132 {
        png_error(png_ptr, cstr!("ICC profile too short"));
    }

    if png_get_uint_32(profile) != proflen {
        png_error(png_ptr, cstr!("Incorrect data in iCCP"));
    }

    temp = *profile.add(8) as png_uint_32;
    if temp > 3 && (proflen & 0x03) != 0 {
        png_error(
            png_ptr,
            cstr!("ICC profile length invalid (not a multiple of 4)"),
        );
    }

    {
        let embedded_profile_len: png_uint_32 = png_get_uint_32(profile);

        if proflen != embedded_profile_len {
            png_error(png_ptr, cstr!("Profile length does not match profile"));
        }
    }

    name_len = png_check_keyword(png_ptr, name, new_name.as_mut_ptr());

    if name_len == 0 {
        png_error(png_ptr, cstr!("iCCP: invalid keyword"));
    }

    name_len += 1;
    new_name[name_len as usize] = PNG_COMPRESSION_TYPE_BASE as png_byte;

    /* Make sure we include the NULL after the name and the compression type */
    name_len += 1;

    png_text_compress_init(&mut comp, profile, proflen as png_alloc_size_t);

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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_sPLT(png_ptr: png_structrp, palette: png_const_sPLT_tp) {
    let name_len: png_uint_32;
    let mut new_name: [png_byte; 80] = [0; 80];
    let mut entrybuf: [png_byte; 10] = [0; 10];
    let entry_size: usize = if (*palette).depth as c_int == 8 { 6 } else { 10 };
    let palette_size: usize = entry_size * (*palette).nentries as usize;
    let mut ep: png_sPLT_entryp;

    name_len = png_check_keyword(png_ptr, (*palette).name, new_name.as_mut_ptr());

    if name_len == 0 {
        png_error(png_ptr, cstr!("sPLT: invalid keyword"));
    }

    /* Make sure we include the NULL after the name */
    png_write_chunk_header(
        png_ptr,
        png_sPLT,
        (name_len as usize + 2 + palette_size) as png_uint_32,
    );

    png_write_chunk_data(png_ptr, new_name.as_mut_ptr(), (name_len + 1) as usize);

    png_write_chunk_data(png_ptr, core::ptr::addr_of!((*palette).depth), 1);

    /* Loop through each palette entry, writing appropriately */
    ep = (*palette).entries;
    while ep < (*palette).entries.offset((*palette).nentries as isize) {
        if (*palette).depth as c_int == 8 {
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
            8 as c_int
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
            png_warning(png_ptr, cstr!("Invalid sBIT depth specified"));
            return;
        }

        buf[0] = (*sbit).red;
        buf[1] = (*sbit).green;
        buf[2] = (*sbit).blue;
        size = 3;
    } else {
        if (*sbit).gray == 0 || (*sbit).gray > (*png_ptr).usr_bit_depth {
            png_warning(png_ptr, cstr!("Invalid sBIT depth specified"));
            return;
        }

        buf[0] = (*sbit).gray;
        size = 1;
    }

    if (color_type & PNG_COLOR_MASK_ALPHA) != 0 {
        if (*sbit).alpha == 0 || (*sbit).alpha > (*png_ptr).usr_bit_depth {
            png_warning(png_ptr, cstr!("Invalid sBIT depth specified"));
            return;
        }

        buf[size] = (*sbit).alpha;
        size += 1;
    }

    png_write_complete_chunk(png_ptr, png_sBIT, buf.as_ptr(), size);
}
