//! Translation of pngwutil.c

use crate::*;

/* Private helper macros translated as inline functions (from png.h / pngpriv.h /
 * pngstruct.h). These are not in the prelude, so they are defined locally. */

#[inline]
fn PNG_PASS_START_ROW(pass: c_int) -> c_int {
    (((1 & !pass) << (3 - (pass >> 1))) & 7) as c_int
}
#[inline]
fn PNG_PASS_START_COL(pass: c_int) -> c_int {
    (((1 & pass) << (3 - ((pass + 1) >> 1))) & 7) as c_int
}
#[inline]
fn PNG_PASS_ROW_SHIFT(pass: c_int) -> c_int {
    if pass > 2 { (8 - pass) >> 1 } else { 3 }
}
#[inline]
fn PNG_PASS_COL_SHIFT(pass: c_int) -> c_int {
    if pass > 1 { (7 - pass) >> 1 } else { 3 }
}
#[inline]
fn PNG_PASS_ROWS(height: png_uint_32, pass: c_int) -> png_uint_32 {
    (height.wrapping_add(
        ((1u32 << PNG_PASS_ROW_SHIFT(pass)) - 1).wrapping_sub(PNG_PASS_START_ROW(pass) as png_uint_32),
    )) >> PNG_PASS_ROW_SHIFT(pass)
}
#[inline]
fn PNG_PASS_COLS(width: png_uint_32, pass: c_int) -> png_uint_32 {
    (width.wrapping_add(
        ((1u32 << PNG_PASS_COL_SHIFT(pass)) - 1).wrapping_sub(PNG_PASS_START_COL(pass) as png_uint_32),
    )) >> PNG_PASS_COL_SHIFT(pass)
}

/* PNGZ_MSG_CAST / PNGZ_INPUT_CAST are no-op casts in this configuration. */
#[inline]
fn PNGZ_MSG_CAST(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}
#[inline]
fn PNGZ_INPUT_CAST(b: png_const_bytep) -> *const Bytef {
    b
}

/* Arrays to facilitate interlacing - use pass (0 - 6) as index. */

/* Start of interlace block */
static png_pass_start: [png_byte; 7] = [0, 4, 0, 2, 0, 1, 0];
/* Offset to next interlace block */
static png_pass_inc: [png_byte; 7] = [8, 8, 4, 4, 2, 2, 1];
/* Start of interlace block in the y direction */
static png_pass_ystart: [png_byte; 7] = [0, 0, 4, 0, 2, 0, 1];
/* Offset to next interlace block in the y direction */
static png_pass_yinc: [png_byte; 7] = [8, 8, 8, 4, 4, 2, 2];

/* Place a 32-bit number into a buffer in PNG byte order.  We work
 * with unsigned numbers for convenience, although one supported
 * ancillary chunk uses signed (two's complement) numbers.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_save_uint_32(buf: png_bytep, i: png_uint_32) {
    unsafe {
        *buf.add(0) = ((i >> 24) & 0xffu32) as png_byte;
        *buf.add(1) = ((i >> 16) & 0xffu32) as png_byte;
        *buf.add(2) = ((i >> 8) & 0xffu32) as png_byte;
        *buf.add(3) = (i & 0xffu32) as png_byte;
    }
}

/* Place a 16-bit number into a buffer in PNG byte order.
 * The parameter is declared unsigned int, not png_uint_16,
 * just to avoid potential problems on pre-ANSI C compilers.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_save_uint_16(buf: png_bytep, i: c_uint) {
    unsafe {
        *buf.add(0) = ((i >> 8) & 0xffu32) as png_byte;
        *buf.add(1) = (i & 0xffu32) as png_byte;
    }
}

/* Simple function to write the signature. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_sig(png_ptr: png_structrp) {
    unsafe {
        let png_signature: [png_byte; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

        /* Inform the I/O callback that the signature is being written */
        (*png_ptr).io_state = PNG_IO_WRITING | PNG_IO_SIGNATURE;

        /* Write the rest of the 8 byte signature */
        png_write_data(
            png_ptr,
            &png_signature[(*png_ptr).sig_bytes as usize],
            (8 - (*png_ptr).sig_bytes) as usize,
        );

        if ((*png_ptr).sig_bytes as c_int) < 3 {
            (*png_ptr).mode |= PNG_HAVE_PNG_SIGNATURE;
        }
    }
}

