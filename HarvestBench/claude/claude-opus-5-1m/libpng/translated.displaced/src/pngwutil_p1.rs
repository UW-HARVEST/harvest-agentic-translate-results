// pngwutil.c - utilities to write a PNG file
//
// Chunk 1: png_save_uint_32 .. png_write_compressed_data_out
//
// This file contains routines that are only called from within
// libpng itself during the course of writing an image.

use crate::*;

/* Place a 32-bit number into a buffer in PNG byte order.  We work
 * with unsigned numbers for convenience, although one supported
 * ancillary chunk uses signed (two's complement) numbers.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_save_uint_32(buf: png_bytep, i: png_uint_32) {
    *buf.offset(0) = ((i >> 24) & 0xffu32) as png_byte;
    *buf.offset(1) = ((i >> 16) & 0xffu32) as png_byte;
    *buf.offset(2) = ((i >> 8) & 0xffu32) as png_byte;
    *buf.offset(3) = (i & 0xffu32) as png_byte;
}

/* Place a 16-bit number into a buffer in PNG byte order.
 * The parameter is declared unsigned int, not png_uint_16,
 * just to avoid potential problems on pre-ANSI C compilers.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_save_uint_16(buf: png_bytep, i: c_uint) {
    *buf.offset(0) = ((i >> 8) & 0xffu32) as png_byte;
    *buf.offset(1) = (i & 0xffu32) as png_byte;
}

/* Simple function to write the signature.  If we have already written
 * the magic bytes of the signature, or more likely, the PNG stream is
 * being embedded into another stream and doesn't need its own signature,
 * we should call png_set_sig_bytes() to tell libpng how many of the
 * bytes have already been written.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_sig(png_ptr: png_structrp) {
    let mut png_signature: [png_byte; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    /* Inform the I/O callback that the signature is being written */
    (*png_ptr).io_state = PNG_IO_WRITING | PNG_IO_SIGNATURE;

    /* Write the rest of the 8 byte signature */
    png_write_data(
        png_ptr,
        png_signature
            .as_mut_ptr()
            .add((*png_ptr).sig_bytes as usize) as png_const_bytep,
        (8 - (*png_ptr).sig_bytes as c_int) as usize,
    );

    if ((*png_ptr).sig_bytes as c_int) < 3 {
        (*png_ptr).mode |= PNG_HAVE_PNG_SIGNATURE;
    }
}

/* Write the start of a PNG chunk.  The type is the chunk type.
 * The total_length is the sum of the lengths of all the data you will be
 * passing in png_write_chunk_data().
 */
unsafe fn png_write_chunk_header(
    png_ptr: png_structrp,
    chunk_name: png_uint_32,
    length: png_uint_32,
) {
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
    png_write_data(png_ptr, buf.as_ptr() as png_const_bytep, 8);

    /* Put the chunk name into png_ptr->chunk_name */
    (*png_ptr).chunk_name = chunk_name;

    /* Reset the crc and run it over the chunk name */
    png_reset_crc(png_ptr);

    png_calculate_crc(png_ptr, buf.as_ptr().add(4) as png_const_bytep, 4);

    /* Inform the I/O callback that chunk data will (possibly) be written.
     * PNG_IO_CHUNK_DATA does NOT require a specific number of I/O calls.
     */
    (*png_ptr).io_state = PNG_IO_WRITING | PNG_IO_CHUNK_DATA;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_chunk_start(
    png_ptr: png_structrp,
    chunk_name: png_const_bytep,
    length: png_uint_32,
) {
    png_write_chunk_header(
        png_ptr,
        PNG_CHUNK_FROM_STRING(chunk_name as *const c_char),
        length,
    );
}

/* Write the data of a PNG chunk started with png_write_chunk_header().
 * Note that multiple calls to this function are allowed, and that the
 * sum of the lengths from these calls *must* add up to the total_length
 * given to png_write_chunk_header().
 */
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

        /* Update the CRC after writing the data,
         * in case the user I/O routine alters it.
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

    /* Inform the I/O callback that the chunk CRC is being written.
     * PNG_IO_CHUNK_CRC requires a single I/O function call.
     */
    (*png_ptr).io_state = PNG_IO_WRITING | PNG_IO_CHUNK_CRC;

    /* Write the crc in a single operation */
    png_save_uint_32(buf.as_mut_ptr(), (*png_ptr).crc);

    png_write_data(png_ptr, buf.as_ptr() as png_const_bytep, 4);
}

