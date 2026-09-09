//! Translation of c_src/src/pngwutil.c lines 1..1448
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
use crate::prelude::*;

/* ---------------------------------------------------------------- */
/* Interlacing arrays (PNG_WRITE_INTERLACING_SUPPORTED)             */
/* ---------------------------------------------------------------- */

/* Start of interlace block */
pub static png_pass_start: [png_byte; 7] = [0, 4, 0, 2, 0, 1, 0];
/* Offset to next interlace block */
pub static png_pass_inc: [png_byte; 7] = [8, 8, 4, 4, 2, 2, 1];
/* Start of interlace block in the y direction */
pub static png_pass_ystart: [png_byte; 7] = [0, 0, 4, 0, 2, 0, 1];
/* Offset to next interlace block in the y direction */
pub static png_pass_yinc: [png_byte; 7] = [8, 8, 8, 4, 4, 2, 2];

/* ---------------------------------------------------------------- */
/* PNGZ_MSG_CAST / PNGZ_INPUT_CAST helpers (pngpriv.h macros)       */
/* ---------------------------------------------------------------- */

#[inline]
unsafe fn PNGZ_MSG_CAST(s: *const c_char) -> *const c_char {
    s
}

#[inline]
unsafe fn PNGZ_INPUT_CAST(b: png_const_bytep) -> *const Bytef {
    b as *const Bytef
}

/* Place a 32-bit number into a buffer in PNG byte order. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_save_uint_32(buf: png_bytep, i: png_uint_32) {
    *buf.add(0) = ((i >> 24) & 0xffu32) as png_byte;
    *buf.add(1) = ((i >> 16) & 0xffu32) as png_byte;
    *buf.add(2) = ((i >> 8) & 0xffu32) as png_byte;
    *buf.add(3) = (i & 0xffu32) as png_byte;
}

/* Place a 16-bit number into a buffer in PNG byte order. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_save_uint_16(buf: png_bytep, i: c_uint) {
    *buf.add(0) = ((i >> 8) & 0xffu32) as png_byte;
    *buf.add(1) = (i & 0xffu32) as png_byte;
}

/* Simple function to write the signature. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_sig(png_ptr: png_structrp) {
    let mut png_signature: [png_byte; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    /* Inform the I/O callback that the signature is being written */
    (*png_ptr).io_state = PNG_IO_WRITING | PNG_IO_SIGNATURE;

    /* Write the rest of the 8 byte signature */
    png_write_data(
        png_ptr,
        &mut png_signature[(*png_ptr).sig_bytes as usize] as png_bytep,
        (8 - (*png_ptr).sig_bytes as c_int) as usize,
    );

    if ((*png_ptr).sig_bytes as c_int) < 3 {
        (*png_ptr).mode |= PNG_HAVE_PNG_SIGNATURE;
    }
}