/* Write the start of a PNG chunk. */
unsafe fn png_write_chunk_header(
    png_ptr: png_structrp,
    chunk_name: png_uint_32,
    length: png_uint_32,
) {
    unsafe {
        let mut buf: [png_byte; 8] = [0; 8];

        if png_ptr.is_null() {
            return;
        }

        /* Inform the I/O callback that the chunk header is being written.
         * PNG_IO_CHUNK_HDR requires a single I/O call.
         */
        (*png_ptr).io_state = PNG_IO_WRITING | PNG_IO_CHUNK_HDR;

        /* Write the length and the chunk name */
        png_save_uint_32(buf.as_mut_ptr(), length);
        png_save_uint_32(buf.as_mut_ptr().add(4), chunk_name);
        png_write_data(png_ptr, buf.as_ptr(), 8);

        /* Put the chunk name into png_ptr->chunk_name */
        (*png_ptr).chunk_name = chunk_name;

        /* Reset the crc and run it over the chunk name */
        png_reset_crc(png_ptr);

        png_calculate_crc(png_ptr, buf.as_ptr().add(4), 4);

        /* Inform the I/O callback that chunk data will (possibly) be written.
         * PNG_IO_CHUNK_DATA does NOT require a specific number of I/O calls.
         */
        (*png_ptr).io_state = PNG_IO_WRITING | PNG_IO_CHUNK_DATA;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_chunk_start(
    png_ptr: png_structrp,
    chunk_string: png_const_bytep,
    length: png_uint_32,
) {
    unsafe {
        png_write_chunk_header(
            png_ptr,
            PNG_CHUNK_FROM_STRING(chunk_string as *const c_char),
            length,
        );
    }
}

/* Write the data of a PNG chunk started with png_write_chunk_header(). */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_chunk_data(
    png_ptr: png_structrp,
    data: png_const_bytep,
    length: usize,
) {
    unsafe {
        /* Write the data, and run the CRC over it */
        if png_ptr.is_null() {
            return;
        }

        if !data.is_null() && length > 0 {
            png_write_data(png_ptr, data, length);

            /* Update the CRC after writing the data,
             * in case the user I/O routine alters it.
             */
            png_calculate_crc(png_ptr, data, length);
        }
    }
}

/* Finish a chunk started with png_write_chunk_header(). */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_chunk_end(png_ptr: png_structrp) {
    unsafe {
        let mut buf: [png_byte; 4] = [0; 4];

        if png_ptr.is_null() {
            return;
        }

        /* Inform the I/O callback that the chunk CRC is being written.
         * PNG_IO_CHUNK_CRC requires a single I/O function call.
         */
        (*png_ptr).io_state = PNG_IO_WRITING | PNG_IO_CHUNK_CRC;

        /* Write the crc in a single operation */
        png_save_uint_32(buf.as_mut_ptr(), (*png_ptr).crc);

        png_write_data(png_ptr, buf.as_ptr(), 4);
    }
}

/* Write a PNG chunk all at once. */
unsafe fn png_write_complete_chunk(
    png_ptr: png_structrp,
    chunk_name: png_uint_32,
    data: png_const_bytep,
    length: usize,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        /* On 64-bit architectures 'length' may not fit in a png_uint_32. */
        if length > PNG_UINT_31_MAX as usize {
            png_error(png_ptr, c"length exceeds PNG maximum".as_ptr());
        }

        png_write_chunk_header(png_ptr, chunk_name, length as png_uint_32);
        png_write_chunk_data(png_ptr, data, length);
        png_write_chunk_end(png_ptr);
    }
}

/* This is the API that calls the internal function above. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_chunk(
    png_ptr: png_structrp,
    chunk_string: png_const_bytep,
    data: png_const_bytep,
    length: usize,
) {
    unsafe {
        png_write_complete_chunk(
            png_ptr,
            PNG_CHUNK_FROM_STRING(chunk_string as *const c_char),
            data,
            length,
        );
    }
}

/* This is used below to find the size of an image to pass to png_deflate_claim,
 * so it only needs to be accurate if the size is less than 16384 bytes (the
 * point at which a lower LZ window size can be used.)
 */
unsafe fn png_image_size(png_ptr: png_structrp) -> png_alloc_size_t {
    unsafe {
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
                let mut cb_base: png_alloc_size_t;
                let mut pass: c_int;

                cb_base = 0;
                pass = 0;
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
}

/* This is the code to hack the first two bytes of the deflate stream (the
 * deflate header) to correct the windowBits value to match the actual data
 * size.
 */
unsafe fn optimize_cmf(data: png_bytep, data_size: png_alloc_size_t) {
    unsafe {
        /* Optimize the CMF field in the zlib stream. */
        if data_size <= 16384
        /* else windowBits must be 15 */
        {
            let mut z_cmf: c_uint = *data.add(0) as c_uint; /* zlib compression method and flags */

            if (z_cmf & 0x0f) == 8 && (z_cmf & 0xf0) <= 0x70 {
                let mut z_cinfo: c_uint;
                let mut half_z_window_size: c_uint;

                z_cinfo = z_cmf >> 4;
                half_z_window_size = 1u32 << (z_cinfo + 7);

                if data_size <= half_z_window_size as png_alloc_size_t
                /* else no change */
                {
                    let mut tmp: c_uint;

                    loop {
                        half_z_window_size >>= 1;
                        z_cinfo -= 1;

                        if !(z_cinfo > 0
                            && data_size <= half_z_window_size as png_alloc_size_t)
                        {
                            break;
                        }
                    }

                    z_cmf = (z_cmf & 0x0f) | (z_cinfo << 4);

                    *data.add(0) = z_cmf as png_byte;
                    tmp = *data.add(1) as c_uint & 0xe0;
                    tmp += 0x1f - ((z_cmf << 8) + tmp) % 0x1f;
                    *data.add(1) = tmp as png_byte;
                }
            }
        }
    }
}

/* Initialize the compressor for the appropriate type of compression. */
unsafe fn png_deflate_claim(
    png_ptr: png_structrp,
    owner: png_uint_32,
    data_size: png_alloc_size_t,
) -> c_int {
    unsafe {
        if (*png_ptr).zowner != 0 {
            let mut msg: [c_char; 64] = [0; 64];

            PNG_STRING_FROM_CHUNK(msg.as_mut_ptr(), owner);
            msg[4] = b':' as c_char;
            msg[5] = b' ' as c_char;
            PNG_STRING_FROM_CHUNK(msg.as_mut_ptr().add(6), (*png_ptr).zowner);
            /* So the message that results is "<chunk> using zstream"; this is an
             * internal error, but is very useful for debugging.  i18n requirements
             * are minimal.
             */
            let _ = png_safecat(
                msg.as_mut_ptr(),
                core::mem::size_of_val(&msg),
                10,
                c" using zstream".as_ptr(),
            );
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
                if deflateEnd(&raw mut (*png_ptr).zstream) != Z_OK {
                    png_warning(png_ptr, c"deflateEnd failed (ignored)".as_ptr());
                }

                (*png_ptr).flags &= !PNG_FLAG_ZSTREAM_INITIALIZED;
            }

            /* For safety clear out the input and output pointers. */
            (*png_ptr).zstream.next_in = core::ptr::null_mut();
            (*png_ptr).zstream.avail_in = 0;
            (*png_ptr).zstream.next_out = core::ptr::null_mut();
            (*png_ptr).zstream.avail_out = 0;

            /* Now initialize if required, setting the new parameters, otherwise just
             * do a simple reset to the previous parameters.
             */
            if ((*png_ptr).flags & PNG_FLAG_ZSTREAM_INITIALIZED) != 0 {
                ret = deflateReset(&raw mut (*png_ptr).zstream);
            } else {
                ret = deflateInit2(
                    &raw mut (*png_ptr).zstream,
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
}

/* Clean up (or trim) a linked list of compression buffers. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_free_buffer_list(
    png_ptr: png_structrp,
    listp: *mut png_compression_bufferp,
) {
    unsafe {
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
}

/* This pair of functions encapsulates the operation of (a) compressing a
 * text string, and (b) issuing it later as a series of chunk data writes.
 */
#[repr(C)]
struct compression_state {
    input: png_const_bytep,       /* The uncompressed input data */
    input_len: png_alloc_size_t,  /* Its length */
    output_len: png_uint_32,      /* Final compressed length */
    output: [png_byte; 1024],     /* First block of output */
}

unsafe fn png_text_compress_init(
    comp: *mut compression_state,
    input: png_const_bytep,
    input_len: png_alloc_size_t,
) {
    unsafe {
        (*comp).input = input;
        (*comp).input_len = input_len;
        (*comp).output_len = 0;
    }
}

/* Compress the data in the compression state input */
unsafe fn png_text_compress(
    png_ptr: png_structrp,
    chunk_name: png_uint_32,
    comp: *mut compression_state,
    prefix_len: png_uint_32,
) -> c_int {
    unsafe {
        let mut ret: c_int;

        ret = png_deflate_claim(png_ptr, chunk_name, (*comp).input_len);

        if ret != Z_OK {
            return ret;
        }

        /* Set up the compression buffers, we need a loop here to avoid overflowing a
         * uInt.
         */
        {
            let mut end: *mut png_compression_bufferp = &raw mut (*png_ptr).zbuffer_list;
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
                    let mut next: *mut png_compression_buffer;

                    /* Chunk data is limited to 2^31 bytes in length, so the prefix
                     * length must be counted here.
                     */
                    if output_len + prefix_len > PNG_UINT_31_MAX {
                        ret = Z_MEM_ERROR;
                        break;
                    }

                    /* Need a new (malloc'ed) buffer, but there may be one present
                     * already.
                     */
                    next = *end;
                    if next.is_null() {
                        next = png_malloc_base(png_ptr, PNG_COMPRESSION_BUFFER_SIZE(png_ptr))
                            as png_compression_bufferp;

                        if next.is_null() {
                            ret = Z_MEM_ERROR;
                            break;
                        }

                        /* Link in this buffer (so that it will be freed later) */
                        (*next).next = core::ptr::null_mut();
                        *end = next;
                    }

                    (*png_ptr).zstream.next_out = (*next).output.as_mut_ptr();
                    (*png_ptr).zstream.avail_out = (*png_ptr).zbuffer_size;
                    output_len += (*png_ptr).zstream.avail_out;

                    /* Move 'end' to the next buffer pointer. */
                    end = &raw mut (*next).next;
                }

                /* Compress the data */
                ret = deflate(
                    &raw mut (*png_ptr).zstream,
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
                (*png_ptr).zstream.msg = PNGZ_MSG_CAST(b"compressed data too long\0");
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
}

/* Ship the compressed text out via chunk writes */
unsafe fn png_write_compressed_data_out(png_ptr: png_structrp, comp: *mut compression_state) {
    unsafe {
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
                c"error writing ancillary chunked compressed data".as_ptr(),
            );
        }
    }
}

/* Write the IHDR chunk, and update the png_struct with the necessary
 * information.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_IHDR(
    png_ptr: png_structrp,
    width: png_uint_32,
    height: png_uint_32,
    bit_depth: c_int,
    color_type: c_int,
    mut compression_type: c_int,
    mut filter_type: c_int,
    mut interlace_type: c_int,
) {
    unsafe {
        let mut buf: [png_byte; 13] = [0; 13]; /* Buffer to store the IHDR info */
        let mut is_invalid_depth: c_int;

        /* Check that we have valid input data from the application info */
        match color_type {
            PNG_COLOR_TYPE_GRAY => match bit_depth {
                1 | 2 | 4 | 8 | 16 => {
                    (*png_ptr).channels = 1;
                }
                _ => {
                    png_error(png_ptr, c"Invalid bit depth for grayscale image".as_ptr());
                }
            },

            PNG_COLOR_TYPE_RGB => {
                is_invalid_depth = (bit_depth != 8) as c_int;
                is_invalid_depth = (is_invalid_depth != 0 && bit_depth != 16) as c_int;
                if is_invalid_depth != 0 {
                    png_error(png_ptr, c"Invalid bit depth for RGB image".as_ptr());
                }

                (*png_ptr).channels = 3;
            }

            PNG_COLOR_TYPE_PALETTE => match bit_depth {
                1 | 2 | 4 | 8 => {
                    (*png_ptr).channels = 1;
                }
                _ => {
                    png_error(png_ptr, c"Invalid bit depth for paletted image".as_ptr());
                }
            },

            PNG_COLOR_TYPE_GRAY_ALPHA => {
                is_invalid_depth = (bit_depth != 8) as c_int;
                is_invalid_depth = (is_invalid_depth != 0 && bit_depth != 16) as c_int;
                if is_invalid_depth != 0 {
                    png_error(
                        png_ptr,
                        c"Invalid bit depth for grayscale+alpha image".as_ptr(),
                    );
                }

                (*png_ptr).channels = 2;
            }

            PNG_COLOR_TYPE_RGB_ALPHA => {
                is_invalid_depth = (bit_depth != 8) as c_int;
                is_invalid_depth = (is_invalid_depth != 0 && bit_depth != 16) as c_int;
                if is_invalid_depth != 0 {
                    png_error(png_ptr, c"Invalid bit depth for RGBA image".as_ptr());
                }

                (*png_ptr).channels = 4;
            }

            _ => {
                png_error(png_ptr, c"Invalid image color type specified".as_ptr());
            }
        }

        if compression_type != PNG_COMPRESSION_TYPE_BASE {
            png_warning(png_ptr, c"Invalid compression type specified".as_ptr());
            compression_type = PNG_COMPRESSION_TYPE_BASE;
        }

        /* Write filter_method 64 (intrapixel differencing) only if [MNG]... */
        if !(((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
            && ((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) == 0
            && (color_type == PNG_COLOR_TYPE_RGB || color_type == PNG_COLOR_TYPE_RGB_ALPHA)
            && (filter_type == PNG_INTRAPIXEL_DIFFERENCING))
            && filter_type != PNG_FILTER_TYPE_BASE
        {
            png_warning(png_ptr, c"Invalid filter type specified".as_ptr());
            filter_type = PNG_FILTER_TYPE_BASE;
        }

        if interlace_type != PNG_INTERLACE_NONE && interlace_type != PNG_INTERLACE_ADAM7 {
            png_warning(png_ptr, c"Invalid interlace type specified".as_ptr());
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
}

/* Write the palette. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_PLTE(
    png_ptr: png_structrp,
    palette: png_const_colorp,
    num_pal: png_uint_32,
) {
    unsafe {
        let max_palette_length: png_uint_32;
        let mut i: png_uint_32;
        let mut pal_ptr: png_const_colorp;
        let mut buf: [png_byte; 3] = [0; 3];

        max_palette_length = if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            1u32 << (*png_ptr).bit_depth
        } else {
            PNG_MAX_PALETTE_LENGTH as png_uint_32
        };

        if (((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_EMPTY_PLTE) == 0 && num_pal == 0)
            || num_pal > max_palette_length
        {
            if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
                png_error(png_ptr, c"Invalid number of colors in palette".as_ptr());
            } else {
                png_warning(png_ptr, c"Invalid number of colors in palette".as_ptr());
                return;
            }
        }

        if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0 {
            png_warning(
                png_ptr,
                c"Ignoring request to write a PLTE chunk in grayscale PNG".as_ptr(),
            );

            return;
        }

        (*png_ptr).num_palette = num_pal as png_uint_16;

        png_write_chunk_header(png_ptr, png_PLTE, num_pal * 3);

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
}

/* This is similar to png_text_compress, above, except that it does not require
 * all of the data at once and, instead of buffering the compressed result,
 * writes it as IDAT chunks.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_compress_IDAT(
    png_ptr: png_structrp,
    input: png_const_bytep,
    mut input_len: png_alloc_size_t,
    flush: c_int,
) {
    unsafe {
        if (*png_ptr).zowner != png_IDAT {
            /* First time.   Ensure we have a temporary buffer for compression and
             * trim the buffer list if it has more than one entry to free memory.
             */
            if (*png_ptr).zbuffer_list.is_null() {
                (*png_ptr).zbuffer_list =
                    png_malloc(png_ptr, PNG_COMPRESSION_BUFFER_SIZE(png_ptr))
                        as png_compression_bufferp;
                (*(*png_ptr).zbuffer_list).next = core::ptr::null_mut();
            } else {
                png_free_buffer_list(png_ptr, &raw mut (*(*png_ptr).zbuffer_list).next);
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
                &raw mut (*png_ptr).zstream,
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

                /* For SYNC_FLUSH or FINISH it is essential to keep calling zlib. */
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
                        png_error(png_ptr, c"Z_OK on Z_FINISH with output space".as_ptr());
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
}

/* Write an IEND chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_IEND(png_ptr: png_structrp) {
    unsafe {
        png_write_complete_chunk(png_ptr, png_IEND, core::ptr::null(), 0);
        (*png_ptr).mode |= PNG_HAVE_IEND;
    }
}

/* Write a gAMA chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_gAMA_fixed(
    png_ptr: png_structrp,
    file_gamma: png_fixed_point,
) {
    unsafe {
        let mut buf: [png_byte; 4] = [0; 4];

        /* file_gamma is saved in 1/100,000ths */
        png_save_uint_32(buf.as_mut_ptr(), file_gamma as png_uint_32);
        png_write_complete_chunk(png_ptr, png_gAMA, buf.as_ptr(), 4);
    }
}

/* Write a sRGB chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_sRGB(png_ptr: png_structrp, srgb_intent: c_int) {
    unsafe {
        let mut buf: [png_byte; 1] = [0; 1];

        if srgb_intent >= PNG_sRGB_INTENT_LAST {
            png_warning(png_ptr, c"Invalid sRGB rendering intent specified".as_ptr());
        }

        buf[0] = srgb_intent as png_byte;
        png_write_complete_chunk(png_ptr, png_sRGB, buf.as_ptr(), 1);
    }
}

/* Write an iCCP chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_iCCP(
    png_ptr: png_structrp,
    name: png_const_charp,
    profile: png_const_bytep,
    profile_len: png_uint_32,
) {
    unsafe {
        let mut name_len: png_uint_32;
        let mut new_name: [png_byte; 81] = [0; 81]; /* 1 byte for the compression byte */
        let mut comp: compression_state = core::mem::zeroed();
        let temp: png_uint_32;

        /* These are all internal problems. */
        if profile.is_null() {
            png_error(png_ptr, c"No profile for iCCP chunk".as_ptr()); /* internal error */
        }

        if profile_len < 132 {
            png_error(png_ptr, c"ICC profile too short".as_ptr());
        }

        if png_get_uint_32(profile) != profile_len {
            png_error(png_ptr, c"Incorrect data in iCCP".as_ptr());
        }

        temp = *profile.add(8) as png_uint_32;
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

        png_text_compress_init(&raw mut comp, profile, profile_len as png_alloc_size_t);

        /* Allow for keyword terminator and compression byte */
        if png_text_compress(png_ptr, png_iCCP, &raw mut comp, name_len) != Z_OK {
            png_error(png_ptr, (*png_ptr).zstream.msg);
        }

        png_write_chunk_header(png_ptr, png_iCCP, name_len + comp.output_len);

        png_write_chunk_data(png_ptr, new_name.as_ptr(), name_len as usize);

        png_write_compressed_data_out(png_ptr, &raw mut comp);

        png_write_chunk_end(png_ptr);
    }
}

/* Write a sPLT chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_sPLT(png_ptr: png_structrp, spalette: png_const_sPLT_tp) {
    unsafe {
        let name_len: png_uint_32;
        let mut new_name: [png_byte; 80] = [0; 80];
        let mut entrybuf: [png_byte; 10] = [0; 10];
        let entry_size: usize = if (*spalette).depth == 8 { 6 } else { 10 };
        let palette_size: usize = entry_size * (*spalette).nentries as usize;
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

        png_write_chunk_data(png_ptr, new_name.as_ptr(), (name_len + 1) as usize);

        png_write_chunk_data(png_ptr, &raw const (*spalette).depth, 1);

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
}

/* Write the sBIT chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_sBIT(
    png_ptr: png_structrp,
    sbit: png_const_color_8p,
    color_type: c_int,
) {
    unsafe {
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
}

/* Write the cHRM chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_cHRM_fixed(png_ptr: png_structrp, xy: *const png_xy) {
    unsafe {
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
    unsafe {
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
            if (*tran).gray as c_int >= (1 << (*png_ptr).bit_depth) {
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
}

/* Write the background chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_bKGD(
    png_ptr: png_structrp,
    back: png_const_color_16p,
    color_type: c_int,
) {
    unsafe {
        let mut buf: [png_byte; 6] = [0; 6];

        if color_type == PNG_COLOR_TYPE_PALETTE {
            if ((*png_ptr).num_palette != 0
                || ((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_EMPTY_PLTE) == 0)
                && (*back).index as png_uint_16 >= (*png_ptr).num_palette
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
            if (*back).gray as c_int >= (1 << (*png_ptr).bit_depth) {
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
    unsafe {
        let mut buf: [png_byte; 4] = [0; 4];

        png_write_chunk_header(png_ptr, png_cICP, 4);

        buf[0] = colour_primaries;
        buf[1] = transfer_function;
        buf[2] = matrix_coefficients;
        buf[3] = video_full_range_flag;
        png_write_chunk_data(png_ptr, buf.as_ptr(), 4);

        png_write_chunk_end(png_ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_cLLI_fixed(
    png_ptr: png_structrp,
    maxCLL: png_uint_32,
    maxFALL: png_uint_32,
) {
    unsafe {
        let mut buf: [png_byte; 8] = [0; 8];

        png_save_uint_32(buf.as_mut_ptr(), maxCLL);
        png_save_uint_32(buf.as_mut_ptr().add(4), maxFALL);

        png_write_complete_chunk(png_ptr, png_cLLI, buf.as_ptr(), 8);
    }
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
    unsafe {
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
}

/* Write the Exif data */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_eXIf(
    png_ptr: png_structrp,
    exif: png_bytep,
    num_exif: c_int,
) {
    unsafe {
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
}

/* Write the histogram */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_hIST(
    png_ptr: png_structrp,
    hist: png_const_uint_16p,
    num_hist: c_int,
) {
    unsafe {
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
            png_save_uint_16(buf.as_mut_ptr(), *hist.add(i as usize) as c_uint);
            png_write_chunk_data(png_ptr, buf.as_ptr(), 2);
            i += 1;
        }

        png_write_chunk_end(png_ptr);
    }
}

/* Write a tEXt chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_tEXt(
    png_ptr: png_structrp,
    key: png_const_charp,
    text: png_const_charp,
    mut text_len: usize,
) {
    unsafe {
        let key_len: png_uint_32;
        let mut new_key: [png_byte; 80] = [0; 80];

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
         * contents of the text.
         */
        png_write_chunk_data(png_ptr, new_key.as_ptr(), (key_len + 1) as usize);

        if text_len != 0 {
            png_write_chunk_data(png_ptr, text as png_const_bytep, text_len);
        }

        png_write_chunk_end(png_ptr);
    }
}

/* Write a compressed text chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_zTXt(
    png_ptr: png_structrp,
    key: png_const_charp,
    text: png_const_charp,
    compression: c_int,
) {
    unsafe {
        let mut key_len: png_uint_32;
        let mut new_key: [png_byte; 81] = [0; 81];
        let mut comp: compression_state = core::mem::zeroed();

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
            &raw mut comp,
            text as png_const_bytep,
            if text.is_null() { 0 } else { strlen(text) },
        );

        if png_text_compress(png_ptr, png_zTXt, &raw mut comp, key_len) != Z_OK {
            png_error(png_ptr, (*png_ptr).zstream.msg);
        }

        /* Write start of chunk */
        png_write_chunk_header(png_ptr, png_zTXt, key_len + comp.output_len);

        /* Write key */
        png_write_chunk_data(png_ptr, new_key.as_ptr(), key_len as usize);

        /* Write the compressed data */
        png_write_compressed_data_out(png_ptr, &raw mut comp);

        /* Close the chunk */
        png_write_chunk_end(png_ptr);
    }
}