/* Write a PNG chunk all at once.  The type is an array of ASCII characters
 * representing the chunk name.  The array must be at least 4 bytes in
 * length, and does not need to be null terminated.  To be safe, pass the
 * pre-defined chunk names here, and if you need a new one, define it
 * where the others are defined.  The length is the length of the data.
 * All the data must be present.  If that is not possible, use the
 * png_write_chunk_start(), png_write_chunk_data(), and png_write_chunk_end()
 * functions instead.
 */
unsafe fn png_write_complete_chunk(
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
        png_error(
            png_ptr as png_const_structrp,
            cstr!("length exceeds PNG maximum"),
        );
    }

    png_write_chunk_header(png_ptr, chunk_name, length as png_uint_32);
    png_write_chunk_data(png_ptr, data, length);
    png_write_chunk_end(png_ptr);
}

/* This is the API that calls the internal function above. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_chunk(
    png_ptr: png_structrp,
    chunk_name: png_const_bytep,
    data: png_const_bytep,
    length: usize,
) {
    png_write_complete_chunk(
        png_ptr,
        PNG_CHUNK_FROM_STRING(chunk_name as *const c_char),
        data,
        length,
    );
}

/* This is used below to find the size of an image to pass to png_deflate_claim,
 * so it only needs to be accurate if the size is less than 16384 bytes (the
 * point at which a lower LZ window size can be used.)
 */
unsafe fn png_image_size(png_ptr: png_structrp) -> png_alloc_size_t {
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

            return cb_base;
        } else {
            return ((*png_ptr).rowbytes + 1) * h as usize;
        }
    } else {
        return 0xffffffffu32 as png_alloc_size_t;
    }
}

/* This is the code to hack the first two bytes of the deflate stream (the
 * deflate header) to correct the windowBits value to match the actual data
 * size.  Note that the second argument is the *uncompressed* size but the
 * first argument is the *compressed* data (and it must be deflate
 * compressed.)
 */
unsafe fn optimize_cmf(data: png_bytep, data_size: png_alloc_size_t) {
    /* Optimize the CMF field in the zlib stream.  The resultant zlib stream is
     * still compliant to the stream specification.
     */
    if data_size <= 16384
    /* else windowBits must be 15 */
    {
        let mut z_cmf: c_uint = *data.offset(0) as c_uint; /* zlib compression method and flags */

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
                    z_cinfo = z_cinfo.wrapping_sub(1);

                    if !(z_cinfo > 0 && data_size <= half_z_window_size as png_alloc_size_t) {
                        break;
                    }
                }

                z_cmf = (z_cmf & 0x0f) | (z_cinfo << 4);

                *data.offset(0) = z_cmf as png_byte;
                tmp = *data.offset(1) as c_uint & 0xe0;
                tmp = tmp.wrapping_add(
                    0x1f - ((z_cmf << 8).wrapping_add(tmp)) % 0x1f,
                );
                *data.offset(1) = tmp as png_byte;
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
        png_safecat(msg.as_mut_ptr(), msg.len(), 10, cstr!(" using zstream"));

        png_error(png_ptr as png_const_structrp, msg.as_ptr() as png_const_charp);
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

        /* Adjust 'windowBits' down if larger than 'data_size'; to stop this
         * happening just pass 32768 as the data_size parameter.  Notice that zlib
         * requires an extra 262 bytes in the window in addition to the data to be
         * able to see the whole of the data, so if data_size+262 takes us to the
         * next windowBits size we need to fix up the value later.  (Because even
         * though deflate needs the extra window, inflate does not!)
         */
        if data_size <= 16384 {
            /* IMPLEMENTATION NOTE: this 'half_window_size' stuff is only here to
             * work round a Microsoft Visual C misbehavior which, contrary to C-90,
             * widens the result of the following shift to 64-bits if (and,
             * apparently, only if) it is used in a test.
             */
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
                png_warning(
                    png_ptr as png_const_structrp,
                    cstr!("deflateEnd failed (ignored)"),
                );
            }

            (*png_ptr).flags &= !PNG_FLAG_ZSTREAM_INITIALIZED;
        }

        /* For safety clear out the input and output pointers (currently zlib
         * doesn't use them on Init, but it might in the future).
         */
        (*png_ptr).zstream.next_in = core::ptr::null();
        (*png_ptr).zstream.avail_in = 0;
        (*png_ptr).zstream.next_out = core::ptr::null_mut();
        (*png_ptr).zstream.avail_out = 0;

        /* Now initialize if required, setting the new parameters, otherwise just
         * do a simple reset to the previous parameters.
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

        /* The return code is from either deflateReset or deflateInit2; they have
         * pretty much the same set of error codes.
         */
        if ret == Z_OK {
            (*png_ptr).zowner = owner;
        } else {
            png_zstream_error(png_ptr, ret);
        }

        return ret;
    }
}

