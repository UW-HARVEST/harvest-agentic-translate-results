/* pngwutil.c lines 1..308 */

/* Place a 32-bit number into a buffer in PNG byte order.  We work
 * with unsigned numbers for convenience, although one supported
 * ancillary chunk uses signed (two's complement) numbers.
 */
/* png_save_uint_32 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_save_uint_32(buf: png_bytep, i: png_uint_32) {
    *buf.add(0) = ((i >> 24) & 0xffu32) as png_byte;
    *buf.add(1) = ((i >> 16) & 0xffu32) as png_byte;
    *buf.add(2) = ((i >> 8) & 0xffu32) as png_byte;
    *buf.add(3) = (i & 0xffu32) as png_byte;
}

/* Place a 16-bit number into a buffer in PNG byte order.
 * The parameter is declared unsigned int, not png_uint_16,
 * just to avoid potential problems on pre-ANSI C compilers.
 */
/* png_save_uint_16 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_save_uint_16(buf: png_bytep, i: c_uint) {
    *buf.add(0) = ((i >> 8) & 0xffu32) as png_byte;
    *buf.add(1) = (i & 0xffu32) as png_byte;
}

/* Simple function to write the signature.  If we have already written
 * the magic bytes of the signature, or more likely, the PNG stream is
 * being embedded into another stream and doesn't need its own signature,
 * we should call png_set_sig_bytes() to tell libpng how many of the
 * bytes have already been written.
 */
/* png_write_sig */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_sig(png_ptr: png_structrp) {
    let mut png_signature: [png_byte; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    /* Inform the I/O callback that the signature is being written */
    (*png_ptr).io_state = PNG_IO_WRITING | PNG_IO_SIGNATURE;

    /* Write the rest of the 8 byte signature */
    png_write_data(
        png_ptr,
        png_signature.as_mut_ptr().add((*png_ptr).sig_bytes as usize),
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
/* png_write_chunk_header (static) */
unsafe fn png_write_chunk_header(
    png_ptr: png_structrp,
    chunk_name: png_uint_32,
    length: png_uint_32,
) {
    let mut buf: [png_byte; 8] = [0; 8];

    if png_ptr == core::ptr::null_mut() {
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

/* png_write_chunk_start */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_chunk_start(
    png_ptr: png_structrp,
    chunk_string: png_const_bytep,
    length: png_uint_32,
) {
    png_write_chunk_header(png_ptr, PNG_CHUNK_FROM_STRING(chunk_string), length);
}

/* Write the data of a PNG chunk started with png_write_chunk_header().
 * Note that multiple calls to this function are allowed, and that the
 * sum of the lengths from these calls *must* add up to the total_length
 * given to png_write_chunk_header().
 */
/* png_write_chunk_data */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_chunk_data(
    png_ptr: png_structrp,
    data: png_const_bytep,
    length: usize,
) {
    /* Write the data, and run the CRC over it */
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    if data != core::ptr::null() && length > 0 {
        png_write_data(png_ptr, data, length);

        /* Update the CRC after writing the data,
         * in case the user I/O routine alters it.
         */
        png_calculate_crc(png_ptr, data, length);
    }
}

/* Finish a chunk started with png_write_chunk_header(). */
/* png_write_chunk_end */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_chunk_end(png_ptr: png_structrp) {
    let mut buf: [png_byte; 4] = [0; 4];

    if png_ptr == core::ptr::null_mut() {
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

/* Write a PNG chunk all at once.  The type is an array of ASCII characters
 * representing the chunk name.  The array must be at least 4 bytes in
 * length, and does not need to be null terminated.  To be safe, pass the
 * pre-defined chunk names here, and if you need a new one, define it
 * where the others are defined.  The length is the length of the data.
 * All the data must be present.  If that is not possible, use the
 * png_write_chunk_start(), png_write_chunk_data(), and png_write_chunk_end()
 * functions instead.
 */
/* png_write_complete_chunk (static) */
unsafe fn png_write_complete_chunk(
    png_ptr: png_structrp,
    chunk_name: png_uint_32,
    data: png_const_bytep,
    length: usize,
) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    /* On 64-bit architectures 'length' may not fit in a png_uint_32. */
    if length > PNG_UINT_31_MAX as usize {
        png_error(
            png_ptr,
            b"length exceeds PNG maximum\0".as_ptr() as png_const_charp,
        );
    }

    png_write_chunk_header(png_ptr, chunk_name, length as png_uint_32);
    png_write_chunk_data(png_ptr, data, length);
    png_write_chunk_end(png_ptr);
}

/* This is the API that calls the internal function above. */
/* png_write_chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_chunk(
    png_ptr: png_structrp,
    chunk_string: png_const_bytep,
    data: png_const_bytep,
    length: usize,
) {
    png_write_complete_chunk(
        png_ptr,
        PNG_CHUNK_FROM_STRING(chunk_string),
        data,
        length,
    );
}

/* This is used below to find the size of an image to pass to png_deflate_claim,
 * so it only needs to be accurate if the size is less than 16384 bytes (the
 * point at which a lower LZ window size can be used.)
 */
/* png_image_size (static) */
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
                    cb_base = cb_base.wrapping_add(
                        (PNG_ROWBYTES(pd as usize, pw as usize).wrapping_add(1))
                            .wrapping_mul(PNG_PASS_ROWS(h, pass) as usize),
                    );
                }

                pass += 1;
            }

            return cb_base;
        } else {
            return ((*png_ptr).rowbytes.wrapping_add(1)).wrapping_mul(h as usize);
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
/* optimize_cmf (static) */
unsafe fn optimize_cmf(data: png_bytep, data_size: png_alloc_size_t) {
    /* Optimize the CMF field in the zlib stream.  The resultant zlib stream is
     * still compliant to the stream specification.
     */
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
                    z_cinfo = z_cinfo.wrapping_sub(1);

                    if !(z_cinfo > 0 && data_size <= half_z_window_size as png_alloc_size_t) {
                        break;
                    }
                }

                z_cmf = (z_cmf & 0x0f) | (z_cinfo << 4);

                *data.add(0) = z_cmf as png_byte;
                tmp = (*data.add(1) as c_uint) & 0xe0;
                tmp = tmp.wrapping_add(
                    0x1f - ((z_cmf << 8).wrapping_add(tmp)) % 0x1f,
                );
                *data.add(1) = tmp as png_byte;
            }
        }
    }
}