/* Write an iTXt chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_iTXt(
    png_ptr: png_structrp,
    mut compression: c_int,
    key: png_const_charp,
    mut lang: png_const_charp,
    mut lang_key: png_const_charp,
    mut text: png_const_charp,
) {
    unsafe {
        let mut key_len: png_uint_32;
        let mut prefix_len: png_uint_32;
        let lang_len: usize;
        let lang_key_len: usize;
        let mut new_key: [png_byte; 82] = [0; 82];
        let mut comp: compression_state = core::mem::zeroed();

        key_len = png_check_keyword(png_ptr, key, new_key.as_mut_ptr());

        if key_len == 0 {
            png_error(png_ptr, c"iTXt: invalid keyword".as_ptr());
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
                png_error(png_ptr, c"iTXt: invalid compression".as_ptr());
            }
        }

        key_len += 1;
        new_key[key_len as usize] = PNG_COMPRESSION_TYPE_BASE as png_byte;
        key_len += 1; /* for the keyword separator */

        /* We leave it to the application to meet PNG-1.0 requirements... */
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

        png_text_compress_init(&raw mut comp, text as png_const_bytep, strlen(text));

        if compression != 0 {
            if png_text_compress(png_ptr, png_iTXt, &raw mut comp, prefix_len) != Z_OK {
                png_error(png_ptr, (*png_ptr).zstream.msg);
            }
        } else {
            if comp.input_len > (PNG_UINT_31_MAX - prefix_len) as usize {
                png_error(png_ptr, c"iTXt: uncompressed text too long".as_ptr());
            }

            /* So the string will fit in a chunk: */
            comp.output_len = comp.input_len as png_uint_32 /*SAFE*/;
        }

        png_write_chunk_header(png_ptr, png_iTXt, comp.output_len + prefix_len);

        png_write_chunk_data(png_ptr, new_key.as_ptr(), key_len as usize);

        png_write_chunk_data(png_ptr, lang as png_const_bytep, lang_len);

        png_write_chunk_data(png_ptr, lang_key as png_const_bytep, lang_key_len);

        if compression != 0 {
            png_write_compressed_data_out(png_ptr, &raw mut comp);
        } else {
            png_write_chunk_data(png_ptr, text as png_const_bytep, comp.output_len as usize);
        }

        png_write_chunk_end(png_ptr);
    }
}