/* Clean up (or trim) a linked list of compression buffers. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_buffer_list(
    png_ptr: png_structrp,
    list: *mut png_compression_bufferp,
) {
    let listp: *mut png_compression_bufferp = list;
    let mut list: png_compression_bufferp = *listp;

    if !list.is_null() {
        *listp = core::ptr::null_mut();

        loop {
            let next: png_compression_bufferp = (*list).next;

            png_free(png_ptr as png_const_structrp, list as png_voidp);
            list = next;

            if list.is_null() {
                break;
            }
        }
    }
}

/* This pair of functions encapsulates the operation of (a) compressing a
 * text string, and (b) issuing it later as a series of chunk data writes.
 * The compression_state structure is shared context for these functions
 * set up by the caller to allow access to the relevant local variables.
 *
 * compression_buffer (new in 1.6.0) is just a linked list of zbuffer_size
 * temporary buffers.  From 1.6.0 it is retained in png_struct so that it will
 * be correctly freed in the event of a write error (previous implementations
 * just leaked memory.)
 */
#[repr(C)]
#[derive(Copy, Clone)]
struct compression_state {
    input: png_const_bytep,      /* The uncompressed input data */
    input_len: png_alloc_size_t, /* Its length */
    output_len: png_uint_32,     /* Final compressed length */
    output: [png_byte; 1024],    /* First block of output */
}

unsafe fn png_text_compress_init(
    comp: *mut compression_state,
    input: png_const_bytep,
    input_len: png_alloc_size_t,
) {
    (*comp).input = input;
    (*comp).input_len = input_len;
    (*comp).output_len = 0;
}