/* Write the start of a PNG chunk. */
pub unsafe extern "C" fn png_write_chunk_header(
    png_ptr: png_structrp,
    chunk_name: png_uint_32,
    length: png_uint_32,
) {
    let mut buf: [png_byte; 8] = [0; 8];

    if png_ptr.is_null() {
        return;
    }

    /* Inform the I/O callback that the chunk header is being written. */
    (*png_ptr).io_state = PNG_IO_WRITING | PNG_IO_CHUNK_HDR;

    /* Write the length and the chunk name */
    png_save_uint_32(buf.as_mut_ptr(), length);
    png_save_uint_32(buf.as_mut_ptr().add(4), chunk_name);
    png_write_data(png_ptr, buf.as_mut_ptr(), 8);

    /* Put the chunk name into png_ptr->chunk_name */
    (*png_ptr).chunk_name = chunk_name;

    /* Reset the crc and run it over the chunk name */
    png_reset_crc(png_ptr);

    png_calculate_crc(png_ptr, buf.as_ptr().add(4), 4);

    /* Inform the I/O callback that chunk data will (possibly) be written. */
    (*png_ptr).io_state = PNG_IO_WRITING | PNG_IO_CHUNK_DATA;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_chunk_start(
    png_ptr: png_structrp,
    chunk_string: png_const_bytep,
    length: png_uint_32,
) {
    png_write_chunk_header(
        png_ptr,
        PNG_CHUNK_FROM_STRING(chunk_string as *const c_char),
        length,
    );
}

/* Write the data of a PNG chunk started with png_write_chunk_header(). */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_chunk_data(
    png_ptr: png_structrp,
    data: png_const_bytep,
    length: usize,
) {
    /* Write the data, and run the CRC over it */
    if png_ptr.is_null() {
        return;
    }

    if !data.is_null() && length > 0 {
        png_write_data(png_ptr, data, length);

        /* Update the CRC after writing the data, in case the user I/O
         * routine alters it.
         */
        png_calculate_crc(png_ptr, data, length);
    }
}

/* Finish a chunk started with png_write_chunk_header(). */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_chunk_end(png_ptr: png_structrp) {
    let mut buf: [png_byte; 4] = [0; 4];

    if png_ptr.is_null() {
        return;
    }

    /* Inform the I/O callback that the chunk CRC is being written. */
    (*png_ptr).io_state = PNG_IO_WRITING | PNG_IO_CHUNK_CRC;

    /* Write the crc in a single operation */
    png_save_uint_32(buf.as_mut_ptr(), (*png_ptr).crc);

    png_write_data(png_ptr, buf.as_mut_ptr(), 4);
}

/* Write a PNG chunk all at once. */
pub unsafe extern "C" fn png_write_complete_chunk(
    png_ptr: png_structrp,
    chunk_name: png_uint_32,
    data: png_const_bytep,
    length: usize,
) {
    if png_ptr.is_null() {
        return;
    }

    /* On 64-bit architectures 'length' may not fit in a png_uint_32. */
    if length > PNG_UINT_31_MAX as usize {
        png_error(png_ptr, cstr(b"length exceeds PNG maximum\0"));
    }

    png_write_chunk_header(png_ptr, chunk_name, length as png_uint_32);
    png_write_chunk_data(png_ptr, data, length);
    png_write_chunk_end(png_ptr);
}

/* This is the API that calls the internal function above. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_chunk(
    png_ptr: png_structrp,
    chunk_string: png_const_bytep,
    data: png_const_bytep,
    length: usize,
) {
    png_write_complete_chunk(
        png_ptr,
        PNG_CHUNK_FROM_STRING(chunk_string as *const c_char),
        data,
        length,
    );
}

/* This is used below to find the size of an image to pass to
 * png_deflate_claim.
 */
pub unsafe extern "C" fn png_image_size(png_ptr: png_structrp) -> png_alloc_size_t {
    /* Only return sizes up to the maximum of a png_uint_32; do this by limiting
     * the width and height used to 15 bits.
     */
    let h: png_uint_32 = (*png_ptr).height;

    if (*png_ptr).rowbytes < 32768 && h < 32768 {
        if (*png_ptr).interlaced != 0 {
            /* Interlacing makes the image larger because of the replication of
             * both the filter byte and the padding to a byte boundary.
             */
            let w: png_uint_32 = (*png_ptr).width;
            let pd: c_uint = (*png_ptr).pixel_depth as c_uint;
            let mut cb_base: png_alloc_size_t = 0;
            let mut pass: c_int = 0;

            while pass <= 6 {
                let pw: png_uint_32 = PNG_PASS_COLS(w, pass);

                if pw > 0 {
                    cb_base += (PNG_ROWBYTES(pd as usize, pw as usize) + 1)
                        * PNG_PASS_ROWS(h, pass) as usize;
                }
                pass += 1;
            }

            cb_base
        } else {
            ((*png_ptr).rowbytes + 1) * h as usize
        }
    } else {
        0xffffffffu32 as png_alloc_size_t
    }
}

/* This is the code to hack the first two bytes of the deflate stream (the
 * deflate header) to correct the windowBits value to match the actual data
 * size.
 */
pub unsafe extern "C" fn optimize_cmf(data: png_bytep, data_size: png_alloc_size_t) {
    /* Optimize the CMF field in the zlib stream. */
    if data_size <= 16384
    /* else windowBits must be 15 */
    {
        let z_cmf: c_uint = *data.add(0) as c_uint; /* zlib compression method and flags */

        if (z_cmf & 0x0f) == 8 && (z_cmf & 0xf0) <= 0x70 {
            let mut z_cinfo: c_uint;
            let mut half_z_window_size: c_uint;

            z_cinfo = z_cmf >> 4;
            half_z_window_size = 1u32 << (z_cinfo + 7);

            if data_size <= half_z_window_size as png_alloc_size_t
            /* else no change */
            {
                let mut tmp: c_uint;
                let mut z_cmf = z_cmf;

                loop {
                    half_z_window_size >>= 1;
                    z_cinfo -= 1;
                    if !(z_cinfo > 0 && data_size <= half_z_window_size as png_alloc_size_t) {
                        break;
                    }
                }

                z_cmf = (z_cmf & 0x0f) | (z_cinfo << 4);

                *data.add(0) = z_cmf as png_byte;
                tmp = *data.add(1) as c_uint & 0xe0;
                tmp = tmp.wrapping_add(0x1f - ((z_cmf << 8) + tmp) % 0x1f);
                *data.add(1) = tmp as png_byte;
            }
        }
    }
}

/* Initialize the compressor for the appropriate type of compression. */
pub unsafe extern "C" fn png_deflate_claim(
    png_ptr: png_structrp,
    owner: png_uint_32,
    data_size: png_alloc_size_t,
) -> c_int {
    if (*png_ptr).zowner != 0 {
        let mut msg: [c_char; 64] = [0; 64];

        PNG_STRING_FROM_CHUNK(msg.as_mut_ptr(), owner);
        msg[4] = b':' as c_char;
        msg[5] = b' ' as c_char;
        PNG_STRING_FROM_CHUNK(msg.as_mut_ptr().add(6), (*png_ptr).zowner);
        /* So the message that results is "<chunk> using zstream". */
        let _ = png_safecat(
            msg.as_mut_ptr(),
            core::mem::size_of::<[c_char; 64]>(),
            10,
            cstr(b" using zstream\0"),
        );

        /* PNG_RELEASE_BUILD == 0, so this is the #else branch */
        png_error(png_ptr, msg.as_ptr());
    }

    {
        let mut level: c_int = (*png_ptr).zlib_level;
        let mut method: c_int = (*png_ptr).zlib_method;
        let mut windowBits: c_int = (*png_ptr).zlib_window_bits;
        let mut memLevel: c_int = (*png_ptr).zlib_mem_level;
        let strategy: c_int; /* set below */
        let ret: c_int; /* zlib return code */

        if owner == png_IDAT {
            if ((*png_ptr).flags & PNG_FLAG_ZLIB_CUSTOM_STRATEGY) != 0 {
                strategy = (*png_ptr).zlib_strategy;
            } else if (*png_ptr).do_filter as c_int != PNG_FILTER_NONE {
                strategy = PNG_Z_DEFAULT_STRATEGY;
            } else {
                strategy = PNG_Z_DEFAULT_NOFILTER_STRATEGY;
            }
        } else {
            /* PNG_WRITE_CUSTOMIZE_ZTXT_COMPRESSION_SUPPORTED */
            level = (*png_ptr).zlib_text_level;
            method = (*png_ptr).zlib_text_method;
            windowBits = (*png_ptr).zlib_text_window_bits;
            memLevel = (*png_ptr).zlib_text_mem_level;
            strategy = (*png_ptr).zlib_text_strategy;
        }

        /* Adjust 'windowBits' down if larger than 'data_size'. */
        if data_size <= 16384 {
            let mut half_window_size: c_uint = 1u32 << (windowBits - 1);

            while data_size + 262 <= half_window_size as png_alloc_size_t {
                half_window_size >>= 1;
                windowBits -= 1;
            }
        }

        /* Check against the previous initialized values, if any. */
        if ((*png_ptr).flags & PNG_FLAG_ZSTREAM_INITIALIZED) != 0
            && ((*png_ptr).zlib_set_level != level
                || (*png_ptr).zlib_set_method != method
                || (*png_ptr).zlib_set_window_bits != windowBits
                || (*png_ptr).zlib_set_mem_level != memLevel
                || (*png_ptr).zlib_set_strategy != strategy)
        {
            if deflateEnd(&mut (*png_ptr).zstream) != Z_OK {
                png_warning(png_ptr, cstr(b"deflateEnd failed (ignored)\0"));
            }

            (*png_ptr).flags &= !PNG_FLAG_ZSTREAM_INITIALIZED;
        }

        /* For safety clear out the input and output pointers. */
        (*png_ptr).zstream.next_in = core::ptr::null();
        (*png_ptr).zstream.avail_in = 0;
        (*png_ptr).zstream.next_out = core::ptr::null_mut();
        (*png_ptr).zstream.avail_out = 0;

        /* Now initialize if required, setting the new parameters, otherwise
         * just do a simple reset to the previous parameters.
         */
        if ((*png_ptr).flags & PNG_FLAG_ZSTREAM_INITIALIZED) != 0 {
            ret = deflateReset(&mut (*png_ptr).zstream);
        } else {
            ret = deflateInit2(
                &mut (*png_ptr).zstream,
                level,
                method,
                windowBits,
                memLevel,
                strategy,
            );

            if ret == Z_OK {
                (*png_ptr).flags |= PNG_FLAG_ZSTREAM_INITIALIZED;
            }
        }

        /* The return code is from either deflateReset or deflateInit2. */
        if ret == Z_OK {
            (*png_ptr).zowner = owner;
        } else {
            png_zstream_error(png_ptr, ret);
        }

        ret
    }
}

/* Clean up (or trim) a linked list of compression buffers. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_buffer_list(
    png_ptr: png_structrp,
    listp: *mut png_compression_bufferp,
) {
    let mut list: png_compression_bufferp = *listp;

    if !list.is_null() {
        *listp = core::ptr::null_mut();

        loop {
            let next: png_compression_bufferp = (*list).next;

            png_free(png_ptr, list as png_voidp);
            list = next;

            if list.is_null() {
                break;
            }
        }
    }
}

/* PNG_WRITE_COMPRESSED_TEXT_SUPPORTED
 * This pair of functions encapsulates the operation of (a) compressing a
 * text string, and (b) issuing it later as a series of chunk data writes.
 */
#[repr(C)]
pub struct compression_state {
    pub input: png_const_bytep,      /* The uncompressed input data */
    pub input_len: png_alloc_size_t, /* Its length */
    pub output_len: png_uint_32,     /* Final compressed length */
    pub output: [png_byte; 1024],    /* First block of output */
}

pub unsafe extern "C" fn png_text_compress_init(
    comp: *mut compression_state,
    input: png_const_bytep,
    input_len: png_alloc_size_t,
) {
    (*comp).input = input;
    (*comp).input_len = input_len;
    (*comp).output_len = 0;
}

/* Compress the data in the compression state input */
pub unsafe extern "C" fn png_text_compress(
    png_ptr: png_structrp,
    chunk_name: png_uint_32,
    comp: *mut compression_state,
    prefix_len: png_uint_32,
) -> c_int {
    let mut ret: c_int;

    /* To find the length of the output it is necessary to first compress the
     * input.
     */
    ret = png_deflate_claim(png_ptr, chunk_name, (*comp).input_len);

    if ret != Z_OK {
        return ret;
    }

    /* Set up the compression buffers. */
    {
        let mut end: *mut png_compression_bufferp = &mut (*png_ptr).zbuffer_list;
        let mut input_len: png_alloc_size_t = (*comp).input_len; /* may be zero! */
        let mut output_len: png_uint_32;

        /* zlib updates these for us: */
        (*png_ptr).zstream.next_in = PNGZ_INPUT_CAST((*comp).input);
        (*png_ptr).zstream.avail_in = 0; /* Set below */
        (*png_ptr).zstream.next_out = (*comp).output.as_mut_ptr();
        (*png_ptr).zstream.avail_out = core::mem::size_of_val(&(*comp).output) as uInt;

        output_len = (*png_ptr).zstream.avail_out;

        loop {
            let mut avail_in: uInt = ZLIB_IO_MAX;

            if avail_in as png_alloc_size_t > input_len {
                avail_in = input_len as uInt;
            }

            input_len -= avail_in as png_alloc_size_t;

            (*png_ptr).zstream.avail_in = avail_in;

            if (*png_ptr).zstream.avail_out == 0 {
                let next: *mut png_compression_buffer;

                /* Chunk data is limited to 2^31 bytes in length. */
                if output_len + prefix_len > PNG_UINT_31_MAX {
                    ret = Z_MEM_ERROR;
                    break;
                }

                /* Need a new (malloc'ed) buffer, but there may be one present
                 * already.
                 */
                let next_existing = *end;
                if next_existing.is_null() {
                    next = png_malloc_base(png_ptr, PNG_COMPRESSION_BUFFER_SIZE(png_ptr))
                        as png_compression_bufferp;

                    if next.is_null() {
                        ret = Z_MEM_ERROR;
                        break;
                    }

                    /* Link in this buffer (so that it will be freed later) */
                    (*next).next = core::ptr::null_mut();
                    *end = next;
                } else {
                    next = next_existing;
                }

                (*png_ptr).zstream.next_out = (*next).output.as_mut_ptr();
                (*png_ptr).zstream.avail_out = (*png_ptr).zbuffer_size;
                output_len += (*png_ptr).zstream.avail_out;

                /* Move 'end' to the next buffer pointer. */
                end = &mut (*next).next;
            }

            /* Compress the data */
            ret = deflate(
                &mut (*png_ptr).zstream,
                if input_len > 0 { Z_NO_FLUSH } else { Z_FINISH },
            );

            /* Claw back input data that was not consumed. */
            input_len += (*png_ptr).zstream.avail_in as png_alloc_size_t;
            (*png_ptr).zstream.avail_in = 0; /* safety */

            if ret != Z_OK {
                break;
            }
        }

        /* There may be some space left in the last output buffer. */
        output_len -= (*png_ptr).zstream.avail_out;
        (*png_ptr).zstream.avail_out = 0; /* safety */
        (*comp).output_len = output_len;

        /* Now double check the output length. */
        if output_len + prefix_len >= PNG_UINT_31_MAX {
            (*png_ptr).zstream.msg = PNGZ_MSG_CAST(cstr(b"compressed data too long\0"));
            ret = Z_MEM_ERROR;
        } else {
            png_zstream_error(png_ptr, ret);
        }

        /* Reset zlib for another zTXt/iTXt or image data */
        (*png_ptr).zowner = 0;

        /* The only success case is Z_STREAM_END, input_len must be 0. */
        if ret == Z_STREAM_END && input_len == 0 {
            /* Fix up the deflate header, if required */
            optimize_cmf((*comp).output.as_mut_ptr(), (*comp).input_len);
            /* But Z_OK is returned, not Z_STREAM_END. */
            Z_OK
        } else {
            ret
        }
    }
}

/* Ship the compressed text out via chunk writes */
pub unsafe extern "C" fn png_write_compressed_data_out(
    png_ptr: png_structrp,
    comp: *mut compression_state,
) {
    let mut output_len: png_uint_32 = (*comp).output_len;
    let mut output: png_const_bytep = (*comp).output.as_ptr();
    let mut avail: png_uint_32 = core::mem::size_of_val(&(*comp).output) as png_uint_32;
    let mut next: *mut png_compression_buffer = (*png_ptr).zbuffer_list;

    loop {
        if avail > output_len {
            avail = output_len;
        }

        png_write_chunk_data(png_ptr, output, avail as usize);

        output_len -= avail;

        if output_len == 0 || next.is_null() {
            break;
        }

        avail = (*png_ptr).zbuffer_size;
        output = (*next).output.as_ptr();
        next = (*next).next;
    }

    /* This is an internal error; 'next' must have been NULL! */
    if output_len > 0 {
        png_error(
            png_ptr,
            cstr(b"error writing ancillary chunked compressed data\0"),
        );
    }
}

/* Write the IHDR chunk. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_IHDR(
    png_ptr: png_structrp,
    width: png_uint_32,
    height: png_uint_32,
    bit_depth: c_int,
    color_type: c_int,
    mut compression_type: c_int,
    mut filter_type: c_int,
    mut interlace_type: c_int,
) {
    let mut buf: [png_byte; 13] = [0; 13]; /* Buffer to store the IHDR info */
    let mut is_invalid_depth: c_int;

    /* Check that we have valid input data from the application info */
    if color_type == PNG_COLOR_TYPE_GRAY {
        match bit_depth {
            1 | 2 | 4 | 8 | 16 => {
                (*png_ptr).channels = 1;
            }
            _ => {
                png_error(png_ptr, cstr(b"Invalid bit depth for grayscale image\0"));
            }
        }
    } else if color_type == PNG_COLOR_TYPE_RGB {
        is_invalid_depth = (bit_depth != 8) as c_int;
        is_invalid_depth = (is_invalid_depth != 0 && bit_depth != 16) as c_int;
        if is_invalid_depth != 0 {
            png_error(png_ptr, cstr(b"Invalid bit depth for RGB image\0"));
        }

        (*png_ptr).channels = 3;
    } else if color_type == PNG_COLOR_TYPE_PALETTE {
        match bit_depth {
            1 | 2 | 4 | 8 => {
                (*png_ptr).channels = 1;
            }
            _ => {
                png_error(png_ptr, cstr(b"Invalid bit depth for paletted image\0"));
            }
        }
    } else if color_type == PNG_COLOR_TYPE_GRAY_ALPHA {
        is_invalid_depth = (bit_depth != 8) as c_int;
        is_invalid_depth = (is_invalid_depth != 0 && bit_depth != 16) as c_int;
        if is_invalid_depth != 0 {
            png_error(
                png_ptr,
                cstr(b"Invalid bit depth for grayscale+alpha image\0"),
            );
        }

        (*png_ptr).channels = 2;
    } else if color_type == PNG_COLOR_TYPE_RGB_ALPHA {
        is_invalid_depth = (bit_depth != 8) as c_int;
        is_invalid_depth = (is_invalid_depth != 0 && bit_depth != 16) as c_int;
        if is_invalid_depth != 0 {
            png_error(png_ptr, cstr(b"Invalid bit depth for RGBA image\0"));
        }

        (*png_ptr).channels = 4;
    } else {
        png_error(png_ptr, cstr(b"Invalid image color type specified\0"));
    }

    if compression_type != PNG_COMPRESSION_TYPE_BASE {
        png_warning(png_ptr, cstr(b"Invalid compression type specified\0"));
        compression_type = PNG_COMPRESSION_TYPE_BASE;
    }

    /* Write filter_method 64 (intrapixel differencing) only under MNG rules. */
    if !(((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
        && ((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) == 0
        && (color_type == PNG_COLOR_TYPE_RGB || color_type == PNG_COLOR_TYPE_RGB_ALPHA)
        && (filter_type == PNG_INTRAPIXEL_DIFFERENCING))
        && filter_type != PNG_FILTER_TYPE_BASE
    {
        png_warning(png_ptr, cstr(b"Invalid filter type specified\0"));
        filter_type = PNG_FILTER_TYPE_BASE;
    }

    if interlace_type != PNG_INTERLACE_NONE && interlace_type != PNG_INTERLACE_ADAM7 {
        png_warning(png_ptr, cstr(b"Invalid interlace type specified\0"));
        interlace_type = PNG_INTERLACE_ADAM7;
    }

    /* Save the relevant information */
    (*png_ptr).bit_depth = bit_depth as png_byte;
    (*png_ptr).color_type = color_type as png_byte;
    (*png_ptr).interlaced = interlace_type as png_byte;
    (*png_ptr).filter_type = filter_type as png_byte;
    (*png_ptr).compression_type = compression_type as png_byte;
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
    buf[10] = compression_type as png_byte;
    buf[11] = filter_type as png_byte;
    buf[12] = interlace_type as png_byte;

    /* Write the chunk */
    png_write_complete_chunk(png_ptr, png_IHDR, buf.as_ptr(), 13);

    if (*png_ptr).do_filter as c_int == PNG_NO_FILTERS {
        if (*png_ptr).color_type == PNG_COLOR_TYPE_PALETTE as png_byte || (*png_ptr).bit_depth < 8 {
            (*png_ptr).do_filter = PNG_FILTER_NONE as png_byte;
        } else {
            (*png_ptr).do_filter = PNG_ALL_FILTERS as png_byte;
        }
    }

    (*png_ptr).mode = PNG_HAVE_IHDR; /* not READY_FOR_ZTXT */
}

/* Write the palette. */
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

    max_palette_length = if (*png_ptr).color_type == PNG_COLOR_TYPE_PALETTE as png_byte {
        1u32 << (*png_ptr).bit_depth
    } else {
        PNG_MAX_PALETTE_LENGTH as png_uint_32
    };

    if (((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_EMPTY_PLTE) == 0 && num_pal == 0)
        || num_pal > max_palette_length
    {
        if (*png_ptr).color_type == PNG_COLOR_TYPE_PALETTE as png_byte {
            png_error(png_ptr, cstr(b"Invalid number of colors in palette\0"));
        } else {
            png_warning(png_ptr, cstr(b"Invalid number of colors in palette\0"));
            return;
        }
    }

    if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0 {
        png_warning(
            png_ptr,
            cstr(b"Ignoring request to write a PLTE chunk in grayscale PNG\0"),
        );

        return;
    }

    (*png_ptr).num_palette = num_pal as png_uint_16;

    png_write_chunk_header(png_ptr, png_PLTE, num_pal.wrapping_mul(3));

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

/* This is similar to png_text_compress, above, except that it does not
 * require all of the data at once and writes it as IDAT chunks.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_compress_IDAT(
    png_ptr: png_structrp,
    input: png_const_bytep,
    mut input_len: png_alloc_size_t,
    flush: c_int,
) {
    if (*png_ptr).zowner != png_IDAT {
        /* First time. Ensure we have a temporary buffer for compression. */
        if (*png_ptr).zbuffer_list.is_null() {
            (*png_ptr).zbuffer_list = png_malloc(png_ptr, PNG_COMPRESSION_BUFFER_SIZE(png_ptr))
                as png_compression_bufferp;
            (*(*png_ptr).zbuffer_list).next = core::ptr::null_mut();
        } else {
            png_free_buffer_list(png_ptr, &mut (*(*png_ptr).zbuffer_list).next);
        }

        /* It is a terminal error if we can't claim the zstream. */
        if png_deflate_claim(png_ptr, png_IDAT, png_image_size(png_ptr)) != Z_OK {
            png_error(png_ptr, (*png_ptr).zstream.msg);
        }

        /* The output state is maintained in png_ptr->zstream. */
        (*png_ptr).zstream.next_out = (*(*png_ptr).zbuffer_list).output.as_mut_ptr();
        (*png_ptr).zstream.avail_out = (*png_ptr).zbuffer_size;
    }

    /* Now loop reading and writing until all the input is consumed. */
    (*png_ptr).zstream.next_in = PNGZ_INPUT_CAST(input);
    (*png_ptr).zstream.avail_in = 0; /* set below */
    loop {
        let ret: c_int;

        /* INPUT: from the row data */
        let mut avail: uInt = ZLIB_IO_MAX;

        if avail as png_alloc_size_t > input_len {
            avail = input_len as uInt; /* safe because of the check */
        }

        (*png_ptr).zstream.avail_in = avail;
        input_len -= avail as png_alloc_size_t;

        ret = deflate(
            &mut (*png_ptr).zstream,
            if input_len > 0 { Z_NO_FLUSH } else { flush },
        );

        /* Include as-yet unconsumed input */
        input_len += (*png_ptr).zstream.avail_in as png_alloc_size_t;
        (*png_ptr).zstream.avail_in = 0;

        /* OUTPUT: write complete IDAT chunks when avail_out drops to zero. */
        if (*png_ptr).zstream.avail_out == 0 {
            let data: png_bytep = (*(*png_ptr).zbuffer_list).output.as_mut_ptr();
            let size: uInt = (*png_ptr).zbuffer_size;

            /* Write an IDAT containing the data then reset the buffer. */
            if ((*png_ptr).mode & PNG_HAVE_IDAT) == 0
                && (*png_ptr).compression_type == PNG_COMPRESSION_TYPE_BASE as png_byte
            {
                optimize_cmf(data, png_image_size(png_ptr));
            }

            if size > 0 {
                png_write_complete_chunk(png_ptr, png_IDAT, data, size as usize);
            }
            (*png_ptr).mode |= PNG_HAVE_IDAT;

            (*png_ptr).zstream.next_out = data;
            (*png_ptr).zstream.avail_out = size;

            /* For SYNC_FLUSH or FINISH it is essential to keep calling zlib
             * with the same flush parameter until it has finished output.
             */
            if ret == Z_OK && flush != Z_NO_FLUSH {
                continue;
            }
        }

        /* The order of these checks doesn't matter much. */
        if ret == Z_OK
        /* most likely return code! */
        {
            /* If all the input has been consumed then just return. */
            if input_len == 0 {
                if flush == Z_FINISH {
                    png_error(png_ptr, cstr(b"Z_OK on Z_FINISH with output space\0"));
                }

                return;
            }
        } else if ret == Z_STREAM_END && flush == Z_FINISH {
            /* This is the end of the IDAT data; any pending output must be
             * flushed.
             */
            let data: png_bytep = (*(*png_ptr).zbuffer_list).output.as_mut_ptr();
            let size: uInt = (*png_ptr).zbuffer_size - (*png_ptr).zstream.avail_out;

            if ((*png_ptr).mode & PNG_HAVE_IDAT) == 0
                && (*png_ptr).compression_type == PNG_COMPRESSION_TYPE_BASE as png_byte
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
pub unsafe extern "C" fn png_write_sRGB(png_ptr: png_structrp, srgb_intent: c_int) {
    let mut buf: [png_byte; 1] = [0; 1];

    if srgb_intent >= PNG_sRGB_INTENT_LAST {
        png_warning(png_ptr, cstr(b"Invalid sRGB rendering intent specified\0"));
    }

    buf[0] = srgb_intent as png_byte;
    png_write_complete_chunk(png_ptr, png_sRGB, buf.as_ptr(), 1);
}

/* Write an iCCP chunk */
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

    /* These are all internal problems. */
    if profile.is_null() {
        png_error(png_ptr, cstr(b"No profile for iCCP chunk\0")); /* internal error */
    }

    if profile_len < 132 {
        png_error(png_ptr, cstr(b"ICC profile too short\0"));
    }

    if png_get_uint_32(profile) != profile_len {
        png_error(png_ptr, cstr(b"Incorrect data in iCCP\0"));
    }

    temp = *profile.add(8) as png_uint_32;
    if temp > 3 && (profile_len & 0x03) != 0 {
        png_error(
            png_ptr,
            cstr(b"ICC profile length invalid (not a multiple of 4)\0"),
        );
    }

    {
        let embedded_profile_len: png_uint_32 = png_get_uint_32(profile);

        if profile_len != embedded_profile_len {
            png_error(png_ptr, cstr(b"Profile length does not match profile\0"));
        }
    }

    name_len = png_check_keyword(png_ptr, name, new_name.as_mut_ptr());

    if name_len == 0 {
        png_error(png_ptr, cstr(b"iCCP: invalid keyword\0"));
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
pub unsafe extern "C" fn png_write_sPLT(png_ptr: png_structrp, spalette: png_const_sPLT_tp) {
    let name_len: png_uint_32;
    let mut new_name: [png_byte; 80] = [0; 80];
    let mut entrybuf: [png_byte; 10] = [0; 10];
    let entry_size: usize = if (*spalette).depth == 8 { 6 } else { 10 };
    let palette_size: usize = entry_size * (*spalette).nentries as usize;
    let mut ep: png_sPLT_entryp;

    name_len = png_check_keyword(png_ptr, (*spalette).name, new_name.as_mut_ptr());

    if name_len == 0 {
        png_error(png_ptr, cstr(b"sPLT: invalid keyword\0"));
    }

    /* Make sure we include the NULL after the name */
    png_write_chunk_header(
        png_ptr,
        png_sPLT,
        (name_len as usize + 2 + palette_size) as png_uint_32,
    );

    png_write_chunk_data(png_ptr, new_name.as_ptr(), (name_len + 1) as usize);

    png_write_chunk_data(png_ptr, &(*spalette).depth as *const png_byte, 1);

    /* Loop through each palette entry, writing appropriately */
    ep = (*spalette).entries;
    while ep < (*spalette).entries.add((*spalette).nentries as usize) {
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

        maxbits = if color_type == PNG_COLOR_TYPE_PALETTE {
            8
        } else {
            (*png_ptr).usr_bit_depth
        } as png_byte;

        if (*sbit).red == 0
            || (*sbit).red > maxbits
            || (*sbit).green == 0
            || (*sbit).green > maxbits
            || (*sbit).blue == 0
            || (*sbit).blue > maxbits
        {
            png_warning(png_ptr, cstr(b"Invalid sBIT depth specified\0"));
            return;
        }

        buf[0] = (*sbit).red;
        buf[1] = (*sbit).green;
        buf[2] = (*sbit).blue;
        size = 3;
    } else {
        if (*sbit).gray == 0 || (*sbit).gray > (*png_ptr).usr_bit_depth {
            png_warning(png_ptr, cstr(b"Invalid sBIT depth specified\0"));
            return;
        }

        buf[0] = (*sbit).gray;
        size = 1;
    }

    if (color_type & PNG_COLOR_MASK_ALPHA) != 0 {
        if (*sbit).alpha == 0 || (*sbit).alpha > (*png_ptr).usr_bit_depth {
            png_warning(png_ptr, cstr(b"Invalid sBIT depth specified\0"));
            return;
        }

        buf[size] = (*sbit).alpha;
        size += 1;
    }

    png_write_complete_chunk(png_ptr, png_sBIT, buf.as_ptr(), size);
}

/* Write the cHRM chunk */
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
                cstr(b"Invalid number of transparent colors specified\0"),
            );
            return;
        }

        /* Write the chunk out as it is */
        png_write_complete_chunk(png_ptr, png_tRNS, trans_alpha, num_trans as usize);
    } else if color_type == PNG_COLOR_TYPE_GRAY {
        /* One 16-bit value */
        if (*tran).gray as c_int >= (1 << (*png_ptr).bit_depth) {
            png_app_warning(
                png_ptr,
                cstr(b"Ignoring attempt to write tRNS chunk out-of-range for bit_depth\0"),
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
        if (*png_ptr).bit_depth == 8 && (buf[0] as c_int | buf[2] as c_int | buf[4] as c_int) != 0 {
            png_app_warning(
                png_ptr,
                cstr(b"Ignoring attempt to write 16-bit tRNS chunk when bit_depth is 8\0"),
            );
            return;
        }

        png_write_complete_chunk(png_ptr, png_tRNS, buf.as_ptr(), 6);
    } else {
        png_app_warning(png_ptr, cstr(b"Can't write tRNS with an alpha channel\0"));
    }
}

/* Write the background chunk */
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
            && (*back).index as png_uint_16 >= (*png_ptr).num_palette
        {
            png_warning(png_ptr, cstr(b"Invalid background palette index\0"));
            return;
        }

        buf[0] = (*back).index;
        png_write_complete_chunk(png_ptr, png_bKGD, buf.as_ptr(), 1);
    } else if (color_type & PNG_COLOR_MASK_COLOR) != 0 {
        png_save_uint_16(buf.as_mut_ptr(), (*back).red as c_uint);
        png_save_uint_16(buf.as_mut_ptr().add(2), (*back).green as c_uint);
        png_save_uint_16(buf.as_mut_ptr().add(4), (*back).blue as c_uint);
        if (*png_ptr).bit_depth == 8 && (buf[0] as c_int | buf[2] as c_int | buf[4] as c_int) != 0 {
            png_warning(
                png_ptr,
                cstr(b"Ignoring attempt to write 16-bit bKGD chunk when bit_depth is 8\0"),
            );

            return;
        }

        png_write_complete_chunk(png_ptr, png_bKGD, buf.as_ptr(), 6);
    } else {
        if (*back).gray as c_int >= (1 << (*png_ptr).bit_depth) {
            png_warning(
                png_ptr,
                cstr(b"Ignoring attempt to write bKGD chunk out-of-range for bit_depth\0"),
            );

            return;
        }

        png_save_uint_16(buf.as_mut_ptr(), (*back).gray as c_uint);
        png_write_complete_chunk(png_ptr, png_bKGD, buf.as_ptr(), 2);
    }
}