/* Write the oFFs chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_oFFs(
    png_ptr: png_structrp,
    x_offset: png_int_32,
    y_offset: png_int_32,
    unit_type: c_int,
) {
    unsafe {
        let mut buf: [png_byte; 9] = [0; 9];

        if unit_type >= PNG_OFFSET_LAST {
            png_warning(png_ptr, c"Unrecognized unit type for oFFs chunk".as_ptr());
        }

        png_save_int_32(buf.as_mut_ptr(), x_offset);
        png_save_int_32(buf.as_mut_ptr().add(4), y_offset);
        buf[8] = unit_type as png_byte;

        png_write_complete_chunk(png_ptr, png_oFFs, buf.as_ptr(), 9);
    }
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
    unsafe {
        let mut purpose_len: png_uint_32;
        let units_len: usize;
        let mut total_len: usize;
        let params_len: *mut usize;
        let mut buf: [png_byte; 10] = [0; 10];
        let mut new_purpose: [png_byte; 80] = [0; 80];
        let mut i: c_int;

        if type_ >= PNG_EQUATION_LAST {
            png_error(png_ptr, c"Unrecognized equation type for pCAL chunk".as_ptr());
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
}

/* Write the sCAL chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_sCAL_s(
    png_ptr: png_structrp,
    unit: c_int,
    width: png_const_charp,
    height: png_const_charp,
) {
    unsafe {
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
}

/* Write the pHYs chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_pHYs(
    png_ptr: png_structrp,
    x_pixels_per_unit: png_uint_32,
    y_pixels_per_unit: png_uint_32,
    unit_type: c_int,
) {
    unsafe {
        let mut buf: [png_byte; 9] = [0; 9];

        if unit_type >= PNG_RESOLUTION_LAST {
            png_warning(png_ptr, c"Unrecognized unit type for pHYs chunk".as_ptr());
        }

        png_save_uint_32(buf.as_mut_ptr(), x_pixels_per_unit);
        png_save_uint_32(buf.as_mut_ptr().add(4), y_pixels_per_unit);
        buf[8] = unit_type as png_byte;

        png_write_complete_chunk(png_ptr, png_pHYs, buf.as_ptr(), 9);
    }
}

/* Write the tIME chunk. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_tIME(png_ptr: png_structrp, mod_time: png_const_timep) {
    unsafe {
        let mut buf: [png_byte; 7] = [0; 7];

        if (*mod_time).month > 12
            || (*mod_time).month < 1
            || (*mod_time).day > 31
            || (*mod_time).day < 1
            || (*mod_time).hour > 23
            || (*mod_time).second > 60
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
}

/* Initializes the row writing capability of libpng */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_start_row(png_ptr: png_structrp) {
    unsafe {
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

        if (filters as c_int
            & (PNG_FILTER_SUB | PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH))
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
                (*png_ptr).num_rows = ((*png_ptr).height + png_pass_yinc[0] as png_uint_32 - 1
                    - png_pass_ystart[0] as png_uint_32)
                    / png_pass_yinc[0] as png_uint_32;

                (*png_ptr).usr_width = ((*png_ptr).width + png_pass_inc[0] as png_uint_32 - 1
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
}

/* Internal use only.  Called when finished processing a row of data. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_finish_row(png_ptr: png_structrp) {
    unsafe {
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
            if ((*png_ptr).pass as c_int) < 7 {
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
}

/* Pick out the correct pixels for the interlace pass. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_do_write_interlace(
    row_info: png_row_infop,
    row: png_bytep,
    pass: c_int,
) {
    unsafe {
        /* We don't have to do anything on the last pass (6) */
        if pass < 6 {
            /* Each pixel depth is handled separately */
            match (*row_info).pixel_depth as c_int {
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
                        value = ((*sp as c_int >> (7 - (i & 0x07) as c_int)) & 0x01) as c_int;
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
                        value = (*sp as c_int >> (((3 - (i & 0x03) as c_int) << 1))) & 0x03;
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
                        value = (*sp as c_int >> (((1 - (i & 0x01) as c_int) << 2))) & 0x0f;
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
}

/* This filters the row, chooses which filter to use, if it has not already
 * been specified by the application, and then writes the row out with the
 * chosen filter.
 * (Forward declaration; definition is below.)
 */

unsafe fn png_setup_sub_row(
    png_ptr: png_structrp,
    bpp: png_uint_32,
    row_bytes: usize,
    lmins: usize,
) -> usize {
    unsafe {
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
            v = *rp as c_uint;
            *dp = v as png_byte;
            sum += if v < 128 { v as usize } else { 256 - v as usize };
            i += 1;
            rp = rp.add(1);
            dp = dp.add(1);
        }

        lp = (*png_ptr).row_buf.add(1);
        while i < row_bytes {
            v = ((*rp as c_int - *lp as c_int) & 0xff) as c_uint;
            *dp = v as png_byte;
            sum += if v < 128 { v as usize } else { 256 - v as usize };

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
}

unsafe fn png_setup_sub_row_only(png_ptr: png_structrp, bpp: png_uint_32, row_bytes: usize) {
    unsafe {
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
}

unsafe fn png_setup_up_row(png_ptr: png_structrp, row_bytes: usize, lmins: usize) -> usize {
    unsafe {
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
            v = ((*rp as c_int - *pp as c_int) & 0xff) as c_uint;
            *dp = v as png_byte;
            sum += if v < 128 { v as usize } else { 256 - v as usize };

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
}

unsafe fn png_setup_up_row_only(png_ptr: png_structrp, row_bytes: usize) {
    unsafe {
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
}

unsafe fn png_setup_avg_row(
    png_ptr: png_structrp,
    bpp: png_uint_32,
    row_bytes: usize,
    lmins: usize,
) -> usize {
    unsafe {
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
            v = ((*rp as c_int - (*pp as c_int / 2)) & 0xff) as c_uint;
            *dp = v as png_byte;
            dp = dp.add(1);
            rp = rp.add(1);
            pp = pp.add(1);

            sum += if v < 128 { v as usize } else { 256 - v as usize };
            i += 1;
        }

        lp = (*png_ptr).row_buf.add(1);
        while (i as usize) < row_bytes {
            v = ((*rp as c_int - ((*pp as c_int + *lp as c_int) / 2)) & 0xff) as c_uint;
            *dp = v as png_byte;
            dp = dp.add(1);
            rp = rp.add(1);
            pp = pp.add(1);
            lp = lp.add(1);

            sum += if v < 128 { v as usize } else { 256 - v as usize };

            if sum > lmins {
                /* We are already worse, don't continue. */
                break;
            }
            i += 1;
        }

        sum
    }
}

unsafe fn png_setup_avg_row_only(png_ptr: png_structrp, bpp: png_uint_32, row_bytes: usize) {
    unsafe {
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
}

unsafe fn png_setup_paeth_row(
    png_ptr: png_structrp,
    bpp: png_uint_32,
    row_bytes: usize,
    lmins: usize,
) -> usize {
    unsafe {
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
            v = ((*rp as c_int - *pp as c_int) & 0xff) as c_uint;
            *dp = v as png_byte;
            dp = dp.add(1);
            rp = rp.add(1);
            pp = pp.add(1);

            sum += if v < 128 { v as usize } else { 256 - v as usize };
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

            v = ((*rp as c_int - p) & 0xff) as c_uint;
            *dp = v as png_byte;
            dp = dp.add(1);
            rp = rp.add(1);

            sum += if v < 128 { v as usize } else { 256 - v as usize };

            if sum > lmins {
                /* We are already worse, don't continue. */
                break;
            }
            i += 1;
        }

        sum
    }
}

unsafe fn png_setup_paeth_row_only(png_ptr: png_structrp, bpp: png_uint_32, row_bytes: usize) {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_find_filter(png_ptr: png_structrp, row_info: png_row_infop) {
    unsafe {
        let mut filter_to_do: c_uint = (*png_ptr).do_filter as c_uint;
        let row_buf: png_bytep;
        let mut best_row: png_bytep;
        let bpp: png_uint_32;
        let mut mins: usize;
        let row_bytes: usize = (*row_info).rowbytes;

        /* Find out how many bytes offset each pixel is */
        bpp = ((*row_info).pixel_depth as c_uint + 7) >> 3;

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
            filter_to_do &= (0u32).wrapping_sub(filter_to_do);
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
                    sum += if v < 128 { v as usize } else { 256 - v as usize };
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
}

/* Do the actual writing of a previously filtered row. */
unsafe fn png_write_filtered_row(
    png_ptr: png_structrp,
    filtered_row: png_bytep,
    full_row_length: usize, /*includes filter byte*/
) {
    unsafe {
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
}