/* Compress the data in the compression state input */
unsafe fn png_text_compress(
    png_ptr: png_structrp,
    chunk_name: png_uint_32,
    comp: *mut compression_state,
    prefix_len: png_uint_32,
) -> c_int {
    let mut ret: c_int;

    /* To find the length of the output it is necessary to first compress the
     * input. The result is buffered rather than using the two-pass algorithm
     * that is used on the inflate side; deflate is assumed to be slower and a
     * PNG writer is assumed to have more memory available than a PNG reader.
     *
     * IMPLEMENTATION NOTE: the zlib API deflateBound() can be used to find an
     * upper limit on the output size, but it is always bigger than the input
     * size so it is likely to be more efficient to use this linked-list
     * approach.
     */
    ret = png_deflate_claim(png_ptr, chunk_name, (*comp).input_len);

    if ret != Z_OK {
        return ret;
    }

    /* Set up the compression buffers, we need a loop here to avoid overflowing a
     * uInt.  Use ZLIB_IO_MAX to limit the input.  The output is always limited
     * by the output buffer size, so there is no need to check that.  Since this
     * is ANSI-C we know that an 'int', hence a uInt, is always at least 16 bits
     * in size.
     */
    {
        let mut end: *mut png_compression_bufferp = &mut (*png_ptr).zbuffer_list;
        let mut input_len: png_alloc_size_t = (*comp).input_len; /* may be zero! */
        let mut output_len: png_uint_32;

        /* zlib updates these for us: */
        (*png_ptr).zstream.next_in = (*comp).input;
        (*png_ptr).zstream.avail_in = 0; /* Set below */
        (*png_ptr).zstream.next_out = (*comp).output.as_mut_ptr();
        (*png_ptr).zstream.avail_out = 1024; /* (sizeof comp->output) */

        output_len = (*png_ptr).zstream.avail_out;

        loop {
            let mut avail_in: uInt = uInt::MAX; /* ZLIB_IO_MAX */

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
                if output_len.wrapping_add(prefix_len) > PNG_UINT_31_MAX {
                    ret = Z_MEM_ERROR;
                    break;
                }

                /* Need a new (malloc'ed) buffer, but there may be one present
                 * already.
                 */
                next = *end;
                if next.is_null() {
                    next = png_malloc_base(
                        png_ptr as png_const_structrp,
                        PNG_COMPRESSION_BUFFER_SIZE(png_ptr as png_const_structrp),
                    ) as png_compression_bufferp;

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
                output_len = output_len.wrapping_add((*png_ptr).zstream.avail_out);

                /* Move 'end' to the next buffer pointer. */
                end = &mut (*next).next;
            }

            /* Compress the data */
            ret = deflate(
                &mut (*png_ptr).zstream,
                if input_len > 0 { Z_NO_FLUSH } else { Z_FINISH },
            );

            /* Claw back input data that was not consumed (because avail_in is
             * reset above every time round the loop).
             */
            input_len += (*png_ptr).zstream.avail_in as png_alloc_size_t;
            (*png_ptr).zstream.avail_in = 0; /* safety */

            if !(ret == Z_OK) {
                break;
            }
        }

        /* There may be some space left in the last output buffer. This needs to
         * be subtracted from output_len.
         */
        output_len = output_len.wrapping_sub((*png_ptr).zstream.avail_out);
        (*png_ptr).zstream.avail_out = 0; /* safety */
        (*comp).output_len = output_len;

        /* Now double check the output length, put in a custom message if it is
         * too long.  Otherwise ensure the z_stream::msg pointer is set to
         * something.
         */
        if output_len.wrapping_add(prefix_len) >= PNG_UINT_31_MAX {
            (*png_ptr).zstream.msg = cstr!("compressed data too long");
            ret = Z_MEM_ERROR;
        } else {
            png_zstream_error(png_ptr, ret);
        }

        /* Reset zlib for another zTXt/iTXt or image data */
        (*png_ptr).zowner = 0;

        /* The only success case is Z_STREAM_END, input_len must be 0; if not this
         * is an internal error.
         */
        if ret == Z_STREAM_END && input_len == 0 {
            /* Fix up the deflate header, if required */
            optimize_cmf((*comp).output.as_mut_ptr(), (*comp).input_len);

            /* But Z_OK is returned, not Z_STREAM_END; this allows the claim
             * function above to return Z_STREAM_END on an error (though it never
             * does in the current versions of zlib.)
             */
            return Z_OK;
        } else {
            return ret;
        }
    }
}

/* Ship the compressed text out via chunk writes */
unsafe fn png_write_compressed_data_out(png_ptr: png_structrp, comp: *mut compression_state) {
    let mut output_len: png_uint_32 = (*comp).output_len;
    let mut output: png_const_bytep = (*comp).output.as_ptr();
    let mut avail: png_uint_32 = 1024; /* (sizeof comp->output) */
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
            png_ptr as png_const_structrp,
            cstr!("error writing ancillary chunked compressed data"),
        );
    }
}
