//! Translation of pngrutil.c

use crate::*;

/* The minimum 'zlib' stream is assumed to be just the 2 byte header, 5 bytes
 * minimum 'deflate' stream, and the 4 byte checksum.
 */
const LZ77Min: png_uint_32 = 2 + 5 + 4;

/* Arrays to facilitate interlacing - use pass (0 - 6) as index. */

/* Start of interlace block */
static png_pass_start: [png_byte; 7] = [0, 4, 0, 2, 0, 1, 0];
/* Offset to next interlace block */
static png_pass_inc: [png_byte; 7] = [8, 8, 4, 4, 2, 2, 1];
/* Start of interlace block in the y direction */
static png_pass_ystart: [png_byte; 7] = [0, 0, 4, 0, 2, 0, 1];
/* Offset to next interlace block in the y direction */
static png_pass_yinc: [png_byte; 7] = [8, 8, 8, 4, 4, 2, 2];

#[inline]
unsafe fn png_chunk_max(png_ptr: png_const_structrp) -> png_alloc_size_t {
    unsafe { (*png_ptr).user_chunk_malloc_max }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_uint_31(
    png_ptr: png_const_structrp,
    buf: png_const_bytep,
) -> png_uint_32 {
    unsafe {
        let uval = png_get_uint_32(buf);

        if uval > PNG_UINT_31_MAX {
            png_error(png_ptr, c"PNG unsigned integer out of range".as_ptr());
        }

        uval
    }
}

/* Grab an unsigned 32-bit integer from a buffer in big-endian format. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_uint_32(buf: png_const_bytep) -> png_uint_32 {
    unsafe {
        let uval = ((*buf.add(0) as png_uint_32) << 24)
            + ((*buf.add(1) as png_uint_32) << 16)
            + ((*buf.add(2) as png_uint_32) << 8)
            + (*buf.add(3) as png_uint_32);

        uval
    }
}

/* Grab a signed 32-bit integer from a buffer in big-endian format.  The
 * data is stored in the PNG file in two's complement format and there
 * is no guarantee that a 'png_int_32' is exactly 32 bits, therefore
 * the following code does a two's complement to native conversion.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_int_32(buf: png_const_bytep) -> png_int_32 {
    unsafe {
        let mut uval = png_get_uint_32(buf);
        if (uval & 0x80000000) == 0 {
            /* non-negative */
            return uval as png_int_32;
        }

        uval = (uval ^ 0xffffffff).wrapping_add(1); /* 2's complement: -x = ~x+1 */
        if (uval & 0x80000000) == 0 {
            /* no overflow */
            return -(uval as png_int_32);
        }
        /* The following has to be safe; this function only gets called on PNG data
         * and if we get here that data is invalid.  0 is the most safe value and
         * if not then an attacker would surely just generate a PNG with 0 instead.
         */
        0
    }
}

/* Grab an unsigned 16-bit integer from a buffer in big-endian format. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_uint_16(buf: png_const_bytep) -> png_uint_16 {
    unsafe {
        /* ANSI-C requires an int value to accommodate at least 16 bits so this
         * works and allows the compiler not to worry about possible narrowing
         * on 32-bit systems.  (Pre-ANSI systems did not make integers smaller
         * than 16 bits either.)
         */
        let val: c_uint = ((*buf as c_uint) << 8) + (*buf.add(1) as c_uint);

        val as png_uint_16
    }
}

/* Read and check the PNG file signature */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_sig(png_ptr: png_structrp, info_ptr: png_inforp) {
    unsafe {
        let num_checked: usize;
        let num_to_check: usize;

        /* Exit if the user application does not expect a signature. */
        if (*png_ptr).sig_bytes >= 8 {
            return;
        }

        num_checked = (*png_ptr).sig_bytes as usize;
        num_to_check = 8 - num_checked;

        (*png_ptr).io_state = PNG_IO_READING | PNG_IO_SIGNATURE;

        /* The signature must be serialized in a single I/O call. */
        png_read_data(
            png_ptr,
            &raw mut (*info_ptr).signature[num_checked],
            num_to_check,
        );
        (*png_ptr).sig_bytes = 8;

        if png_sig_cmp(
            (*info_ptr).signature.as_mut_ptr(),
            num_checked,
            num_to_check,
        ) != 0
        {
            if num_checked < 4
                && png_sig_cmp(
                    (*info_ptr).signature.as_mut_ptr(),
                    num_checked,
                    num_to_check - 4,
                ) != 0
            {
                png_error(png_ptr, c"Not a PNG file".as_ptr());
            } else {
                png_error(png_ptr, c"PNG file corrupted by ASCII conversion".as_ptr());
            }
        }
        if num_checked < 3 {
            (*png_ptr).mode |= PNG_HAVE_PNG_SIGNATURE;
        }
    }
}

/* This function is called to verify that a chunk name is valid.
 * Do this using the bit-whacking approach from contrib/tools/pngfix.c
 *
 * Copied from libpng 1.7.
 */
unsafe fn check_chunk_name(mut name: png_uint_32) -> c_int {
    let t: png_uint_32;

    /* Remove bit 5 from all but the reserved byte; this means
     * every 8-bit unit must be in the range 65-90 to be valid.
     * So bit 5 must be zero, bit 6 must be set and bit 7 zero.
     */
    name &= !PNG_U32(32, 32, 0, 32);
    t = (name & !0x1f1f1f1fu32) ^ 0x40404040u32;

    /* Subtract 65 for each 8-bit quantity, this must not
     * overflow and each byte must then be in the range 0-25.
     */
    name = name.wrapping_sub(PNG_U32(65, 65, 65, 65));
    let mut t = t | name;

    /* Subtract 26, handling the overflow which should set the
     * top three bits of each byte.
     */
    name = name.wrapping_sub(PNG_U32(25, 25, 25, 26));
    t |= !name;

    ((t & 0xe0e0e0e0u32) == 0u32) as c_int
}

/* Read the chunk header (length + type name).
 * Put the type name into png_ptr->chunk_name, and return the length.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_chunk_header(png_ptr: png_structrp) -> png_uint_32 {
    unsafe {
        let mut buf: [png_byte; 8] = [0; 8];
        let chunk_name: png_uint_32;
        let length: png_uint_32;

        (*png_ptr).io_state = PNG_IO_READING | PNG_IO_CHUNK_HDR;

        /* Read the length and the chunk name.  png_struct::chunk_name is immediately
         * updated even if they are detectably wrong.  This aids error message
         * handling by allowing png_chunk_error to be used.
         */
        png_read_data(png_ptr, buf.as_mut_ptr(), 8);
        length = png_get_uint_31(png_ptr, buf.as_ptr());
        chunk_name = PNG_CHUNK_FROM_STRING(buf.as_ptr().add(4) as *const c_char);
        (*png_ptr).chunk_name = chunk_name;

        /* Reset the crc and run it over the chunk name. */
        png_reset_crc(png_ptr);
        png_calculate_crc(png_ptr, buf.as_ptr().add(4), 4);

        /* Sanity check the length (first by <= 0x80) and the chunk name.  An error
         * here indicates a broken stream and libpng has no recovery from this.
         */
        if buf[0] >= 0x80u8 {
            png_chunk_error(png_ptr, c"bad header (invalid length)".as_ptr());
        }

        /* Check to see if chunk name is valid. */
        if check_chunk_name(chunk_name) == 0 {
            png_chunk_error(png_ptr, c"bad header (invalid type)".as_ptr());
        }

        (*png_ptr).io_state = PNG_IO_READING | PNG_IO_CHUNK_DATA;

        length
    }
}

/* Read data, and (optionally) run it through the CRC. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_crc_read(
    png_ptr: png_structrp,
    buf: png_bytep,
    length: png_uint_32,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        png_read_data(png_ptr, buf, length as usize);
        png_calculate_crc(png_ptr, buf, length as usize);
    }
}

/* Compare the CRC stored in the PNG file with that calculated by libpng from
 * the data it has read thus far.
 */
unsafe fn png_crc_error(png_ptr: png_structrp, handle_as_ancillary: c_int) -> c_int {
    unsafe {
        let mut crc_bytes: [png_byte; 4] = [0; 4];
        let crc: png_uint_32;
        let mut need_crc: c_int = 1;

        if handle_as_ancillary != 0 || PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name) != 0 {
            if ((*png_ptr).flags & PNG_FLAG_CRC_ANCILLARY_MASK)
                == (PNG_FLAG_CRC_ANCILLARY_USE | PNG_FLAG_CRC_ANCILLARY_NOWARN)
            {
                need_crc = 0;
            }
        } else
        /* critical */
        {
            if ((*png_ptr).flags & PNG_FLAG_CRC_CRITICAL_IGNORE) != 0 {
                need_crc = 0;
            }
        }

        (*png_ptr).io_state = PNG_IO_READING | PNG_IO_CHUNK_CRC;

        /* The chunk CRC must be serialized in a single I/O call. */
        png_read_data(png_ptr, crc_bytes.as_mut_ptr(), 4);

        if need_crc != 0 {
            crc = png_get_uint_32(crc_bytes.as_ptr());
            (crc != (*png_ptr).crc) as c_int
        } else {
            0
        }
    }
}

/* Optionally skip data and then check the CRC. */
unsafe fn png_crc_finish_critical(
    png_ptr: png_structrp,
    mut skip: png_uint_32,
    mut handle_as_ancillary: c_int,
) -> c_int {
    unsafe {
        /* The size of the local buffer for inflate is a good guess as to a
         * reasonable size to use for buffering reads from the application.
         */
        while skip > 0 {
            let mut len: png_uint_32;
            let mut tmpbuf: [png_byte; PNG_INFLATE_BUF_SIZE] = [0; PNG_INFLATE_BUF_SIZE];

            len = core::mem::size_of_val(&tmpbuf) as png_uint_32;
            if len > skip {
                len = skip;
            }
            skip -= len;

            png_crc_read(png_ptr, tmpbuf.as_mut_ptr(), len);
        }

        /* If 'handle_as_ancillary' has been requested and this is a critical chunk
         * but PNG_FLAG_CRC_CRITICAL_IGNORE was set then png_read_crc did not, in
         * fact, calculate the CRC so the ANCILLARY settings should not be used
         * instead.
         */
        if handle_as_ancillary != 0 && ((*png_ptr).flags & PNG_FLAG_CRC_CRITICAL_IGNORE) != 0 {
            handle_as_ancillary = 0;
        }

        if png_crc_error(png_ptr, handle_as_ancillary) != 0 {
            /* See above for the explanation of how the flags work. */
            let cond = if handle_as_ancillary != 0
                || PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name) != 0
            {
                ((*png_ptr).flags & PNG_FLAG_CRC_ANCILLARY_NOWARN) == 0
            } else {
                ((*png_ptr).flags & PNG_FLAG_CRC_CRITICAL_USE) != 0
            };

            if cond {
                png_chunk_warning(png_ptr, c"CRC error".as_ptr());
            } else {
                png_chunk_error(png_ptr, c"CRC error".as_ptr());
            }

            return 1;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_crc_finish(png_ptr: png_structrp, skip: png_uint_32) -> c_int {
    unsafe { png_crc_finish_critical(png_ptr, skip, 0 /*critical handling*/) }
}

/* Manage the read buffer; this simply reallocates the buffer if it is not small
 * enough (or if it is not allocated).
 */
unsafe fn png_read_buffer(png_ptr: png_structrp, new_size: png_alloc_size_t) -> png_bytep {
    unsafe {
        let mut buffer: png_bytep = (*png_ptr).read_buffer;

        if new_size > png_chunk_max(png_ptr) {
            return core::ptr::null_mut();
        }

        if !buffer.is_null() && new_size > (*png_ptr).read_buffer_size {
            (*png_ptr).read_buffer = core::ptr::null_mut();
            (*png_ptr).read_buffer_size = 0;
            png_free(png_ptr, buffer as png_voidp);
            buffer = core::ptr::null_mut();
        }

        if buffer.is_null() {
            buffer = png_malloc_base(png_ptr, new_size) as png_bytep;

            if !buffer.is_null() {
                memset(buffer as *mut c_void, 0, new_size); /* just in case */
                (*png_ptr).read_buffer = buffer;
                (*png_ptr).read_buffer_size = new_size;
            }
        }

        buffer
    }
}

/* png_inflate_claim: claim the zstream for some nefarious purpose that involves
 * decompression.
 */
unsafe fn png_inflate_claim(png_ptr: png_structrp, owner: png_uint_32) -> c_int {
    unsafe {
        if (*png_ptr).zowner != 0 {
            let mut msg: [c_char; 64] = [0; 64];

            PNG_STRING_FROM_CHUNK(msg.as_mut_ptr(), (*png_ptr).zowner);
            /* So the message that results is "<chunk> using zstream"; this is an
             * internal error, but is very useful for debugging.  i18n requirements
             * are minimal.
             */
            let _ = png_safecat(
                msg.as_mut_ptr(),
                core::mem::size_of_val(&msg),
                4,
                c" using zstream".as_ptr(),
            );
            png_chunk_error(png_ptr, msg.as_ptr());
        }

        /* Implementation note: unlike 'png_deflate_claim' this internal function
         * does not take the size of the data as an argument.
         */
        {
            let mut ret: c_int; /* zlib return code */
            let mut window_bits: c_int = 0;

            if (((*png_ptr).options >> PNG_MAXIMUM_INFLATE_WINDOW) & 3) == PNG_OPTION_ON as png_uint_32 {
                window_bits = 15;
                (*png_ptr).zstream_start = 0; /* fixed window size */
            } else {
                (*png_ptr).zstream_start = 1;
            }

            /* Set this for safety, just in case the previous owner left pointers to
             * memory allocations.
             */
            (*png_ptr).zstream.next_in = core::ptr::null();
            (*png_ptr).zstream.avail_in = 0;
            (*png_ptr).zstream.next_out = core::ptr::null_mut();
            (*png_ptr).zstream.avail_out = 0;

            if ((*png_ptr).flags & PNG_FLAG_ZSTREAM_INITIALIZED) != 0 {
                ret = inflateReset2(&raw mut (*png_ptr).zstream, window_bits);
            } else {
                ret = inflateInit2(&raw mut (*png_ptr).zstream, window_bits);

                if ret == Z_OK {
                    (*png_ptr).flags |= PNG_FLAG_ZSTREAM_INITIALIZED;
                }
            }

            if ret == Z_OK {
                (*png_ptr).zowner = owner;
            } else {
                png_zstream_error(png_ptr, ret);
            }

            ret
        }
    }
}

/* Handle the start of the inflate stream if we called inflateInit2(strm,0); */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_zlib_inflate(png_ptr: png_structrp, flush: c_int) -> c_int {
    unsafe {
        if (*png_ptr).zstream_start != 0 && (*png_ptr).zstream.avail_in > 0 {
            if (*(*png_ptr).zstream.next_in >> 4) > 7 {
                (*png_ptr).zstream.msg = c"invalid window size (libpng)".as_ptr();
                return Z_DATA_ERROR;
            }

            (*png_ptr).zstream_start = 0;
        }

        inflate(&raw mut (*png_ptr).zstream, flush)
    }
}

#[inline]
unsafe fn PNG_INFLATE(pp: png_structrp, flush: c_int) -> c_int {
    unsafe { png_zlib_inflate(pp, flush) }
}

/* png_inflate now returns zlib error codes including Z_OK and Z_STREAM_END to
 * allow the caller to do multiple calls if required.
 */
unsafe fn png_inflate(
    png_ptr: png_structrp,
    owner: png_uint_32,
    finish: c_int,
    /* INPUT: */ input: png_const_bytep,
    input_size_ptr: png_uint_32p,
    /* OUTPUT: */ output: png_bytep,
    output_size_ptr: *mut png_alloc_size_t,
) -> c_int {
    unsafe {
        if (*png_ptr).zowner == owner
        /* Else not claimed */
        {
            let mut ret: c_int;
            let mut avail_out: png_alloc_size_t = *output_size_ptr;
            let mut avail_in: png_uint_32 = *input_size_ptr;

            (*png_ptr).zstream.next_in = input;
            /* avail_in and avail_out are set below from 'size' */
            (*png_ptr).zstream.avail_in = 0;
            (*png_ptr).zstream.avail_out = 0;

            /* Read directly into the output if it is available (this is set to
             * a local buffer below if output is NULL).
             */
            if !output.is_null() {
                (*png_ptr).zstream.next_out = output;
            }

            loop {
                let mut avail: uInt;
                let mut local_buffer: [Bytef; PNG_INFLATE_BUF_SIZE] = [0; PNG_INFLATE_BUF_SIZE];

                /* zlib INPUT BUFFER */
                avail_in += (*png_ptr).zstream.avail_in; /* not consumed last time */

                avail = ZLIB_IO_MAX;

                if (avail_in as png_alloc_size_t) < (avail as png_alloc_size_t) {
                    avail = avail_in; /* safe: < than ZLIB_IO_MAX */
                }

                avail_in -= avail;
                (*png_ptr).zstream.avail_in = avail;

                /* zlib OUTPUT BUFFER */
                avail_out += (*png_ptr).zstream.avail_out as png_alloc_size_t; /* not written last time */

                avail = ZLIB_IO_MAX; /* maximum zlib can process */

                if output.is_null() {
                    /* Reset the output buffer each time round if output is NULL and
                     * make available the full buffer, up to 'remaining_space'
                     */
                    (*png_ptr).zstream.next_out = local_buffer.as_mut_ptr();
                    if (core::mem::size_of_val(&local_buffer) as uInt) < avail {
                        avail = core::mem::size_of_val(&local_buffer) as uInt;
                    }
                }

                if avail_out < (avail as png_alloc_size_t) {
                    avail = avail_out as uInt; /* safe: < ZLIB_IO_MAX */
                }

                (*png_ptr).zstream.avail_out = avail;
                avail_out -= avail as png_alloc_size_t;

                /* zlib inflate call */
                ret = PNG_INFLATE(
                    png_ptr,
                    if avail_out > 0 {
                        Z_NO_FLUSH
                    } else if finish != 0 {
                        Z_FINISH
                    } else {
                        Z_SYNC_FLUSH
                    },
                );

                if ret != Z_OK {
                    break;
                }
            }

            /* For safety kill the local buffer pointer now */
            if output.is_null() {
                (*png_ptr).zstream.next_out = core::ptr::null_mut();
            }

            /* Claw back the 'size' and 'remaining_space' byte counts. */
            avail_in += (*png_ptr).zstream.avail_in;
            avail_out += (*png_ptr).zstream.avail_out as png_alloc_size_t;

            /* Update the input and output sizes; the updated values are the amount
             * consumed or written, effectively the inverse of what zlib uses.
             */
            if avail_out > 0 {
                *output_size_ptr -= avail_out;
            }

            if avail_in > 0 {
                *input_size_ptr -= avail_in;
            }

            /* Ensure png_ptr->zstream.msg is set (even in the success case!) */
            png_zstream_error(png_ptr, ret);
            ret
        } else {
            /* This is a bad internal error. */
            (*png_ptr).zstream.msg = c"zstream unclaimed".as_ptr();
            Z_STREAM_ERROR
        }
    }
}

/*
 * Decompress trailing data in a chunk.
 */
unsafe fn png_decompress_chunk(
    png_ptr: png_structrp,
    chunklength: png_uint_32,
    prefix_size: png_uint_32,
    newlength: *mut png_alloc_size_t, /* must be initialized to the maximum! */
    terminate: c_int,                 /*add a '\0' to the end of the uncompressed data*/
) -> c_int {
    unsafe {
        let mut limit: png_alloc_size_t = png_chunk_max(png_ptr);

        if limit >= prefix_size as png_alloc_size_t + (terminate != 0) as png_alloc_size_t {
            let mut ret: c_int;

            limit -= prefix_size as png_alloc_size_t + (terminate != 0) as png_alloc_size_t;

            if limit < *newlength {
                *newlength = limit;
            }

            /* Now try to claim the stream. */
            ret = png_inflate_claim(png_ptr, (*png_ptr).chunk_name);

            if ret == Z_OK {
                let mut lzsize: png_uint_32 = chunklength - prefix_size;

                ret = png_inflate(
                    png_ptr,
                    (*png_ptr).chunk_name,
                    1, /*finish*/
                    /* input: */ (*png_ptr).read_buffer.add(prefix_size as usize),
                    &raw mut lzsize,
                    /* output: */ core::ptr::null_mut(),
                    newlength,
                );

                if ret == Z_STREAM_END {
                    /* Use 'inflateReset' here, not 'inflateReset2'. */
                    if inflateReset(&raw mut (*png_ptr).zstream) == Z_OK {
                        let new_size: png_alloc_size_t = *newlength;
                        let buffer_size: png_alloc_size_t =
                            prefix_size as png_alloc_size_t + new_size + (terminate != 0) as png_alloc_size_t;
                        let mut text: png_bytep =
                            png_malloc_base(png_ptr, buffer_size) as png_bytep;

                        if !text.is_null() {
                            memset(text as *mut c_void, 0, buffer_size);

                            ret = png_inflate(
                                png_ptr,
                                (*png_ptr).chunk_name,
                                1, /*finish*/
                                (*png_ptr).read_buffer.add(prefix_size as usize),
                                &raw mut lzsize,
                                text.add(prefix_size as usize),
                                newlength,
                            );

                            if ret == Z_STREAM_END {
                                if new_size == *newlength {
                                    if terminate != 0 {
                                        *text.add(prefix_size as usize + *newlength) = 0;
                                    }

                                    if prefix_size > 0 {
                                        memcpy(
                                            text as *mut c_void,
                                            (*png_ptr).read_buffer as *const c_void,
                                            prefix_size as usize,
                                        );
                                    }

                                    {
                                        let old_ptr: png_bytep = (*png_ptr).read_buffer;

                                        (*png_ptr).read_buffer = text;
                                        (*png_ptr).read_buffer_size = buffer_size;
                                        text = old_ptr; /* freed below */
                                    }
                                } else {
                                    /* The size changed on the second read. */
                                    ret = PNG_UNEXPECTED_ZLIB_RETURN;
                                }
                            } else if ret == Z_OK {
                                ret = PNG_UNEXPECTED_ZLIB_RETURN; /* for safety */
                            }

                            /* Free the text pointer (this is the old read_buffer on
                             * success)
                             */
                            png_free(png_ptr, text as png_voidp);

                            /* This really is very benign, but it's still an error. */
                            if ret == Z_STREAM_END && chunklength - prefix_size != lzsize {
                                png_chunk_benign_error(png_ptr, c"extra compressed data".as_ptr());
                            }
                        } else {
                            /* Out of memory allocating the buffer */
                            ret = Z_MEM_ERROR;
                            png_zstream_error(png_ptr, Z_MEM_ERROR);
                        }
                    } else {
                        /* inflateReset failed, store the error message */
                        png_zstream_error(png_ptr, ret);
                        ret = PNG_UNEXPECTED_ZLIB_RETURN;
                    }
                } else if ret == Z_OK {
                    ret = PNG_UNEXPECTED_ZLIB_RETURN;
                }

                /* Release the claimed stream */
                (*png_ptr).zowner = 0;
            } else if ret == Z_STREAM_END
            /* the claim failed */
            /* impossible! */
            {
                ret = PNG_UNEXPECTED_ZLIB_RETURN;
            }

            ret
        } else {
            /* Application/configuration limits exceeded */
            png_zstream_error(png_ptr, Z_MEM_ERROR);
            Z_MEM_ERROR
        }
    }
}

/* Perform a partial read and decompress, producing 'avail_out' bytes and
 * reading from the current chunk as required.
 */
unsafe fn png_inflate_read(
    png_ptr: png_structrp,
    read_buffer: png_bytep,
    mut read_size: uInt,
    chunk_bytes: png_uint_32p,
    next_out: png_bytep,
    out_size: *mut png_alloc_size_t,
    finish: c_int,
) -> c_int {
    unsafe {
        if (*png_ptr).zowner == (*png_ptr).chunk_name {
            let mut ret: c_int;

            /* next_in and avail_in must have been initialized by the caller. */
            (*png_ptr).zstream.next_out = next_out;
            (*png_ptr).zstream.avail_out = 0; /* set in the loop */

            loop {
                if (*png_ptr).zstream.avail_in == 0 {
                    if read_size as png_uint_32 > *chunk_bytes {
                        read_size = *chunk_bytes as uInt;
                    }
                    *chunk_bytes -= read_size as png_uint_32;

                    if read_size > 0 {
                        png_crc_read(png_ptr, read_buffer, read_size as png_uint_32);
                    }

                    (*png_ptr).zstream.next_in = read_buffer;
                    (*png_ptr).zstream.avail_in = read_size;
                }

                if (*png_ptr).zstream.avail_out == 0 {
                    let mut avail: uInt = ZLIB_IO_MAX;
                    if (avail as png_alloc_size_t) > *out_size {
                        avail = *out_size as uInt;
                    }
                    *out_size -= avail as png_alloc_size_t;

                    (*png_ptr).zstream.avail_out = avail;
                }

                /* Use Z_SYNC_FLUSH when there is no more chunk data. */
                ret = PNG_INFLATE(
                    png_ptr,
                    if *chunk_bytes > 0 {
                        Z_NO_FLUSH
                    } else if finish != 0 {
                        Z_FINISH
                    } else {
                        Z_SYNC_FLUSH
                    },
                );

                if !(ret == Z_OK && (*out_size > 0 || (*png_ptr).zstream.avail_out > 0)) {
                    break;
                }
            }

            *out_size += (*png_ptr).zstream.avail_out as png_alloc_size_t;
            (*png_ptr).zstream.avail_out = 0; /* Should not be required, but is safe */

            /* Ensure the error message pointer is always set: */
            png_zstream_error(png_ptr, ret);
            ret
        } else {
            (*png_ptr).zstream.msg = c"zstream unclaimed".as_ptr();
            Z_STREAM_ERROR
        }
    }
}

/* CHUNK HANDLING */
/* Read and check the IDHR chunk */
unsafe extern "C-unwind" fn png_handle_IHDR(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let mut buf: [png_byte; 13] = [0; 13];
        let width: png_uint_32;
        let height: png_uint_32;
        let bit_depth: c_int;
        let color_type: c_int;
        let compression_type: c_int;
        let filter_type: c_int;
        let interlace_type: c_int;

        /* Length and position are checked by the caller. */

        (*png_ptr).mode |= PNG_HAVE_IHDR;

        png_crc_read(png_ptr, buf.as_mut_ptr(), 13);
        png_crc_finish(png_ptr, 0);

        width = png_get_uint_31(png_ptr, buf.as_ptr());
        height = png_get_uint_31(png_ptr, buf.as_ptr().add(4));
        bit_depth = buf[8] as c_int;
        color_type = buf[9] as c_int;
        compression_type = buf[10] as c_int;
        filter_type = buf[11] as c_int;
        interlace_type = buf[12] as c_int;

        /* Set internal variables */
        (*png_ptr).width = width;
        (*png_ptr).height = height;
        (*png_ptr).bit_depth = bit_depth as png_byte;
        (*png_ptr).interlaced = interlace_type as png_byte;
        (*png_ptr).color_type = color_type as png_byte;
        (*png_ptr).filter_type = filter_type as png_byte;
        (*png_ptr).compression_type = compression_type as png_byte;

        /* Find number of channels */
        match (*png_ptr).color_type as c_int {
            /* default and GRAY and PALETTE */
            PNG_COLOR_TYPE_RGB => {
                (*png_ptr).channels = 3;
            }

            PNG_COLOR_TYPE_GRAY_ALPHA => {
                (*png_ptr).channels = 2;
            }

            PNG_COLOR_TYPE_RGB_ALPHA => {
                (*png_ptr).channels = 4;
            }

            /* PNG_COLOR_TYPE_GRAY, PNG_COLOR_TYPE_PALETTE, default */
            _ => {
                (*png_ptr).channels = 1;
            }
        }

        /* Set up other useful info */
        (*png_ptr).pixel_depth = ((*png_ptr).bit_depth as c_int * (*png_ptr).channels as c_int) as png_byte;
        (*png_ptr).rowbytes =
            PNG_ROWBYTES((*png_ptr).pixel_depth as usize, (*png_ptr).width as usize);

        /* Rely on png_set_IHDR to completely validate the data and call png_error if
         * it's wrong.
         */
        png_set_IHDR(
            png_ptr,
            info_ptr,
            width,
            height,
            bit_depth,
            color_type,
            interlace_type,
            compression_type,
            filter_type,
        );

        PNG_UNUSED(length);
        handled_ok
    }
}

/* Read and check the palette */
unsafe extern "C-unwind" fn png_handle_PLTE(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let mut errmsg: png_const_charp = core::ptr::null();

        /* 1.6.47: consistency. */
        if ((*png_ptr).mode & PNG_HAVE_PLTE) != 0 {
            errmsg = c"duplicate".as_ptr();
        } else if ((*png_ptr).mode & PNG_HAVE_IDAT) != 0 {
            errmsg = c"out of place".as_ptr();
        } else if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0 {
            errmsg = c"ignored in grayscale PNG".as_ptr();
        } else if length > 3 * PNG_MAX_PALETTE_LENGTH as png_uint_32 || (length % 3) != 0 {
            errmsg = c"invalid".as_ptr();
        }
        /* This drops PLTE in favour of tRNS or bKGD. */
        else if (*png_ptr).color_type as c_int != PNG_COLOR_TYPE_PALETTE
            && (png_file_has_chunk(png_ptr, PNG_INDEX_tRNS)
                || png_file_has_chunk(png_ptr, PNG_INDEX_bKGD))
        {
            errmsg = c"out of place".as_ptr();
        } else {
            /* If the palette has 256 or fewer entries but is too large for the bit
             * depth we don't issue an error to preserve the behavior of previous
             * libpng versions.
             */
            let max_palette_length: c_uint = if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
            {
                1u32 << (*png_ptr).bit_depth
            } else {
                PNG_MAX_PALETTE_LENGTH as c_uint
            };

            /* The cast is safe because 'length' is less than
             * 3*PNG_MAX_PALETTE_LENGTH
             */
            let num: c_uint = if length > 3u32 * max_palette_length {
                max_palette_length
            } else {
                length / 3u32
            };

            let mut i: c_uint;
            let mut j: c_uint;
            let mut buf: [png_byte; 3 * PNG_MAX_PALETTE_LENGTH as usize] =
                [0; 3 * PNG_MAX_PALETTE_LENGTH as usize];
            let mut palette: [png_color; PNG_MAX_PALETTE_LENGTH as usize] =
                [png_color::default(); PNG_MAX_PALETTE_LENGTH as usize];

            /* Read the chunk into the buffer then read to the end of the chunk. */
            png_crc_read(png_ptr, buf.as_mut_ptr(), num * 3u32);
            png_crc_finish_critical(
                png_ptr,
                length - 3u32 * num,
                /* Handle as ancillary if PLTE is optional: */
                ((*png_ptr).color_type as c_int != PNG_COLOR_TYPE_PALETTE) as c_int,
            );

            i = 0u32;
            j = 0u32;
            while i < num {
                palette[i as usize].red = buf[j as usize];
                j += 1;
                palette[i as usize].green = buf[j as usize];
                j += 1;
                palette[i as usize].blue = buf[j as usize];
                j += 1;
                i += 1;
            }

            /* A valid PLTE chunk has been read */
            (*png_ptr).mode |= PNG_HAVE_PLTE;

            png_set_PLTE(png_ptr, info_ptr, palette.as_mut_ptr(), num as c_int);
            return handled_ok;
        }

        /* Here on error: errmsg is non NULL. */
        if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            png_crc_finish(png_ptr, length);
            png_chunk_error(png_ptr, errmsg);
        } else
        /* not critical to this image */
        {
            png_crc_finish_critical(png_ptr, length, 1 /*handle as ancillary*/);
            png_chunk_benign_error(png_ptr, errmsg);
        }

        if !errmsg.is_null() {
            handled_error
        } else {
            handled_error
        }
    }
}

/* On read the IDAT chunk is always handled specially. */

unsafe extern "C-unwind" fn png_handle_IEND(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        (*png_ptr).mode |= PNG_AFTER_IDAT | PNG_HAVE_IEND;

        if length != 0 {
            png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
        }

        png_crc_finish_critical(png_ptr, length, 1 /*handle as ancillary*/);

        PNG_UNUSED(info_ptr);
        handled_ok
    }
}

unsafe extern "C-unwind" fn png_handle_gAMA(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let ugamma: png_uint_32;
        let mut buf: [png_byte; 4] = [0; 4];

        png_crc_read(png_ptr, buf.as_mut_ptr(), 4);

        if png_crc_finish(png_ptr, 0) != 0 {
            return handled_error;
        }

        ugamma = png_get_uint_32(buf.as_ptr());

        if ugamma > PNG_UINT_31_MAX {
            png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
            return handled_error;
        }

        png_set_gAMA_fixed(png_ptr, info_ptr, ugamma as png_fixed_point /*SAFE*/);

        /* PNGv3: chunk precedence for gamma is cICP, [iCCP], sRGB, gAMA. */
        if (*png_ptr).chunk_gamma == 0 {
            (*png_ptr).chunk_gamma = ugamma as png_fixed_point /*SAFE*/;
        }

        PNG_UNUSED(length);
        handled_ok
    }
}

unsafe extern "C-unwind" fn png_handle_sBIT(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let truelen: c_uint;
        let mut i: c_uint;
        let sample_depth: png_byte;
        let mut buf: [png_byte; 4] = [0; 4];

        if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            truelen = 3;
            sample_depth = 8;
        } else {
            truelen = (*png_ptr).channels as c_uint;
            sample_depth = (*png_ptr).bit_depth;
        }

        if length != truelen {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"bad length".as_ptr());
            return handled_error;
        }

        buf[0] = sample_depth;
        buf[1] = sample_depth;
        buf[2] = sample_depth;
        buf[3] = sample_depth;
        png_crc_read(png_ptr, buf.as_mut_ptr(), truelen);

        if png_crc_finish(png_ptr, 0) != 0 {
            return handled_error;
        }

        i = 0;
        while i < truelen {
            if buf[i as usize] == 0 || buf[i as usize] > sample_depth {
                png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
                return handled_error;
            }
            i += 1;
        }

        if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) != 0 {
            (*png_ptr).sig_bit.red = buf[0];
            (*png_ptr).sig_bit.green = buf[1];
            (*png_ptr).sig_bit.blue = buf[2];
            (*png_ptr).sig_bit.alpha = buf[3];
        } else
        /* grayscale */
        {
            (*png_ptr).sig_bit.gray = buf[0];
            (*png_ptr).sig_bit.red = buf[0];
            (*png_ptr).sig_bit.green = buf[0];
            (*png_ptr).sig_bit.blue = buf[0];
            (*png_ptr).sig_bit.alpha = buf[1];
        }

        png_set_sBIT(png_ptr, info_ptr, &raw mut (*png_ptr).sig_bit);
        handled_ok
    }
}

unsafe fn png_get_int_32_checked(buf: png_const_bytep, error: *mut c_int) -> png_int_32 {
    unsafe {
        let mut uval = png_get_uint_32(buf);
        if (uval & 0x80000000) == 0 {
            /* non-negative */
            return uval as png_int_32;
        }

        uval = (uval ^ 0xffffffff).wrapping_add(1); /* 2's complement: -x = ~x+1 */
        if (uval & 0x80000000) == 0 {
            /* no overflow */
            return -(uval as png_int_32);
        }

        /* This version of png_get_int_32 has a way of returning the error. */
        *error = 1;
        0 /* Safe */
    }
}

unsafe extern "C-unwind" fn png_handle_cHRM(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let mut error: c_int = 0;
        let mut xy: png_xy = png_xy::default();
        let mut buf: [png_byte; 32] = [0; 32];

        png_crc_read(png_ptr, buf.as_mut_ptr(), 32);

        if png_crc_finish(png_ptr, 0) != 0 {
            return handled_error;
        }

        xy.whitex = png_get_int_32_checked(buf.as_ptr().add(0), &raw mut error);
        xy.whitey = png_get_int_32_checked(buf.as_ptr().add(4), &raw mut error);
        xy.redx = png_get_int_32_checked(buf.as_ptr().add(8), &raw mut error);
        xy.redy = png_get_int_32_checked(buf.as_ptr().add(12), &raw mut error);
        xy.greenx = png_get_int_32_checked(buf.as_ptr().add(16), &raw mut error);
        xy.greeny = png_get_int_32_checked(buf.as_ptr().add(20), &raw mut error);
        xy.bluex = png_get_int_32_checked(buf.as_ptr().add(24), &raw mut error);
        xy.bluey = png_get_int_32_checked(buf.as_ptr().add(28), &raw mut error);

        if error != 0 {
            png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
            return handled_error;
        }

        png_set_cHRM_fixed(
            png_ptr, info_ptr, xy.whitex, xy.whitey, xy.redx, xy.redy, xy.greenx, xy.greeny,
            xy.bluex, xy.bluey,
        );

        /* We only use 'chromaticities' for RGB to gray */
        if !png_file_has_chunk(png_ptr, PNG_INDEX_mDCV) {
            (*png_ptr).chromaticities = xy;
        }

        PNG_UNUSED(length);
        handled_ok
    }
}

unsafe extern "C-unwind" fn png_handle_sRGB(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let mut intent: png_byte = 0;

        png_crc_read(png_ptr, &raw mut intent, 1);

        if png_crc_finish(png_ptr, 0) != 0 {
            return handled_error;
        }

        /* This checks the range of the "rendering intent". */
        if intent > 3
        /*PNGv3 spec*/
        {
            png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
            return handled_error;
        }

        png_set_sRGB(png_ptr, info_ptr, intent as c_int);

        /* PNGv3: chunk precedence for gamma is cICP, [iCCP], sRGB, gAMA. */
        if !png_file_has_chunk(png_ptr, PNG_INDEX_cICP) || (*png_ptr).chunk_gamma == 0 {
            (*png_ptr).chunk_gamma = PNG_GAMMA_sRGB_INVERSE;
        }

        PNG_UNUSED(length);
        handled_ok
    }
}

unsafe extern "C-unwind" fn png_handle_iCCP(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    mut length: png_uint_32,
) -> png_handle_result_code
/* Note: this does not properly handle profiles that are > 64K under DOS */
{
    unsafe {
        let mut errmsg: png_const_charp = core::ptr::null(); /* error message output, or no error */
        let mut finished: c_int = 0; /* crc checked */

        /* PNGv3: allow PNG files with both sRGB and iCCP. */
        {
            let mut read_length: uInt;
            let keyword_length: uInt;
            let mut keyword: [c_char; 81] = [0; 81];

            /* Find the keyword; the keyword plus separator and compression method
             * bytes can be at most 81 characters long.
             */
            read_length = 81; /* maximum */
            if read_length as png_uint_32 > length {
                read_length = length as uInt /*SAFE*/;
            }

            png_crc_read(png_ptr, keyword.as_mut_ptr() as png_bytep, read_length as png_uint_32);
            length -= read_length as png_uint_32;

            if length < LZ77Min {
                png_crc_finish(png_ptr, length);
                png_chunk_benign_error(png_ptr, c"too short".as_ptr());
                return handled_error;
            }

            let mut kl: uInt = 0;
            while kl < 80 && kl < read_length && keyword[kl as usize] != 0 {
                kl += 1;
            }
            keyword_length = kl;

            /* TODO: make the keyword checking common */
            if keyword_length >= 1 && keyword_length <= 79 {
                /* We only understand '0' compression - deflate. */
                if keyword_length + 1 < read_length
                    && keyword[(keyword_length + 1) as usize] == PNG_COMPRESSION_TYPE_BASE as c_char
                {
                    read_length -= keyword_length + 2;

                    if png_inflate_claim(png_ptr, png_iCCP) == Z_OK {
                        let mut profile_header: [Bytef; 132] = [0; 132];
                        let mut local_buffer: [Bytef; PNG_INFLATE_BUF_SIZE] =
                            [0; PNG_INFLATE_BUF_SIZE];
                        let mut size: png_alloc_size_t = core::mem::size_of_val(&profile_header);

                        (*png_ptr).zstream.next_in =
                            (keyword.as_mut_ptr() as *mut Bytef).add((keyword_length + 2) as usize);
                        (*png_ptr).zstream.avail_in = read_length;
                        let _ = png_inflate_read(
                            png_ptr,
                            local_buffer.as_mut_ptr(),
                            core::mem::size_of_val(&local_buffer) as uInt,
                            &raw mut length,
                            profile_header.as_mut_ptr(),
                            &raw mut size,
                            0, /*finish: don't, because the output is too small*/
                        );

                        if size == 0 {
                            /* We have the ICC profile header; do the basic header checks. */
                            let profile_length: png_uint_32 =
                                png_get_uint_32(profile_header.as_ptr());

                            if png_icc_check_length(
                                png_ptr,
                                keyword.as_ptr(),
                                profile_length,
                            ) != 0
                            {
                                /* The length is apparently ok. */
                                if png_icc_check_header(
                                    png_ptr,
                                    keyword.as_ptr(),
                                    profile_length,
                                    profile_header.as_ptr(),
                                    (*png_ptr).color_type as c_int,
                                ) != 0
                                {
                                    /* Now read the tag table. */
                                    let tag_count: png_uint_32 =
                                        png_get_uint_32(profile_header.as_ptr().add(128));
                                    let profile: png_bytep =
                                        png_read_buffer(png_ptr, profile_length as png_alloc_size_t);

                                    if !profile.is_null() {
                                        memcpy(
                                            profile as *mut c_void,
                                            profile_header.as_ptr() as *const c_void,
                                            core::mem::size_of_val(&profile_header),
                                        );

                                        size = (12 * tag_count) as png_alloc_size_t;

                                        let _ = png_inflate_read(
                                            png_ptr,
                                            local_buffer.as_mut_ptr(),
                                            core::mem::size_of_val(&local_buffer) as uInt,
                                            &raw mut length,
                                            profile.add(core::mem::size_of_val(&profile_header)),
                                            &raw mut size,
                                            0,
                                        );

                                        /* Still expect a buffer error. */
                                        if size == 0 {
                                            if png_icc_check_tag_table(
                                                png_ptr,
                                                keyword.as_ptr(),
                                                profile_length,
                                                profile,
                                            ) != 0
                                            {
                                                /* The profile has been validated. */
                                                size = (profile_length as usize
                                                    - core::mem::size_of_val(&profile_header)
                                                    - (12 * tag_count) as usize)
                                                    as png_alloc_size_t;

                                                let _ = png_inflate_read(
                                                    png_ptr,
                                                    local_buffer.as_mut_ptr(),
                                                    core::mem::size_of_val(&local_buffer) as uInt,
                                                    &raw mut length,
                                                    profile
                                                        .add(core::mem::size_of_val(&profile_header))
                                                        .add((12 * tag_count) as usize),
                                                    &raw mut size,
                                                    1, /*finish*/
                                                );

                                                if length > 0
                                                    && ((*png_ptr).flags
                                                        & PNG_FLAG_BENIGN_ERRORS_WARN)
                                                        == 0
                                                {
                                                    errmsg = c"extra compressed data".as_ptr();
                                                }
                                                /* But otherwise allow extra data: */
                                                else if size == 0 {
                                                    if length > 0 {
                                                        /* This can be handled completely. */
                                                        png_chunk_warning(
                                                            png_ptr,
                                                            c"extra compressed data".as_ptr(),
                                                        );
                                                    }

                                                    png_crc_finish(png_ptr, length);
                                                    finished = 1;

                                                    /* Steal the profile for info_ptr. */
                                                    if !info_ptr.is_null() {
                                                        png_free_data(
                                                            png_ptr,
                                                            info_ptr,
                                                            PNG_FREE_ICCP,
                                                            0,
                                                        );

                                                        (*info_ptr).iccp_name = png_malloc_base(
                                                            png_ptr,
                                                            (keyword_length + 1) as png_alloc_size_t,
                                                        )
                                                            as *mut c_char;
                                                        if !(*info_ptr).iccp_name.is_null() {
                                                            memcpy(
                                                                (*info_ptr).iccp_name as *mut c_void,
                                                                keyword.as_ptr() as *const c_void,
                                                                (keyword_length + 1) as usize,
                                                            );
                                                            (*info_ptr).iccp_proflen =
                                                                profile_length;
                                                            (*info_ptr).iccp_profile = profile;
                                                            (*png_ptr).read_buffer =
                                                                core::ptr::null_mut(); /*steal*/
                                                            (*info_ptr).free_me |= PNG_FREE_ICCP;
                                                            (*info_ptr).valid |= PNG_INFO_iCCP;
                                                        } else {
                                                            errmsg = c"out of memory".as_ptr();
                                                        }
                                                    }

                                                    /* else the profile remains in the read
                                                     * buffer which gets reused.
                                                     */

                                                    if errmsg.is_null() {
                                                        (*png_ptr).zowner = 0;
                                                        return handled_ok;
                                                    }
                                                }
                                                if errmsg.is_null() {
                                                    errmsg = (*png_ptr).zstream.msg;
                                                }
                                            }
                                            /* else png_icc_check_tag_table output an error */
                                        } else
                                        /* profile truncated */
                                        {
                                            errmsg = (*png_ptr).zstream.msg;
                                        }
                                    } else {
                                        errmsg = c"out of memory".as_ptr();
                                    }
                                }

                                /* else png_icc_check_header output an error */
                            }

                            /* else png_icc_check_length output an error */
                        } else
                        /* profile truncated */
                        {
                            errmsg = (*png_ptr).zstream.msg;
                        }

                        /* Release the stream */
                        (*png_ptr).zowner = 0;
                    } else
                    /* png_inflate_claim failed */
                    {
                        errmsg = (*png_ptr).zstream.msg;
                    }
                } else {
                    errmsg = c"bad compression method".as_ptr(); /* or missing */
                }
            } else {
                errmsg = c"bad keyword".as_ptr();
            }
        }

        /* Failure: the reason is in 'errmsg' */
        if finished == 0 {
            png_crc_finish(png_ptr, length);
        }

        if !errmsg.is_null()
        /* else already output */
        {
            png_chunk_benign_error(png_ptr, errmsg);
        }

        handled_error
    }
}

unsafe extern "C-unwind" fn png_handle_sPLT(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code
/* Note: this does not properly handle chunks that are > 64K under DOS */
{
    unsafe {
        let buffer: png_bytep;
        let mut entry_start: png_bytep;
        let mut new_palette: png_sPLT_t = core::mem::zeroed();
        let mut pp: png_sPLT_entryp;
        let data_length: png_uint_32;
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

            (*png_ptr).user_chunk_cache_max -= 1;
            if (*png_ptr).user_chunk_cache_max == 1 {
                png_warning(png_ptr, c"No space in chunk cache for sPLT".as_ptr());
                png_crc_finish(png_ptr, length);
                return handled_error;
            }
        }

        buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);
        if buffer.is_null() {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
            return handled_error;
        }

        /* WARNING: this may break if size_t is less than 32 bits. */
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
            png_warning(png_ptr, c"malformed sPLT chunk".as_ptr());
            return handled_error;
        }

        new_palette.depth = *entry_start;
        entry_start = entry_start.add(1);
        entry_size = if new_palette.depth == 8 { 6 } else { 10 };
        /* This must fit in a png_uint_32. */
        data_length = length - (entry_start.offset_from(buffer) as png_uint_32);

        /* Integrity-check the data length */
        if (data_length % entry_size as c_uint) != 0 {
            png_warning(png_ptr, c"sPLT chunk has bad length".as_ptr());
            return handled_error;
        }

        dl = (data_length / entry_size as c_uint) as png_uint_32;
        max_dl = PNG_SIZE_MAX / core::mem::size_of::<png_sPLT_entry>();

        if dl as usize > max_dl {
            png_warning(png_ptr, c"sPLT chunk too long".as_ptr());
            return handled_error;
        }

        new_palette.nentries = (data_length / entry_size as c_uint) as png_int_32;

        new_palette.entries = png_malloc_warn(
            png_ptr,
            new_palette.nentries as png_alloc_size_t * core::mem::size_of::<png_sPLT_entry>(),
        ) as png_sPLT_entryp;

        if new_palette.entries.is_null() {
            png_warning(png_ptr, c"sPLT chunk requires too much memory".as_ptr());
            return handled_error;
        }

        i = 0;
        while i < new_palette.nentries {
            pp = new_palette.entries.add(i as usize);

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

        png_set_sPLT(png_ptr, info_ptr, &raw mut new_palette, 1);

        png_free(png_ptr, new_palette.entries as png_voidp);
        handled_ok
    }
}

unsafe extern "C-unwind" fn png_handle_tRNS(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let mut readbuf: [png_byte; PNG_MAX_PALETTE_LENGTH as usize] =
            [0; PNG_MAX_PALETTE_LENGTH as usize];

        if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY {
            let mut buf: [png_byte; 2] = [0; 2];

            if length != 2 {
                png_crc_finish(png_ptr, length);
                png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
                return handled_error;
            }

            png_crc_read(png_ptr, buf.as_mut_ptr(), 2);
            (*png_ptr).num_trans = 1;
            (*png_ptr).trans_color.gray = png_get_uint_16(buf.as_ptr());
        } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB {
            let mut buf: [png_byte; 6] = [0; 6];

            if length != 6 {
                png_crc_finish(png_ptr, length);
                png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
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
                png_chunk_benign_error(png_ptr, c"out of place".as_ptr());
                return handled_error;
            }

            if length > (*png_ptr).num_palette as c_uint
                || length > PNG_MAX_PALETTE_LENGTH as c_uint
                || length == 0
            {
                png_crc_finish(png_ptr, length);
                png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
                return handled_error;
            }

            png_crc_read(png_ptr, readbuf.as_mut_ptr(), length);
            (*png_ptr).num_trans = length as png_uint_16;
        } else {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"invalid with alpha channel".as_ptr());
            return handled_error;
        }

        if png_crc_finish(png_ptr, 0) != 0 {
            (*png_ptr).num_trans = 0;
            return handled_error;
        }

        png_set_tRNS(
            png_ptr,
            info_ptr,
            readbuf.as_mut_ptr(),
            (*png_ptr).num_trans as c_int,
            &raw mut (*png_ptr).trans_color,
        );
        handled_ok
    }
}

unsafe extern "C-unwind" fn png_handle_bKGD(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let truelen: c_uint;
        let mut buf: [png_byte; 6] = [0; 6];
        let mut background: png_color_16 = png_color_16::default();

        if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            if ((*png_ptr).mode & PNG_HAVE_PLTE) == 0 {
                png_crc_finish(png_ptr, length);
                png_chunk_benign_error(png_ptr, c"out of place".as_ptr());
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
            png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
            return handled_error;
        }

        png_crc_read(png_ptr, buf.as_mut_ptr(), truelen);

        if png_crc_finish(png_ptr, 0) != 0 {
            return handled_error;
        }

        /* We convert the index value into RGB components. */
        if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
            background.index = buf[0];

            if !info_ptr.is_null() && (*info_ptr).num_palette != 0 {
                if buf[0] as c_uint >= (*info_ptr).num_palette as c_uint {
                    png_chunk_benign_error(png_ptr, c"invalid index".as_ptr());
                    return handled_error;
                }

                background.red = (*(*png_ptr).palette.add(buf[0] as usize)).red as png_uint_16;
                background.green = (*(*png_ptr).palette.add(buf[0] as usize)).green as png_uint_16;
                background.blue = (*(*png_ptr).palette.add(buf[0] as usize)).blue as png_uint_16;
            } else {
                background.red = 0;
                background.green = 0;
                background.blue = 0;
            }

            background.gray = 0;
        } else if ((*png_ptr).color_type as c_int & PNG_COLOR_MASK_COLOR) == 0
        /* GRAY */
        {
            if (*png_ptr).bit_depth <= 8 {
                if buf[0] != 0 || buf[1] as c_uint >= (1u32 << (*png_ptr).bit_depth) {
                    png_chunk_benign_error(png_ptr, c"invalid gray level".as_ptr());
                    return handled_error;
                }
            }

            background.index = 0;
            let g = png_get_uint_16(buf.as_ptr());
            background.red = g;
            background.green = g;
            background.blue = g;
            background.gray = g;
        } else {
            if (*png_ptr).bit_depth <= 8 {
                if buf[0] != 0 || buf[2] != 0 || buf[4] != 0 {
                    png_chunk_benign_error(png_ptr, c"invalid color".as_ptr());
                    return handled_error;
                }
            }

            background.index = 0;
            background.red = png_get_uint_16(buf.as_ptr());
            background.green = png_get_uint_16(buf.as_ptr().add(2));
            background.blue = png_get_uint_16(buf.as_ptr().add(4));
            background.gray = 0;
        }

        png_set_bKGD(png_ptr, info_ptr, &raw mut background);
        handled_ok
    }
}

unsafe extern "C-unwind" fn png_handle_cICP(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
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

        /* PNGv3: chunk precedence for gamma. */
        /* TODO: set png_struct::chunk_gamma when possible */

        PNG_UNUSED(length);
        handled_ok
    }
}

unsafe extern "C-unwind" fn png_handle_cLLI(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
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
        PNG_UNUSED(length);
        handled_ok
    }
}

unsafe extern "C-unwind" fn png_handle_mDCV(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let mut chromaticities: png_xy = png_xy::default();
        let mut buf: [png_byte; 24] = [0; 24];

        png_crc_read(png_ptr, buf.as_mut_ptr(), 24);

        if png_crc_finish(png_ptr, 0) != 0 {
            return handled_error;
        }

        /* The error checking happens here. */
        chromaticities.redx = (png_get_uint_16(buf.as_ptr().add(0)) as png_fixed_point) << 1; /* red x */
        chromaticities.redy = (png_get_uint_16(buf.as_ptr().add(2)) as png_fixed_point) << 1; /* red y */
        chromaticities.greenx = (png_get_uint_16(buf.as_ptr().add(4)) as png_fixed_point) << 1; /* green x */
        chromaticities.greeny = (png_get_uint_16(buf.as_ptr().add(6)) as png_fixed_point) << 1; /* green y */
        chromaticities.bluex = (png_get_uint_16(buf.as_ptr().add(8)) as png_fixed_point) << 1; /* blue x */
        chromaticities.bluey = (png_get_uint_16(buf.as_ptr().add(10)) as png_fixed_point) << 1; /* blue y */
        chromaticities.whitex = (png_get_uint_16(buf.as_ptr().add(12)) as png_fixed_point) << 1; /* white x */
        chromaticities.whitey = (png_get_uint_16(buf.as_ptr().add(14)) as png_fixed_point) << 1; /* white y */

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
            png_get_uint_32(buf.as_ptr().add(16)), /* peak luminance */
            png_get_uint_32(buf.as_ptr().add(20)),
        ); /* minimum perceivable luminance */

        /* We only use 'chromaticities' for RGB to gray */
        (*png_ptr).chromaticities = chromaticities;

        PNG_UNUSED(length);
        handled_ok
    }
}

unsafe extern "C-unwind" fn png_handle_eXIf(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let buffer: png_bytep;

        buffer = png_read_buffer(png_ptr, length as png_alloc_size_t);

        if buffer.is_null() {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
            return handled_error;
        }

        png_crc_read(png_ptr, buffer, length);

        if png_crc_finish(png_ptr, 0) != 0 {
            return handled_error;
        }

        /* PNGv3: the first 4 bytes should be checked. */
        {
            let header: png_uint_32 = png_get_uint_32(buffer);

            /* These numbers are copied from the PNGv3 spec: */
            if header != 0x49492A00 && header != 0x4D4D002A {
                png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
                return handled_error;
            }
        }

        png_set_eXIf_1(png_ptr, info_ptr, length, buffer);
        handled_ok
    }
}

unsafe extern "C-unwind" fn png_handle_hIST(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let num: c_uint;
        let mut i: c_uint;
        let mut readbuf: [png_uint_16; PNG_MAX_PALETTE_LENGTH as usize] =
            [0; PNG_MAX_PALETTE_LENGTH as usize];

        num = length / 2;

        if length != num * 2
            || num != (*png_ptr).num_palette as c_uint
            || num > PNG_MAX_PALETTE_LENGTH as c_uint
        {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
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

        png_set_hIST(png_ptr, info_ptr, readbuf.as_mut_ptr());
        handled_ok
    }
}

unsafe extern "C-unwind" fn png_handle_pHYs(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
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
        PNG_UNUSED(length);
        handled_ok
    }
}

unsafe extern "C-unwind" fn png_handle_oFFs(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
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
        PNG_UNUSED(length);
        handled_ok
    }
}

/* Read the pCAL chunk (described in the PNG Extensions document) */
unsafe extern "C-unwind" fn png_handle_pCAL(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let buffer: png_bytep;
        let mut buf: png_bytep;
        let endptr: png_bytep;
        let X0: png_int_32;
        let X1: png_int_32;
        let type_: png_byte;
        let nparams: png_byte;
        let units: *mut png_byte;
        let params: png_charpp;
        let mut i: c_int;

        buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

        if buffer.is_null() {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
            return handled_error;
        }

        png_crc_read(png_ptr, buffer, length);

        if png_crc_finish(png_ptr, 0) != 0 {
            return handled_error;
        }

        *buffer.add(length as usize) = 0; /* Null terminate the last string */

        buf = buffer;
        while *buf != 0 {
            /* Empty loop */
            buf = buf.add(1);
        }

        endptr = buffer.add(length as usize);

        /* We need to have at least 12 bytes after the purpose string. */
        if endptr.offset_from(buf) <= 12 {
            png_chunk_benign_error(png_ptr, c"invalid".as_ptr());
            return handled_error;
        }

        X0 = png_get_int_32(buf.add(1));
        X1 = png_get_int_32(buf.add(5));
        type_ = *buf.add(9);
        nparams = *buf.add(10);
        units = buf.add(11);

        /* Check that we have the right number of parameters for known
         * equation types.
         */
        if (type_ as c_int == PNG_EQUATION_LINEAR && nparams != 2)
            || (type_ as c_int == PNG_EQUATION_BASE_E && nparams != 3)
            || (type_ as c_int == PNG_EQUATION_ARBITRARY && nparams != 3)
            || (type_ as c_int == PNG_EQUATION_HYPERBOLIC && nparams != 4)
        {
            png_chunk_benign_error(png_ptr, c"invalid parameter count".as_ptr());
            return handled_error;
        } else if type_ as c_int >= PNG_EQUATION_LAST {
            png_chunk_benign_error(png_ptr, c"unrecognized equation type".as_ptr());
        }

        buf = units;
        while *buf != 0 {
            /* Empty loop to move past the units string. */
            buf = buf.add(1);
        }

        params = png_malloc_warn(
            png_ptr,
            nparams as png_alloc_size_t * core::mem::size_of::<png_charp>(),
        ) as png_charpp;

        if params.is_null() {
            png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
            return handled_error;
        }

        /* Get pointers to the start of each parameter string. */
        i = 0;
        while i < nparams as c_int {
            buf = buf.add(1); /* Skip the null string terminator from previous parameter. */

            *params.add(i as usize) = buf as png_charp;
            while buf <= endptr && *buf != 0 {
                /* Empty loop to move past each parameter string */
                buf = buf.add(1);
            }

            /* Make sure we haven't run out of data yet */
            if buf > endptr {
                png_free(png_ptr, params as png_voidp);
                png_chunk_benign_error(png_ptr, c"invalid data".as_ptr());
                return handled_error;
            }
            i += 1;
        }

        png_set_pCAL(
            png_ptr,
            info_ptr,
            buffer as png_charp,
            X0,
            X1,
            type_ as c_int,
            nparams as c_int,
            units as png_charp,
            params,
        );

        png_free(png_ptr, params as png_voidp);
        handled_ok
    }
}

/* Read the sCAL chunk */
unsafe extern "C-unwind" fn png_handle_sCAL(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let buffer: png_bytep;
        let mut i: usize;
        let mut state: c_int;

        buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

        if buffer.is_null() {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
            return handled_error;
        }

        png_crc_read(png_ptr, buffer, length);
        *buffer.add(length as usize) = 0; /* Null terminate the last string */

        if png_crc_finish(png_ptr, 0) != 0 {
            return handled_error;
        }

        /* Validate the unit. */
        if *buffer.add(0) != 1 && *buffer.add(0) != 2 {
            png_chunk_benign_error(png_ptr, c"invalid unit".as_ptr());
            return handled_error;
        }

        /* Validate the ASCII numbers. */
        i = 1;
        state = 0;

        if png_check_fp_number(buffer as png_const_charp, length as usize, &raw mut state, &raw mut i)
            == 0
            || i >= length as usize
            || {
                let b = *buffer.add(i);
                i += 1;
                b != 0
            }
        {
            png_chunk_benign_error(png_ptr, c"bad width format".as_ptr());
        } else if PNG_FP_IS_POSITIVE(state) == false {
            png_chunk_benign_error(png_ptr, c"non-positive width".as_ptr());
        } else {
            let heighti: usize = i;

            state = 0;
            if png_check_fp_number(
                buffer as png_const_charp,
                length as usize,
                &raw mut state,
                &raw mut i,
            ) == 0
                || i != length as usize
            {
                png_chunk_benign_error(png_ptr, c"bad height format".as_ptr());
            } else if PNG_FP_IS_POSITIVE(state) == false {
                png_chunk_benign_error(png_ptr, c"non-positive height".as_ptr());
            } else {
                /* This is the (only) success case. */
                png_set_sCAL_s(
                    png_ptr,
                    info_ptr,
                    *buffer.add(0) as c_int,
                    (buffer as png_charp).add(1),
                    (buffer as png_charp).add(heighti),
                );
                return handled_ok;
            }
        }

        handled_error
    }
}

unsafe extern "C-unwind" fn png_handle_tIME(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let mut buf: [png_byte; 7] = [0; 7];
        let mut mod_time: png_time = png_time::default();

        if ((*png_ptr).mode & PNG_HAVE_IDAT) != 0 {
            (*png_ptr).mode |= PNG_AFTER_IDAT;
        }

        png_crc_read(png_ptr, buf.as_mut_ptr(), 7);

        if png_crc_finish(png_ptr, 0) != 0 {
            return handled_error;
        }

        mod_time.second = buf[6];
        mod_time.minute = buf[5];
        mod_time.hour = buf[4];
        mod_time.day = buf[3];
        mod_time.month = buf[2];
        mod_time.year = png_get_uint_16(buf.as_ptr());

        png_set_tIME(png_ptr, info_ptr, &raw mut mod_time);
        PNG_UNUSED(length);
        handled_ok
    }
}

/* Note: this does not properly handle chunks that are > 64K under DOS */
unsafe extern "C-unwind" fn png_handle_tEXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let mut text_info: png_text = core::mem::zeroed();
        let buffer: png_bytep;
        let key: png_charp;
        let mut text: png_charp;
        let skip: png_uint_32 = 0;

        if (*png_ptr).user_chunk_cache_max != 0 {
            if (*png_ptr).user_chunk_cache_max == 1 {
                png_crc_finish(png_ptr, length);
                return handled_error;
            }

            (*png_ptr).user_chunk_cache_max -= 1;
            if (*png_ptr).user_chunk_cache_max == 1 {
                png_crc_finish(png_ptr, length);
                png_chunk_benign_error(png_ptr, c"no space in chunk cache".as_ptr());
                return handled_error;
            }
        }

        buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

        if buffer.is_null() {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
            return handled_error;
        }

        png_crc_read(png_ptr, buffer, length);

        if png_crc_finish(png_ptr, skip) != 0 {
            return handled_error;
        }

        key = buffer as png_charp;
        *key.add(length as usize) = 0;

        text = key;
        while *text != 0 {
            /* Empty loop to find end of key */
            text = text.add(1);
        }

        if text != key.add(length as usize) {
            text = text.add(1);
        }

        text_info.compression = PNG_TEXT_COMPRESSION_NONE;
        text_info.key = key;
        text_info.lang = core::ptr::null_mut();
        text_info.lang_key = core::ptr::null_mut();
        text_info.itxt_length = 0;
        text_info.text = text;
        text_info.text_length = strlen(text);

        if png_set_text_2(png_ptr, info_ptr, &raw mut text_info, 1) == 0 {
            return handled_ok;
        }

        png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
        handled_error
    }
}

/* Note: this does not correctly handle chunks that are > 64K under DOS */
unsafe extern "C-unwind" fn png_handle_zTXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let mut errmsg: png_const_charp = core::ptr::null();
        let mut buffer: png_bytep;
        let mut keyword_length: png_uint_32;

        if (*png_ptr).user_chunk_cache_max != 0 {
            if (*png_ptr).user_chunk_cache_max == 1 {
                png_crc_finish(png_ptr, length);
                return handled_error;
            }

            (*png_ptr).user_chunk_cache_max -= 1;
            if (*png_ptr).user_chunk_cache_max == 1 {
                png_crc_finish(png_ptr, length);
                png_chunk_benign_error(png_ptr, c"no space in chunk cache".as_ptr());
                return handled_error;
            }
        }

        /* Note, "length" is sufficient here. */
        buffer = png_read_buffer(png_ptr, length as png_alloc_size_t);

        if buffer.is_null() {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
            return handled_error;
        }

        png_crc_read(png_ptr, buffer, length);

        if png_crc_finish(png_ptr, 0) != 0 {
            return handled_error;
        }

        /* TODO: also check that the keyword contents match the spec! */
        keyword_length = 0;
        while keyword_length < length && *buffer.add(keyword_length as usize) != 0 {
            keyword_length += 1;
        }

        if keyword_length > 79 || keyword_length < 1 {
            errmsg = c"bad keyword".as_ptr();
        }
        /* zTXt must have some LZ data after the keyword. */
        else if keyword_length + 3 > length {
            errmsg = c"truncated".as_ptr();
        } else if *buffer.add((keyword_length + 1) as usize) != PNG_COMPRESSION_TYPE_BASE as png_byte
        {
            errmsg = c"unknown compression type".as_ptr();
        } else {
            let mut uncompressed_length: png_alloc_size_t = PNG_SIZE_MAX;

            if png_decompress_chunk(
                png_ptr,
                length,
                keyword_length + 2,
                &raw mut uncompressed_length,
                1, /*terminate*/
            ) == Z_STREAM_END
            {
                let mut text: png_text = core::mem::zeroed();

                if (*png_ptr).read_buffer.is_null() {
                    errmsg = c"Read failure in png_handle_zTXt".as_ptr();
                } else {
                    /* It worked. */
                    buffer = (*png_ptr).read_buffer;
                    *buffer.add((uncompressed_length + (keyword_length + 2) as usize) as usize) = 0;

                    text.compression = PNG_TEXT_COMPRESSION_zTXt;
                    text.key = buffer as png_charp;
                    text.text = (buffer.add((keyword_length + 2) as usize)) as png_charp;
                    text.text_length = uncompressed_length;
                    text.itxt_length = 0;
                    text.lang = core::ptr::null_mut();
                    text.lang_key = core::ptr::null_mut();

                    if png_set_text_2(png_ptr, info_ptr, &raw mut text, 1) == 0 {
                        return handled_ok;
                    }

                    errmsg = c"out of memory".as_ptr();
                }
            } else {
                errmsg = (*png_ptr).zstream.msg;
            }
        }

        png_chunk_benign_error(png_ptr, errmsg);
        handled_error
    }
}

/* Note: this does not correctly handle chunks that are > 64K under DOS */
unsafe extern "C-unwind" fn png_handle_iTXt(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let mut errmsg: png_const_charp = core::ptr::null();
        let mut buffer: png_bytep;
        let mut prefix_length: png_uint_32;

        if (*png_ptr).user_chunk_cache_max != 0 {
            if (*png_ptr).user_chunk_cache_max == 1 {
                png_crc_finish(png_ptr, length);
                return handled_error;
            }

            (*png_ptr).user_chunk_cache_max -= 1;
            if (*png_ptr).user_chunk_cache_max == 1 {
                png_crc_finish(png_ptr, length);
                png_chunk_benign_error(png_ptr, c"no space in chunk cache".as_ptr());
                return handled_error;
            }
        }

        buffer = png_read_buffer(png_ptr, (length + 1) as png_alloc_size_t);

        if buffer.is_null() {
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"out of memory".as_ptr());
            return handled_error;
        }

        png_crc_read(png_ptr, buffer, length);

        if png_crc_finish(png_ptr, 0) != 0 {
            return handled_error;
        }

        /* First the keyword. */
        prefix_length = 0;
        while prefix_length < length && *buffer.add(prefix_length as usize) != 0 {
            prefix_length += 1;
        }

        /* Perform a basic check on the keyword length here. */
        if prefix_length > 79 || prefix_length < 1 {
            errmsg = c"bad keyword".as_ptr();
        }
        /* Expect keyword, compression flag, compression type, language, translated
         * keyword then the text.
         */
        else if prefix_length + 5 > length {
            errmsg = c"truncated".as_ptr();
        } else if *buffer.add((prefix_length + 1) as usize) == 0
            || (*buffer.add((prefix_length + 1) as usize) == 1
                && *buffer.add((prefix_length + 2) as usize) == PNG_COMPRESSION_TYPE_BASE as png_byte)
        {
            let compressed: c_int = (*buffer.add((prefix_length + 1) as usize) != 0) as c_int;
            let language_offset: png_uint_32;
            let translated_keyword_offset: png_uint_32;
            let mut uncompressed_length: png_alloc_size_t = 0;

            /* Now the language tag */
            prefix_length += 3;
            language_offset = prefix_length;

            while prefix_length < length && *buffer.add(prefix_length as usize) != 0 {
                prefix_length += 1;
            }

            /* WARNING: the length may be invalid here, this is checked below. */
            prefix_length += 1;
            translated_keyword_offset = prefix_length;

            while prefix_length < length && *buffer.add(prefix_length as usize) != 0 {
                prefix_length += 1;
            }

            /* prefix_length should now be at the trailing '\0'. */
            prefix_length += 1;

            if compressed == 0 && prefix_length <= length {
                uncompressed_length = (length - prefix_length) as png_alloc_size_t;
            } else if compressed != 0 && prefix_length < length {
                uncompressed_length = PNG_SIZE_MAX;

                if png_decompress_chunk(
                    png_ptr,
                    length,
                    prefix_length,
                    &raw mut uncompressed_length,
                    1, /*terminate*/
                ) == Z_STREAM_END
                {
                    buffer = (*png_ptr).read_buffer;
                } else {
                    errmsg = (*png_ptr).zstream.msg;
                }
            } else {
                errmsg = c"truncated".as_ptr();
            }

            if errmsg.is_null() {
                let mut text: png_text = core::mem::zeroed();

                *buffer.add((uncompressed_length + prefix_length as usize) as usize) = 0;

                if compressed == 0 {
                    text.compression = PNG_ITXT_COMPRESSION_NONE;
                } else {
                    text.compression = PNG_ITXT_COMPRESSION_zTXt;
                }

                text.key = buffer as png_charp;
                text.lang = (buffer as png_charp).add(language_offset as usize);
                text.lang_key = (buffer as png_charp).add(translated_keyword_offset as usize);
                text.text = (buffer as png_charp).add(prefix_length as usize);
                text.text_length = 0;
                text.itxt_length = uncompressed_length;

                if png_set_text_2(png_ptr, info_ptr, &raw mut text, 1) == 0 {
                    return handled_ok;
                }

                errmsg = c"out of memory".as_ptr();
            }
        } else {
            errmsg = c"bad compression info".as_ptr();
        }

        if !errmsg.is_null() {
            png_chunk_benign_error(png_ptr, errmsg);
        }
        handled_error
    }
}

/* Utility function for png_handle_unknown; set up png_ptr::unknown_chunk */
unsafe fn png_cache_unknown_chunk(png_ptr: png_structrp, length: png_uint_32) -> c_int {
    unsafe {
        let limit: png_alloc_size_t = png_chunk_max(png_ptr);

        if !(*png_ptr).unknown_chunk.data.is_null() {
            png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
            (*png_ptr).unknown_chunk.data = core::ptr::null_mut();
        }

        if length as png_alloc_size_t <= limit {
            PNG_CSTRING_FROM_CHUNK(
                (*png_ptr).unknown_chunk.name.as_mut_ptr() as *mut c_char,
                (*png_ptr).chunk_name,
            );
            /* The following is safe because of the PNG_SIZE_MAX init above */
            (*png_ptr).unknown_chunk.size = length as usize /*SAFE*/;
            /* 'mode' is a flag array, only the bottom four bits matter here */
            (*png_ptr).unknown_chunk.location = (*png_ptr).mode as png_byte /*SAFE*/;

            if length == 0 {
                (*png_ptr).unknown_chunk.data = core::ptr::null_mut();
            } else {
                /* Do a 'warn' here - it is handled below. */
                (*png_ptr).unknown_chunk.data =
                    png_malloc_warn(png_ptr, length as png_alloc_size_t) as png_bytep;
            }
        }

        if (*png_ptr).unknown_chunk.data.is_null() && length > 0 {
            /* This is benign because we clean up correctly */
            png_crc_finish(png_ptr, length);
            png_chunk_benign_error(png_ptr, c"unknown chunk exceeds memory limits".as_ptr());
            0
        } else {
            if length > 0 {
                png_crc_read(png_ptr, (*png_ptr).unknown_chunk.data, length);
            }
            png_crc_finish(png_ptr, 0);
            1
        }
    }
}

/* Handle an unknown, or known but disabled, chunk */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_handle_unknown(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
    mut keep: c_int,
) -> png_handle_result_code {
    unsafe {
        let mut handled: png_handle_result_code = handled_discarded; /* the default */

        /* The user callback takes precedence over the chunk keep value. */
        if (*png_ptr).read_user_chunk_fn.is_some() {
            if png_cache_unknown_chunk(png_ptr, length) != 0 {
                /* Callback to user unknown chunk handler */
                let ret: c_int = ((*png_ptr).read_user_chunk_fn.unwrap())(
                    png_ptr,
                    &raw mut (*png_ptr).unknown_chunk,
                );

                if ret < 0
                /* handled_error */
                {
                    png_chunk_error(png_ptr, c"error in user chunk".as_ptr());
                } else if ret == 0 {
                    /* If the keep value is 'default' or 'never' override it. */
                    if keep < PNG_HANDLE_CHUNK_IF_SAFE {
                        if (*png_ptr).unknown_default < PNG_HANDLE_CHUNK_IF_SAFE {
                            png_chunk_warning(png_ptr, c"Saving unknown chunk:".as_ptr());
                            png_app_warning(
                                png_ptr,
                                c"forcing save of an unhandled chunk; please call png_set_keep_unknown_chunks".as_ptr(),
                            );
                            /* with keep = PNG_HANDLE_CHUNK_IF_SAFE */
                        }
                        keep = PNG_HANDLE_CHUNK_IF_SAFE;
                    }
                } else
                /* chunk was handled */
                {
                    handled = handled_ok;
                    /* Critical chunks can be safely discarded at this point. */
                    keep = PNG_HANDLE_CHUNK_NEVER;
                }
            } else {
                keep = PNG_HANDLE_CHUNK_NEVER; /* insufficient memory */
            }
        } else
        /* Use the SAVE_UNKNOWN_CHUNKS code or skip the chunk */
        {
            /* keep is currently just the per-chunk setting. */
            if keep == PNG_HANDLE_CHUNK_AS_DEFAULT {
                keep = (*png_ptr).unknown_default;
            }

            if keep == PNG_HANDLE_CHUNK_ALWAYS
                || (keep == PNG_HANDLE_CHUNK_IF_SAFE
                    && PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name) != 0)
            {
                if png_cache_unknown_chunk(png_ptr, length) == 0 {
                    keep = PNG_HANDLE_CHUNK_NEVER;
                }
            } else {
                png_crc_finish(png_ptr, length);
            }
        }

        /* Now store the chunk in the chunk list if appropriate. */
        if keep == PNG_HANDLE_CHUNK_ALWAYS
            || (keep == PNG_HANDLE_CHUNK_IF_SAFE
                && PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name) != 0)
        {
            match (*png_ptr).user_chunk_cache_max {
                2 => {
                    (*png_ptr).user_chunk_cache_max = 1;
                    png_chunk_benign_error(png_ptr, c"no space in chunk cache".as_ptr());
                    /* FALLTHROUGH */
                    /* NOTE: prior to 1.6.0 this case resulted in an unknown critical
                     * chunk being skipped, now there will be a hard error below.
                     */
                    /* case 1: break */
                }
                1 => {
                    /* NOTE: prior to 1.6.0 this case resulted in an unknown critical
                     * chunk being skipped, now there will be a hard error below.
                     */
                    /* break */
                }
                0 => {
                    /* no limit */
                    /* Here when the limit isn't reached or when limits are compiled
                     * out; store the chunk.
                     */
                    png_set_unknown_chunks(
                        png_ptr,
                        info_ptr,
                        &raw mut (*png_ptr).unknown_chunk,
                        1,
                    );
                    handled = handled_saved;
                }
                _ => {
                    /* default: not at limit */
                    (*png_ptr).user_chunk_cache_max -= 1;
                    /* FALLTHROUGH */
                    /* Here when the limit isn't reached or when limits are compiled
                     * out; store the chunk.
                     */
                    png_set_unknown_chunks(
                        png_ptr,
                        info_ptr,
                        &raw mut (*png_ptr).unknown_chunk,
                        1,
                    );
                    handled = handled_saved;
                }
            }
        }

        /* Regardless of the error handling below the cached data (if any) can be
         * freed now.
         */
        if !(*png_ptr).unknown_chunk.data.is_null() {
            png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
        }
        (*png_ptr).unknown_chunk.data = core::ptr::null_mut();

        /* Check for unhandled critical chunks */
        if handled < handled_saved && PNG_CHUNK_CRITICAL((*png_ptr).chunk_name) != 0 {
            png_chunk_error(png_ptr, c"unhandled critical chunk".as_ptr());
        }

        handled
    }
}

/* APNG handling: cause unknown handling for acTL, fcTL, fdAT (handler NULL). */

/*
 * 1.6.47: This is the new table driven interface to all the chunk handling.
 */
struct ReadChunk {
    handler: Option<
        unsafe extern "C-unwind" fn(png_structrp, png_inforp, png_uint_32) -> png_handle_result_code,
    >,
    max_length: png_uint_32, /* :12 */
    min_length: png_uint_32, /* :8 */
    pos_before: png_uint_32, /* :4 */
    pos_after: png_uint_32,  /* :4 */
    multiple: png_uint_32,   /* :1 */
}

const NoCheck: png_uint_32 = 0x801; /* Do not check the maximum length */
const Limit: png_uint_32 = 0x802; /* Limit to png_chunk_max bytes */
const LKMin: png_uint_32 = 3 + LZ77Min; /* Minimum length of keyword+LZ77 */

const hIHDR: png_uint_32 = PNG_HAVE_IHDR;
const hPLTE: png_uint_32 = PNG_HAVE_PLTE;
const hIDAT: png_uint_32 = PNG_HAVE_IDAT;
const hCOL: png_uint_32 = PNG_HAVE_PLTE | PNG_HAVE_IDAT;
const aIDAT: png_uint_32 = PNG_AFTER_IDAT;

/* Table in PNG_KNOWN_CHUNKS order (index == PNG_INDEX_cHNK).
 * Entry: handler, max_length, min_length, pos_before, pos_after, multiple.
 */
static read_chunks: [ReadChunk; PNG_INDEX_unknown as usize] = [
    /* IHDR: CDIHDR  13U, 13U, hIHDR, 0, 0 */
    ReadChunk { handler: Some(png_handle_IHDR), max_length: 13, min_length: 13, pos_before: hIHDR, pos_after: 0, multiple: 0 },
    /* PLTE: CDPLTE NoCheck, 0, 0, hIHDR, 1 */
    ReadChunk { handler: Some(png_handle_PLTE), max_length: NoCheck, min_length: 0, pos_before: 0, pos_after: hIHDR, multiple: 1 },
    /* IDAT: CDIDAT NoCheck, 0, aIDAT, hIHDR, 1 -- handler NULL */
    ReadChunk { handler: None, max_length: NoCheck, min_length: 0, pos_before: aIDAT, pos_after: hIHDR, multiple: 1 },
    /* IEND: CDIEND NoCheck, 0, 0, aIDAT, 0 */
    ReadChunk { handler: Some(png_handle_IEND), max_length: NoCheck, min_length: 0, pos_before: 0, pos_after: aIDAT, multiple: 0 },
    /* acTL: CDacTL 8, 8, hIDAT, hIHDR, 0 -- handler NULL */
    ReadChunk { handler: None, max_length: 8, min_length: 8, pos_before: hIDAT, pos_after: hIHDR, multiple: 0 },
    /* bKGD: CDbKGD 6, 1, hIDAT, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_bKGD), max_length: 6, min_length: 1, pos_before: hIDAT, pos_after: hIHDR, multiple: 0 },
    /* cHRM: CDcHRM 32, 32, hCOL, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_cHRM), max_length: 32, min_length: 32, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    /* cICP: CDcICP 4, 4, hCOL, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_cICP), max_length: 4, min_length: 4, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    /* cLLI: CDcLLI 8, 8, hCOL, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_cLLI), max_length: 8, min_length: 8, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    /* eXIf: CDeXIf Limit, 4, 0, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_eXIf), max_length: Limit, min_length: 4, pos_before: 0, pos_after: hIHDR, multiple: 0 },
    /* fcTL: CDfcTL 25, 26, 0, hIHDR, 1 -- handler NULL */
    ReadChunk { handler: None, max_length: 25, min_length: 26, pos_before: 0, pos_after: hIHDR, multiple: 1 },
    /* fdAT: CDfdAT Limit, 4, hIDAT, hIHDR, 1 -- handler NULL */
    ReadChunk { handler: None, max_length: Limit, min_length: 4, pos_before: hIDAT, pos_after: hIHDR, multiple: 1 },
    /* gAMA: CDgAMA 4, 4, hCOL, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_gAMA), max_length: 4, min_length: 4, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    /* hIST: CDhIST 1024, 0, hPLTE, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_hIST), max_length: 1024, min_length: 0, pos_before: hPLTE, pos_after: hIHDR, multiple: 0 },
    /* iCCP: CDiCCP NoCheck, LKMin, hCOL, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_iCCP), max_length: NoCheck, min_length: LKMin, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    /* iTXt: CDiTXt NoCheck, 6, 0, hIHDR, 1 */
    ReadChunk { handler: Some(png_handle_iTXt), max_length: NoCheck, min_length: 6, pos_before: 0, pos_after: hIHDR, multiple: 1 },
    /* mDCV: CDmDCV 24, 24, hCOL, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_mDCV), max_length: 24, min_length: 24, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    /* oFFs: CDoFFs 9, 9, hIDAT, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_oFFs), max_length: 9, min_length: 9, pos_before: hIDAT, pos_after: hIHDR, multiple: 0 },
    /* pCAL: CDpCAL NoCheck, 14, hIDAT, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_pCAL), max_length: NoCheck, min_length: 14, pos_before: hIDAT, pos_after: hIHDR, multiple: 0 },
    /* pHYs: CDpHYs 9, 9, hIDAT, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_pHYs), max_length: 9, min_length: 9, pos_before: hIDAT, pos_after: hIHDR, multiple: 0 },
    /* sBIT: CDsBIT 4, 1, hCOL, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_sBIT), max_length: 4, min_length: 1, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    /* sCAL: CDsCAL Limit, 4, hIDAT, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_sCAL), max_length: Limit, min_length: 4, pos_before: hIDAT, pos_after: hIHDR, multiple: 0 },
    /* sPLT: CDsPLT NoCheck, 3, hIDAT, hIHDR, 1 */
    ReadChunk { handler: Some(png_handle_sPLT), max_length: NoCheck, min_length: 3, pos_before: hIDAT, pos_after: hIHDR, multiple: 1 },
    /* sRGB: CDsRGB 1, 1, hCOL, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_sRGB), max_length: 1, min_length: 1, pos_before: hCOL, pos_after: hIHDR, multiple: 0 },
    /* tEXt: CDtEXt NoCheck, 2, 0, hIHDR, 1 */
    ReadChunk { handler: Some(png_handle_tEXt), max_length: NoCheck, min_length: 2, pos_before: 0, pos_after: hIHDR, multiple: 1 },
    /* tIME: CDtIME 7, 7, 0, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_tIME), max_length: 7, min_length: 7, pos_before: 0, pos_after: hIHDR, multiple: 0 },
    /* tRNS: CDtRNS 256, 0, hIDAT, hIHDR, 0 */
    ReadChunk { handler: Some(png_handle_tRNS), max_length: 256, min_length: 0, pos_before: hIDAT, pos_after: hIHDR, multiple: 0 },
    /* zTXt: CDzTXt Limit, LKMin, 0, hIHDR, 1 */
    ReadChunk { handler: Some(png_handle_zTXt), max_length: Limit, min_length: LKMin, pos_before: 0, pos_after: hIHDR, multiple: 1 },
];

type png_index = u32;

unsafe fn png_chunk_index_from_name(chunk_name: png_uint_32) -> png_index {
    /* For chunk png_cHNK return PNG_INDEX_cHNK. */
    match chunk_name {
        png_IHDR => PNG_INDEX_IHDR,
        png_PLTE => PNG_INDEX_PLTE,
        png_IDAT => PNG_INDEX_IDAT,
        png_IEND => PNG_INDEX_IEND,
        png_acTL => PNG_INDEX_acTL,
        png_bKGD => PNG_INDEX_bKGD,
        png_cHRM => PNG_INDEX_cHRM,
        png_cICP => PNG_INDEX_cICP,
        png_cLLI => PNG_INDEX_cLLI,
        png_eXIf => PNG_INDEX_eXIf,
        png_fcTL => PNG_INDEX_fcTL,
        png_fdAT => PNG_INDEX_fdAT,
        png_gAMA => PNG_INDEX_gAMA,
        png_hIST => PNG_INDEX_hIST,
        png_iCCP => PNG_INDEX_iCCP,
        png_iTXt => PNG_INDEX_iTXt,
        png_mDCV => PNG_INDEX_mDCV,
        png_oFFs => PNG_INDEX_oFFs,
        png_pCAL => PNG_INDEX_pCAL,
        png_pHYs => PNG_INDEX_pHYs,
        png_sBIT => PNG_INDEX_sBIT,
        png_sCAL => PNG_INDEX_sCAL,
        png_sPLT => PNG_INDEX_sPLT,
        png_sRGB => PNG_INDEX_sRGB,
        png_tEXt => PNG_INDEX_tEXt,
        png_tIME => PNG_INDEX_tIME,
        png_tRNS => PNG_INDEX_tRNS,
        png_zTXt => PNG_INDEX_zTXt,
        _ => PNG_INDEX_unknown,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_handle_chunk(
    png_ptr: png_structrp,
    info_ptr: png_inforp,
    length: png_uint_32,
) -> png_handle_result_code {
    unsafe {
        let chunk_name: png_uint_32 = (*png_ptr).chunk_name;
        let chunk_index: png_index = png_chunk_index_from_name(chunk_name);

        let mut handled: png_handle_result_code = handled_error;
        let mut errmsg: png_const_charp = core::ptr::null();

        /* Is this a known chunk? */
        if chunk_index == PNG_INDEX_unknown
            || read_chunks[chunk_index as usize].handler.is_none()
        {
            handled = png_handle_unknown(
                png_ptr,
                info_ptr,
                length,
                PNG_HANDLE_CHUNK_AS_DEFAULT,
            );
        }
        /* First check the position. */
        else if chunk_index != PNG_INDEX_IHDR && ((*png_ptr).mode & PNG_HAVE_IHDR) == 0 {
            png_chunk_error(png_ptr, c"missing IHDR".as_ptr()); /* NORETURN */
        }
        /* Before all the pos_before chunks, after all the pos_after chunks. */
        else if ((*png_ptr).mode & read_chunks[chunk_index as usize].pos_before) != 0
            || ((*png_ptr).mode & read_chunks[chunk_index as usize].pos_after)
                != read_chunks[chunk_index as usize].pos_after
        {
            errmsg = c"out of place".as_ptr();
        }
        /* Now check for duplicates. */
        else if read_chunks[chunk_index as usize].multiple == 0
            && png_file_has_chunk(png_ptr, chunk_index)
        {
            errmsg = c"duplicate".as_ptr();
        } else if length < read_chunks[chunk_index as usize].min_length {
            errmsg = c"too short".as_ptr();
        } else {
            let max_length: c_uint = read_chunks[chunk_index as usize].max_length;

            /* Replicates the C switch with 'goto MeetsLimit'. */
            let meets_limit: bool = if max_length == Limit {
                if length as png_alloc_size_t <= png_chunk_max(png_ptr) {
                    true
                } else {
                    errmsg = c"length exceeds libpng limit".as_ptr();
                    false
                }
            } else if max_length == NoCheck {
                true
            } else {
                /* default */
                if length <= max_length {
                    true
                } else {
                    errmsg = c"too long".as_ptr();
                    false
                }
            };

            if meets_limit {
                handled = (read_chunks[chunk_index as usize].handler.unwrap())(
                    png_ptr, info_ptr, length,
                );
            }
        }

        /* If there was an error or the chunk was simply skipped it is not counted
         * as 'seen'.
         */
        if !errmsg.is_null() {
            if PNG_CHUNK_CRITICAL(chunk_name) != 0
            /* stop immediately */
            {
                png_chunk_error(png_ptr, errmsg);
            } else
            /* ancillary chunk */
            {
                /* The chunk data is skipped: */
                png_crc_finish(png_ptr, length);
                png_chunk_benign_error(png_ptr, errmsg);
            }
        } else if handled >= handled_saved {
            if chunk_index != PNG_INDEX_unknown {
                png_file_add_chunk(png_ptr, chunk_index);
            }
        }

        handled
    }
}

#[inline]
fn PNG_PASS_START_COL(pass: c_int) -> png_uint_32 {
    (((1 & pass) << (3 - (((pass) + 1) >> 1))) & 7) as png_uint_32
}
#[inline]
fn PNG_PASS_COL_OFFSET(pass: c_int) -> png_uint_32 {
    (1 << ((7 - pass) >> 1)) as png_uint_32
}

/* Precomputed compile-time masks (PNG_USE_COMPILE_TIME_MASKS = 1).
 * Indexed by [PACKSWAP(png)][DEPTH_INDEX(depth)][pass].
 */
static row_mask: [[[png_uint_32; 6]; 3]; 2] = [
    /* Little-endian byte masks for PACKSWAP */
    [
        [0x01010101, 0x10101010, 0x11111111, 0x44444444, 0x55555555, 0xaaaaaaaa],
        [0x00030003, 0x03000300, 0x03030303, 0x30303030, 0x33333333, 0xcccccccc],
        [0x0000000f, 0x000f0000, 0x000f000f, 0x0f000f00, 0x0f0f0f0f, 0xf0f0f0f0],
    ],
    /* Normal (big-endian byte) masks - PNG format */
    [
        [0x80808080, 0x08080808, 0x88888888, 0x22222222, 0xaaaaaaaa, 0x55555555],
        [0x00c000c0, 0xc000c000, 0xc0c0c0c0, 0x0c0c0c0c, 0xcccccccc, 0x33333333],
        [0x000000f0, 0x00f00000, 0x00f000f0, 0xf000f000, 0xf0f0f0f0, 0x0f0f0f0f],
    ],
];

static display_mask: [[[png_uint_32; 3]; 3]; 2] = [
    /* Little-endian byte masks for PACKSWAP */
    [
        [0xf0f0f0f0, 0xcccccccc, 0xaaaaaaaa],
        [0xff00ff00, 0xf0f0f0f0, 0xcccccccc],
        [0xffff0000, 0xff00ff00, 0xf0f0f0f0],
    ],
    /* Normal (big-endian byte) masks - PNG format */
    [
        [0x0f0f0f0f, 0x33333333, 0x55555555],
        [0xff00ff00, 0x0f0f0f0f, 0x33333333],
        [0xffff0000, 0xff00ff00, 0x0f0f0f0f],
    ],
];

#[inline]
fn DEPTH_INDEX(d: c_uint) -> usize {
    if d == 1 {
        0
    } else if d == 2 {
        1
    } else {
        2
    }
}

#[inline]
fn MASK(pass: c_uint, depth: c_uint, display: c_int, png: usize) -> png_uint_32 {
    if display != 0 {
        display_mask[png][DEPTH_INDEX(depth)][(pass >> 1) as usize]
    } else {
        row_mask[png][DEPTH_INDEX(depth)][pass as usize]
    }
}

/* Combines the row recently read in with the existing pixels in the row. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_combine_row(
    png_ptr: png_const_structrp,
    mut dp: png_bytep,
    display: c_int,
) {
    unsafe {
        let mut pixel_depth: c_uint = (*png_ptr).transformed_pixel_depth as c_uint;
        let mut sp: png_const_bytep = (*png_ptr).row_buf.add(1);
        let mut row_width: png_alloc_size_t = (*png_ptr).width as png_alloc_size_t;
        let pass: c_uint = (*png_ptr).pass as c_uint;
        let mut end_ptr: png_bytep = core::ptr::null_mut();
        let mut end_byte: png_byte = 0;
        let mut end_mask: c_uint;

        /* Added in 1.5.6. */
        if pixel_depth == 0 {
            png_error(png_ptr, c"internal row logic error".as_ptr());
        }

        /* Added in 1.5.4. */
        if (*png_ptr).info_rowbytes != 0
            && (*png_ptr).info_rowbytes != PNG_ROWBYTES(pixel_depth as usize, row_width)
        {
            png_error(png_ptr, c"internal row size calculation error".as_ptr());
        }

        /* Don't expect this to ever happen: */
        if row_width == 0 {
            png_error(png_ptr, c"internal row width error".as_ptr());
        }

        /* Preserve the last byte in cases where only part of it will be
         * overwritten.
         */
        end_mask = (pixel_depth.wrapping_mul(row_width as c_uint)) & 7;
        if end_mask != 0 {
            /* end_ptr == NULL is a flag to say do nothing */
            end_ptr = dp.add(PNG_ROWBYTES(pixel_depth as usize, row_width) - 1);
            end_byte = *end_ptr;
            if ((*png_ptr).transformations & PNG_PACKSWAP) != 0 {
                /* little-endian byte */
                end_mask = (0xffu32 << end_mask) as c_uint;
            } else {
                /* big-endian byte */
                end_mask = 0xff >> end_mask;
            }
            /* end_mask is now the bits to *keep* from the destination row */
        }

        /* For non-interlaced images this reduces to a memcpy(). */
        if (*png_ptr).interlaced != 0
            && ((*png_ptr).transformations & PNG_INTERLACE) != 0
            && pass < 6
            && (display == 0 ||
                /* The following copies everything for 'display' on passes 0, 2 and 4. */
                (display == 1 && (pass & 1) != 0))
        {
            /* Narrow images may have no bits in a pass. */
            if row_width <= PNG_PASS_START_COL(pass as c_int) as png_alloc_size_t {
                return;
            }

            if pixel_depth < 8 {
                let pixels_per_byte: png_uint_32 = 8 / pixel_depth;
                let mut mask: png_uint_32;

                if ((*png_ptr).transformations & PNG_PACKSWAP) != 0 {
                    mask = MASK(pass, pixel_depth, display, 0);
                } else {
                    mask = MASK(pass, pixel_depth, display, 1);
                }

                loop {
                    let mut m: png_uint_32;

                    m = mask;
                    mask = (m >> 8) | (m << 24); /* rotate right to good compilers */
                    m &= 0xff;

                    if m != 0
                    /* something to copy */
                    {
                        if m != 0xff {
                            *dp = (((*dp as c_uint) & !m) | ((*sp as c_uint) & m)) as png_byte;
                        } else {
                            *dp = *sp;
                        }
                    }

                    /* NOTE: this may overwrite the last byte with garbage. */
                    if row_width <= pixels_per_byte as png_alloc_size_t {
                        break; /* May need to restore part of the last byte */
                    }

                    row_width -= pixels_per_byte as png_alloc_size_t;
                    dp = dp.add(1);
                    sp = sp.add(1);
                }
            } else
            /* pixel_depth >= 8 */
            {
                let mut bytes_to_copy: c_uint;
                let bytes_to_jump: c_uint;

                /* Validate the depth - it must be a multiple of 8 */
                if pixel_depth & 7 != 0 {
                    png_error(png_ptr, c"invalid user transform pixel depth".as_ptr());
                }

                pixel_depth >>= 3; /* now in bytes */
                row_width *= pixel_depth as png_alloc_size_t;

                /* Regardless of pass number the Adam 7 interlace always results in a
                 * fixed number of pixels to copy then to skip.
                 */
                {
                    let offset: c_uint =
                        PNG_PASS_START_COL(pass as c_int) * pixel_depth;

                    row_width -= offset as png_alloc_size_t;
                    dp = dp.add(offset as usize);
                    sp = sp.add(offset as usize);
                }

                /* Work out the bytes to copy. */
                if display != 0 {
                    /* When doing the 'block' algorithm the pixel in the pass gets
                     * replicated to adjacent pixels.
                     */
                    bytes_to_copy = (1u32 << ((6 - pass) >> 1)) * pixel_depth;

                    /* But don't allow this number to exceed the actual row width. */
                    if bytes_to_copy as png_alloc_size_t > row_width {
                        bytes_to_copy = row_width as c_uint /*SAFE*/;
                    }
                } else
                /* normal row; Adam7 only ever gives us one pixel to copy. */
                {
                    bytes_to_copy = pixel_depth;
                }

                /* In Adam7 there is a constant offset between where the pixels go. */
                bytes_to_jump = PNG_PASS_COL_OFFSET(pass as c_int) * pixel_depth;

                /* And simply copy these bytes. */
                match bytes_to_copy {
                    1 => loop {
                        *dp = *sp;

                        if row_width <= bytes_to_jump as png_alloc_size_t {
                            return;
                        }

                        dp = dp.add(bytes_to_jump as usize);
                        sp = sp.add(bytes_to_jump as usize);
                        row_width -= bytes_to_jump as png_alloc_size_t;
                    },

                    2 => {
                        /* There is a possibility of a partial copy at the end. */
                        loop {
                            *dp.add(0) = *sp.add(0);
                            *dp.add(1) = *sp.add(1);

                            if row_width <= bytes_to_jump as png_alloc_size_t {
                                return;
                            }

                            sp = sp.add(bytes_to_jump as usize);
                            dp = dp.add(bytes_to_jump as usize);
                            row_width -= bytes_to_jump as png_alloc_size_t;

                            if row_width <= 1 {
                                break;
                            }
                        }

                        /* And there can only be one byte left at this point: */
                        *dp = *sp;
                        return;
                    }

                    3 => {
                        /* This can only be the RGB case. */
                        loop {
                            *dp.add(0) = *sp.add(0);
                            *dp.add(1) = *sp.add(1);
                            *dp.add(2) = *sp.add(2);

                            if row_width <= bytes_to_jump as png_alloc_size_t {
                                return;
                            }

                            sp = sp.add(bytes_to_jump as usize);
                            dp = dp.add(bytes_to_jump as usize);
                            row_width -= bytes_to_jump as png_alloc_size_t;
                        }
                    }

                    _ => {
                        /* Check for double byte alignment and, if possible, use a
                         * 16-bit copy.
                         */
                        if (bytes_to_copy as usize) < 16 /*else use memcpy*/
                            && png_isaligned_u16(dp)
                            && png_isaligned_u16(sp)
                            && (bytes_to_copy as usize) % core::mem::size_of::<png_uint_16>() == 0
                            && (bytes_to_jump as usize) % core::mem::size_of::<png_uint_16>() == 0
                        {
                            /* Everything is aligned for png_uint_16 copies, but try for
                             * png_uint_32 first.
                             */
                            if png_isaligned_u32(dp)
                                && png_isaligned_u32(sp)
                                && (bytes_to_copy as usize)
                                    % core::mem::size_of::<png_uint_32>()
                                    == 0
                                && (bytes_to_jump as usize)
                                    % core::mem::size_of::<png_uint_32>()
                                    == 0
                            {
                                let mut dp32: png_uint_32p = dp as png_uint_32p;
                                let mut sp32: *const png_uint_32 = sp as *const png_uint_32;
                                let skip: usize = (bytes_to_jump - bytes_to_copy) as usize
                                    / core::mem::size_of::<png_uint_32>();

                                loop {
                                    let mut c: usize = bytes_to_copy as usize;
                                    loop {
                                        *dp32 = *sp32;
                                        dp32 = dp32.add(1);
                                        sp32 = sp32.add(1);
                                        c -= core::mem::size_of::<png_uint_32>();
                                        if c == 0 {
                                            break;
                                        }
                                    }

                                    if row_width <= bytes_to_jump as png_alloc_size_t {
                                        return;
                                    }

                                    dp32 = dp32.add(skip);
                                    sp32 = sp32.add(skip);
                                    row_width -= bytes_to_jump as png_alloc_size_t;

                                    if !(bytes_to_copy as png_alloc_size_t <= row_width) {
                                        break;
                                    }
                                }

                                /* Get to here when the row_width truncates the final copy. */
                                dp = dp32 as png_bytep;
                                sp = sp32 as png_const_bytep;
                                loop {
                                    *dp = *sp;
                                    dp = dp.add(1);
                                    sp = sp.add(1);
                                    row_width -= 1;
                                    if row_width == 0 {
                                        break;
                                    }
                                }
                                return;
                            }
                            /* Else do it in 16-bit quantities. */
                            else {
                                let mut dp16: png_uint_16p = dp as png_uint_16p;
                                let mut sp16: *const png_uint_16 = sp as *const png_uint_16;
                                let skip: usize = (bytes_to_jump - bytes_to_copy) as usize
                                    / core::mem::size_of::<png_uint_16>();

                                loop {
                                    let mut c: usize = bytes_to_copy as usize;
                                    loop {
                                        *dp16 = *sp16;
                                        dp16 = dp16.add(1);
                                        sp16 = sp16.add(1);
                                        c -= core::mem::size_of::<png_uint_16>();
                                        if c == 0 {
                                            break;
                                        }
                                    }

                                    if row_width <= bytes_to_jump as png_alloc_size_t {
                                        return;
                                    }

                                    dp16 = dp16.add(skip);
                                    sp16 = sp16.add(skip);
                                    row_width -= bytes_to_jump as png_alloc_size_t;

                                    if !(bytes_to_copy as png_alloc_size_t <= row_width) {
                                        break;
                                    }
                                }

                                /* End of row - 1 byte left, bytes_to_copy > row_width: */
                                dp = dp16 as png_bytep;
                                sp = sp16 as png_const_bytep;
                                loop {
                                    *dp = *sp;
                                    dp = dp.add(1);
                                    sp = sp.add(1);
                                    row_width -= 1;
                                    if row_width == 0 {
                                        break;
                                    }
                                }
                                return;
                            }
                        }

                        /* The true default - use a memcpy: */
                        loop {
                            memcpy(
                                dp as *mut c_void,
                                sp as *const c_void,
                                bytes_to_copy as usize,
                            );

                            if row_width <= bytes_to_jump as png_alloc_size_t {
                                return;
                            }

                            sp = sp.add(bytes_to_jump as usize);
                            dp = dp.add(bytes_to_jump as usize);
                            row_width -= bytes_to_jump as png_alloc_size_t;
                            if bytes_to_copy as png_alloc_size_t > row_width {
                                bytes_to_copy = row_width as c_uint /*SAFE*/;
                            }
                        }
                    }
                }

                /* NOT REACHED*/
                #[allow(unreachable_code)]
                {
                    /* Here if pixel_depth < 8 to check 'end_ptr' below. */
                }
            } /* pixel_depth >= 8 */
        } else {
            /* If here then the switch above wasn't used so just memcpy the whole
             * row from the temporary row buffer.
             */
            memcpy(
                dp as *mut c_void,
                sp as *const c_void,
                PNG_ROWBYTES(pixel_depth as usize, row_width),
            );
        }

        /* Restore the overwritten bits from the last byte if necessary. */
        if !end_ptr.is_null() {
            *end_ptr = (((end_byte as c_uint) & end_mask)
                | ((*end_ptr as c_uint) & !end_mask)) as png_byte;
        }
    }
}

#[inline]
fn png_isaligned_u16(ptr: *const png_byte) -> bool {
    ((ptr as usize) & (core::mem::size_of::<png_uint_16>() - 1)) == 0
}
#[inline]
fn png_isaligned_u32(ptr: *const png_byte) -> bool {
    ((ptr as usize) & (core::mem::size_of::<png_uint_32>() - 1)) == 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_do_read_interlace(
    row_info: png_row_infop,
    row: png_bytep,
    pass: c_int,
    transformations: png_uint_32, /* Because these may affect the byte layout */
) {
    unsafe {
        if !row.is_null() && !row_info.is_null() {
            let final_width: png_uint_32;

            final_width = (*row_info).width * png_pass_inc[pass as usize] as png_uint_32;

            match (*row_info).pixel_depth {
                1 => {
                    let mut sp: png_bytep = row.add((((*row_info).width - 1) >> 3) as usize);
                    let mut dp: png_bytep = row.add(((final_width - 1) >> 3) as usize);
                    let mut sshift: c_uint;
                    let mut dshift: c_uint;
                    let s_start: c_uint;
                    let s_end: c_uint;
                    let s_inc: c_int;
                    let jstop: c_int = png_pass_inc[pass as usize] as c_int;
                    let mut v: png_byte;
                    let mut i: png_uint_32;
                    let mut j: c_int;

                    if (transformations & PNG_PACKSWAP) != 0 {
                        sshift = ((*row_info).width + 7) & 0x07;
                        dshift = (final_width + 7) & 0x07;
                        s_start = 7;
                        s_end = 0;
                        s_inc = -1;
                    } else {
                        sshift = 7 - (((*row_info).width + 7) & 0x07);
                        dshift = 7 - ((final_width + 7) & 0x07);
                        s_start = 0;
                        s_end = 7;
                        s_inc = 1;
                    }

                    i = 0;
                    while i < (*row_info).width {
                        v = ((*sp >> sshift) & 0x01) as png_byte;
                        j = 0;
                        while j < jstop {
                            let mut tmp: c_uint = (*dp as c_uint) & (0x7f7f >> (7 - dshift));
                            tmp |= (v as c_uint) << dshift;
                            *dp = (tmp & 0xff) as png_byte;

                            if dshift == s_end {
                                dshift = s_start;
                                dp = dp.sub(1);
                            } else {
                                dshift = (dshift as c_int + s_inc) as c_uint;
                            }
                            j += 1;
                        }

                        if sshift == s_end {
                            sshift = s_start;
                            sp = sp.sub(1);
                        } else {
                            sshift = (sshift as c_int + s_inc) as c_uint;
                        }
                        i += 1;
                    }
                }

                2 => {
                    let mut sp: png_bytep = row.add((((*row_info).width - 1) >> 2) as usize);
                    let mut dp: png_bytep = row.add(((final_width - 1) >> 2) as usize);
                    let mut sshift: c_uint;
                    let mut dshift: c_uint;
                    let s_start: c_uint;
                    let s_end: c_uint;
                    let s_inc: c_int;
                    let jstop: c_int = png_pass_inc[pass as usize] as c_int;
                    let mut i: png_uint_32;

                    if (transformations & PNG_PACKSWAP) != 0 {
                        sshift = (((*row_info).width + 3) & 0x03) << 1;
                        dshift = ((final_width + 3) & 0x03) << 1;
                        s_start = 6;
                        s_end = 0;
                        s_inc = -2;
                    } else {
                        sshift = (3 - (((*row_info).width + 3) & 0x03)) << 1;
                        dshift = (3 - ((final_width + 3) & 0x03)) << 1;
                        s_start = 0;
                        s_end = 6;
                        s_inc = 2;
                    }

                    i = 0;
                    while i < (*row_info).width {
                        let v: png_byte;
                        let mut j: c_int;

                        v = ((*sp >> sshift) & 0x03) as png_byte;
                        j = 0;
                        while j < jstop {
                            let mut tmp: c_uint = (*dp as c_uint) & (0x3f3f >> (6 - dshift));
                            tmp |= (v as c_uint) << dshift;
                            *dp = (tmp & 0xff) as png_byte;

                            if dshift == s_end {
                                dshift = s_start;
                                dp = dp.sub(1);
                            } else {
                                dshift = (dshift as c_int + s_inc) as c_uint;
                            }
                            j += 1;
                        }

                        if sshift == s_end {
                            sshift = s_start;
                            sp = sp.sub(1);
                        } else {
                            sshift = (sshift as c_int + s_inc) as c_uint;
                        }
                        i += 1;
                    }
                }

                4 => {
                    let mut sp: png_bytep = row.add((((*row_info).width - 1) >> 1) as usize);
                    let mut dp: png_bytep = row.add(((final_width - 1) >> 1) as usize);
                    let mut sshift: c_uint;
                    let mut dshift: c_uint;
                    let s_start: c_uint;
                    let s_end: c_uint;
                    let s_inc: c_int;
                    let mut i: png_uint_32;
                    let jstop: c_int = png_pass_inc[pass as usize] as c_int;

                    if (transformations & PNG_PACKSWAP) != 0 {
                        sshift = (((*row_info).width + 1) & 0x01) << 2;
                        dshift = ((final_width + 1) & 0x01) << 2;
                        s_start = 4;
                        s_end = 0;
                        s_inc = -4;
                    } else {
                        sshift = (1 - (((*row_info).width + 1) & 0x01)) << 2;
                        dshift = (1 - ((final_width + 1) & 0x01)) << 2;
                        s_start = 0;
                        s_end = 4;
                        s_inc = 4;
                    }

                    i = 0;
                    while i < (*row_info).width {
                        let v: png_byte = ((*sp >> sshift) & 0x0f) as png_byte;
                        let mut j: c_int;

                        j = 0;
                        while j < jstop {
                            let mut tmp: c_uint = (*dp as c_uint) & (0xf0f >> (4 - dshift));
                            tmp |= (v as c_uint) << dshift;
                            *dp = (tmp & 0xff) as png_byte;

                            if dshift == s_end {
                                dshift = s_start;
                                dp = dp.sub(1);
                            } else {
                                dshift = (dshift as c_int + s_inc) as c_uint;
                            }
                            j += 1;
                        }

                        if sshift == s_end {
                            sshift = s_start;
                            sp = sp.sub(1);
                        } else {
                            sshift = (sshift as c_int + s_inc) as c_uint;
                        }
                        i += 1;
                    }
                }

                _ => {
                    let pixel_bytes: usize = ((*row_info).pixel_depth >> 3) as usize;

                    let mut sp: png_bytep =
                        row.add(((*row_info).width - 1) as usize * pixel_bytes);

                    let mut dp: png_bytep = row.add((final_width - 1) as usize * pixel_bytes);

                    let jstop: c_int = png_pass_inc[pass as usize] as c_int;
                    let mut i: png_uint_32;

                    i = 0;
                    while i < (*row_info).width {
                        let mut v: [png_byte; 8] = [0; 8]; /* SAFE; pixel_depth does not exceed 64 */
                        let mut j: c_int;

                        memcpy(
                            v.as_mut_ptr() as *mut c_void,
                            sp as *const c_void,
                            pixel_bytes,
                        );

                        j = 0;
                        while j < jstop {
                            memcpy(
                                dp as *mut c_void,
                                v.as_ptr() as *const c_void,
                                pixel_bytes,
                            );
                            dp = dp.sub(pixel_bytes);
                            j += 1;
                        }

                        sp = sp.sub(pixel_bytes);
                        i += 1;
                    }
                }
            }

            (*row_info).width = final_width;
            (*row_info).rowbytes = PNG_ROWBYTES((*row_info).pixel_depth as usize, final_width as usize);
        }
    }
}

unsafe extern "C-unwind" fn png_read_filter_row_sub(
    row_info: png_row_infop,
    row: png_bytep,
    prev_row: png_const_bytep,
) {
    unsafe {
        let mut i: usize;
        let istop: usize = (*row_info).rowbytes;
        let bpp: c_uint = ((*row_info).pixel_depth as c_uint + 7) >> 3;
        let mut rp: png_bytep = row.add(bpp as usize);

        PNG_UNUSED(prev_row);

        i = bpp as usize;
        while i < istop {
            *rp = (((*rp as c_int) + (*rp.sub(bpp as usize) as c_int)) & 0xff) as png_byte;
            rp = rp.add(1);
            i += 1;
        }
    }
}

unsafe extern "C-unwind" fn png_read_filter_row_up(
    row_info: png_row_infop,
    row: png_bytep,
    prev_row: png_const_bytep,
) {
    unsafe {
        let mut i: usize;
        let istop: usize = (*row_info).rowbytes;
        let mut rp: png_bytep = row;
        let mut pp: png_const_bytep = prev_row;

        i = 0;
        while i < istop {
            *rp = (((*rp as c_int) + (*pp as c_int)) & 0xff) as png_byte;
            pp = pp.add(1);
            rp = rp.add(1);
            i += 1;
        }
    }
}

unsafe extern "C-unwind" fn png_read_filter_row_avg(
    row_info: png_row_infop,
    row: png_bytep,
    prev_row: png_const_bytep,
) {
    unsafe {
        let mut i: usize;
        let mut rp: png_bytep = row;
        let mut pp: png_const_bytep = prev_row;
        let bpp: c_uint = ((*row_info).pixel_depth as c_uint + 7) >> 3;
        let istop: usize = (*row_info).rowbytes - bpp as usize;

        i = 0;
        while i < bpp as usize {
            *rp = (((*rp as c_int) + ((*pp as c_int) / 2)) & 0xff) as png_byte;
            pp = pp.add(1);
            rp = rp.add(1);
            i += 1;
        }

        i = 0;
        while i < istop {
            *rp = (((*rp as c_int)
                + ((*pp as c_int + *rp.sub(bpp as usize) as c_int) / 2))
                & 0xff) as png_byte;
            pp = pp.add(1);
            rp = rp.add(1);
            i += 1;
        }
    }
}

unsafe extern "C-unwind" fn png_read_filter_row_paeth_1byte_pixel(
    row_info: png_row_infop,
    mut row: png_bytep,
    mut prev_row: png_const_bytep,
) {
    unsafe {
        let rp_end: png_bytep = row.add((*row_info).rowbytes);
        let mut a: c_int;
        let mut c: c_int;

        /* First pixel/byte */
        c = *prev_row as c_int;
        prev_row = prev_row.add(1);
        a = *row as c_int + c;
        *row = a as png_byte;
        row = row.add(1);

        /* Remainder */
        while row < rp_end {
            let b: c_int;
            let mut pa: c_int;
            let pb: c_int;
            let mut pc: c_int;
            let p: c_int;

            a &= 0xff; /* From previous iteration or start */
            b = *prev_row as c_int;
            prev_row = prev_row.add(1);

            p = b - c;
            pc = a - c;

            pa = if p < 0 { -p } else { p };
            pb = if pc < 0 { -pc } else { pc };
            pc = if (p + pc) < 0 { -(p + pc) } else { p + pc };

            /* Find the best predictor. */
            if pb < pa {
                pa = pb;
                a = b;
            }
            if pc < pa {
                a = c;
            }

            /* Calculate the current pixel in a, and move the previous row pixel to c. */
            c = b;
            a += *row as c_int;
            *row = a as png_byte;
            row = row.add(1);
        }
    }
}

unsafe extern "C-unwind" fn png_read_filter_row_paeth_multibyte_pixel(
    row_info: png_row_infop,
    mut row: png_bytep,
    mut prev_row: png_const_bytep,
) {
    unsafe {
        let bpp: c_uint = ((*row_info).pixel_depth as c_uint + 7) >> 3;
        let mut rp_end: png_bytep = row.add(bpp as usize);

        /* Process the first pixel in the row completely. */
        while row < rp_end {
            let a: c_int = *row as c_int + *prev_row as c_int;
            prev_row = prev_row.add(1);
            *row = a as png_byte;
            row = row.add(1);
        }

        /* Remainder */
        rp_end = rp_end.add((*row_info).rowbytes - bpp as usize);

        while row < rp_end {
            let mut a: c_int;
            let b: c_int;
            let c: c_int;
            let mut pa: c_int;
            let pb: c_int;
            let mut pc: c_int;
            let p: c_int;

            c = *prev_row.sub(bpp as usize) as c_int;
            a = *row.sub(bpp as usize) as c_int;
            b = *prev_row as c_int;
            prev_row = prev_row.add(1);

            p = b - c;
            pc = a - c;

            pa = if p < 0 { -p } else { p };
            pb = if pc < 0 { -pc } else { pc };
            pc = if (p + pc) < 0 { -(p + pc) } else { p + pc };

            if pb < pa {
                pa = pb;
                a = b;
            }
            if pc < pa {
                a = c;
            }

            a += *row as c_int;
            *row = a as png_byte;
            row = row.add(1);
        }
    }
}

unsafe fn png_init_filter_functions(pp: png_structrp) {
    unsafe {
        let bpp: c_uint = ((*pp).pixel_depth as c_uint + 7) >> 3;

        (*pp).read_filter[(PNG_FILTER_VALUE_SUB - 1) as usize] = Some(png_read_filter_row_sub);
        (*pp).read_filter[(PNG_FILTER_VALUE_UP - 1) as usize] = Some(png_read_filter_row_up);
        (*pp).read_filter[(PNG_FILTER_VALUE_AVG - 1) as usize] = Some(png_read_filter_row_avg);
        if bpp == 1 {
            (*pp).read_filter[(PNG_FILTER_VALUE_PAETH - 1) as usize] =
                Some(png_read_filter_row_paeth_1byte_pixel);
        } else {
            (*pp).read_filter[(PNG_FILTER_VALUE_PAETH - 1) as usize] =
                Some(png_read_filter_row_paeth_multibyte_pixel);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_filter_row(
    pp: png_structrp,
    row_info: png_row_infop,
    row: png_bytep,
    prev_row: png_const_bytep,
    filter: c_int,
) {
    unsafe {
        if filter > PNG_FILTER_VALUE_NONE && filter < PNG_FILTER_VALUE_LAST {
            if (*pp).read_filter[0].is_none() {
                png_init_filter_functions(pp);
            }

            ((*pp).read_filter[(filter - 1) as usize].unwrap())(row_info, row, prev_row);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_IDAT_data(
    png_ptr: png_structrp,
    output: png_bytep,
    mut avail_out: png_alloc_size_t,
) {
    unsafe {
        /* Loop reading IDATs and decompressing the result into output[avail_out] */
        (*png_ptr).zstream.next_out = output;
        (*png_ptr).zstream.avail_out = 0; /* safety: set below */

        if output.is_null() {
            avail_out = 0;
        }

        loop {
            let ret: c_int;
            let mut tmpbuf: [png_byte; PNG_INFLATE_BUF_SIZE] = [0; PNG_INFLATE_BUF_SIZE];

            if (*png_ptr).zstream.avail_in == 0 {
                let mut avail_in: uInt;
                let buffer: png_bytep;

                while (*png_ptr).idat_size == 0 {
                    png_crc_finish(png_ptr, 0);

                    (*png_ptr).idat_size = png_read_chunk_header(png_ptr);
                    /* This is an error even in the 'check' case. */
                    if (*png_ptr).chunk_name != png_IDAT {
                        png_error(png_ptr, c"Not enough image data".as_ptr());
                    }
                }

                avail_in = (*png_ptr).IDAT_read_size;

                if avail_in as png_alloc_size_t > png_chunk_max(png_ptr) {
                    avail_in = png_chunk_max(png_ptr) as uInt /*SAFE*/;
                }

                if avail_in > (*png_ptr).idat_size {
                    avail_in = (*png_ptr).idat_size as uInt;
                }

                /* An error here corresponds to the system being out of memory. */
                buffer = png_read_buffer(png_ptr, avail_in as png_alloc_size_t);

                if buffer.is_null() {
                    png_chunk_error(png_ptr, c"out of memory".as_ptr());
                }

                png_crc_read(png_ptr, buffer, avail_in as png_uint_32);
                (*png_ptr).idat_size -= avail_in as png_uint_32;

                (*png_ptr).zstream.next_in = buffer;
                (*png_ptr).zstream.avail_in = avail_in;
            }

            /* And set up the output side. */
            if !output.is_null()
            /* standard read */
            {
                let mut out: uInt = ZLIB_IO_MAX;

                if out as png_alloc_size_t > avail_out {
                    out = avail_out as uInt;
                }

                avail_out -= out as png_alloc_size_t;
                (*png_ptr).zstream.avail_out = out;
            } else
            /* after last row, checking for end */
            {
                (*png_ptr).zstream.next_out = tmpbuf.as_mut_ptr();
                (*png_ptr).zstream.avail_out = core::mem::size_of_val(&tmpbuf) as uInt;
            }

            /* Use NO_FLUSH. */
            ret = PNG_INFLATE(png_ptr, Z_NO_FLUSH);

            /* Take the unconsumed output back. */
            if !output.is_null() {
                avail_out += (*png_ptr).zstream.avail_out as png_alloc_size_t;
            } else
            /* avail_out counts the extra bytes */
            {
                avail_out += core::mem::size_of_val(&tmpbuf) as png_alloc_size_t
                    - (*png_ptr).zstream.avail_out as png_alloc_size_t;
            }

            (*png_ptr).zstream.avail_out = 0;

            if ret == Z_STREAM_END {
                /* Do this for safety; we won't read any more into this row. */
                (*png_ptr).zstream.next_out = core::ptr::null_mut();

                (*png_ptr).mode |= PNG_AFTER_IDAT;
                (*png_ptr).flags |= PNG_FLAG_ZSTREAM_ENDED;

                if (*png_ptr).zstream.avail_in > 0 || (*png_ptr).idat_size > 0 {
                    png_chunk_benign_error(png_ptr, c"Extra compressed data".as_ptr());
                }
                break;
            }

            if ret != Z_OK {
                png_zstream_error(png_ptr, ret);

                if !output.is_null() {
                    png_chunk_error(png_ptr, (*png_ptr).zstream.msg);
                } else
                /* checking */
                {
                    png_chunk_benign_error(png_ptr, (*png_ptr).zstream.msg);
                    return;
                }
            }

            if !(avail_out > 0) {
                break;
            }
        }

        if avail_out > 0 {
            /* The stream ended before the image. */
            if !output.is_null() {
                png_error(png_ptr, c"Not enough image data".as_ptr());
            } else
            /* the deflate stream contained extra data */
            {
                png_chunk_benign_error(png_ptr, c"Too much image data".as_ptr());
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_finish_IDAT(png_ptr: png_structrp) {
    unsafe {
        /* We don't need any more data and the stream should have ended. */
        if ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0 {
            /* The NULL causes png_read_IDAT_data to swallow any remaining bytes. */
            png_read_IDAT_data(png_ptr, core::ptr::null_mut(), 0);
            (*png_ptr).zstream.next_out = core::ptr::null_mut(); /* safety */

            /* Now clear everything out for safety. */
            if ((*png_ptr).flags & PNG_FLAG_ZSTREAM_ENDED) == 0 {
                (*png_ptr).mode |= PNG_AFTER_IDAT;
                (*png_ptr).flags |= PNG_FLAG_ZSTREAM_ENDED;
            }
        }

        /* If the zstream has not been released do it now. */
        if (*png_ptr).zowner == png_IDAT {
            /* Always do this; the pointers otherwise point into the read buffer. */
            (*png_ptr).zstream.next_in = core::ptr::null();
            (*png_ptr).zstream.avail_in = 0;

            /* Now we no longer own the zstream. */
            (*png_ptr).zowner = 0;

            /* The slightly weird semantics of the sequential IDAT reading. */
            let _ = png_crc_finish(png_ptr, (*png_ptr).idat_size);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_finish_row(png_ptr: png_structrp) {
    unsafe {
        (*png_ptr).row_number += 1;
        if (*png_ptr).row_number < (*png_ptr).num_rows {
            return;
        }

        if (*png_ptr).interlaced != 0 {
            (*png_ptr).row_number = 0;

            /* TO DO: don't do this if prev_row isn't needed. */
            memset(
                (*png_ptr).prev_row as *mut c_void,
                0,
                (*png_ptr).rowbytes + 1,
            );

            loop {
                (*png_ptr).pass += 1;

                if (*png_ptr).pass >= 7 {
                    break;
                }

                (*png_ptr).iwidth = ((*png_ptr).width
                    + png_pass_inc[(*png_ptr).pass as usize] as png_uint_32
                    - 1
                    - png_pass_start[(*png_ptr).pass as usize] as png_uint_32)
                    / png_pass_inc[(*png_ptr).pass as usize] as png_uint_32;

                if ((*png_ptr).transformations & PNG_INTERLACE) == 0 {
                    (*png_ptr).num_rows = ((*png_ptr).height
                        + png_pass_yinc[(*png_ptr).pass as usize] as png_uint_32
                        - 1
                        - png_pass_ystart[(*png_ptr).pass as usize] as png_uint_32)
                        / png_pass_yinc[(*png_ptr).pass as usize] as png_uint_32;
                } else {
                    /* if (png_ptr->transformations & PNG_INTERLACE) */
                    break; /* libpng deinterlacing sees every row */
                }

                if !((*png_ptr).num_rows == 0 || (*png_ptr).iwidth == 0) {
                    break;
                }
            }

            if (*png_ptr).pass < 7 {
                return;
            }
        }

        /* Here after at the end of the last row of the last pass. */
        png_read_finish_IDAT(png_ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_start_row(png_ptr: png_structrp) {
    unsafe {
        let mut max_pixel_depth: c_uint;
        let mut row_bytes: usize;

        png_init_read_transformations(png_ptr);

        if (*png_ptr).interlaced != 0 {
            if ((*png_ptr).transformations & PNG_INTERLACE) == 0 {
                (*png_ptr).num_rows = ((*png_ptr).height + png_pass_yinc[0] as png_uint_32 - 1
                    - png_pass_ystart[0] as png_uint_32)
                    / png_pass_yinc[0] as png_uint_32;
            } else {
                (*png_ptr).num_rows = (*png_ptr).height;
            }

            (*png_ptr).iwidth = ((*png_ptr).width
                + png_pass_inc[(*png_ptr).pass as usize] as png_uint_32
                - 1
                - png_pass_start[(*png_ptr).pass as usize] as png_uint_32)
                / png_pass_inc[(*png_ptr).pass as usize] as png_uint_32;
        } else {
            (*png_ptr).num_rows = (*png_ptr).height;
            (*png_ptr).iwidth = (*png_ptr).width;
        }

        max_pixel_depth = (*png_ptr).pixel_depth as c_uint;

        /* WARNING: png_read_transform_info (pngrtran.c) performs a simpler set of
         * calculations to calculate the final pixel depth.
         */
        if ((*png_ptr).transformations & PNG_PACK) != 0 && (*png_ptr).bit_depth < 8 {
            max_pixel_depth = 8;
        }

        if ((*png_ptr).transformations & PNG_EXPAND) != 0 {
            if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE {
                if (*png_ptr).num_trans != 0 {
                    max_pixel_depth = 32;
                } else {
                    max_pixel_depth = 24;
                }
            } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY {
                if max_pixel_depth < 8 {
                    max_pixel_depth = 8;
                }

                if (*png_ptr).num_trans != 0 {
                    max_pixel_depth *= 2;
                }
            } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB {
                if (*png_ptr).num_trans != 0 {
                    max_pixel_depth *= 4;
                    max_pixel_depth /= 3;
                }
            }
        }

        if ((*png_ptr).transformations & PNG_EXPAND_16) != 0 {
            /* In fact it is an error if it isn't supported, but checking is
             * the safe way.
             */
            if ((*png_ptr).transformations & PNG_EXPAND) != 0 {
                if (*png_ptr).bit_depth < 16 {
                    max_pixel_depth *= 2;
                }
            } else {
                (*png_ptr).transformations &= !PNG_EXPAND_16;
            }
        }

        if ((*png_ptr).transformations & PNG_FILLER) != 0 {
            if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY {
                if max_pixel_depth <= 8 {
                    max_pixel_depth = 16;
                } else {
                    max_pixel_depth = 32;
                }
            } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB
                || (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_PALETTE
            {
                if max_pixel_depth <= 32 {
                    max_pixel_depth = 32;
                } else {
                    max_pixel_depth = 64;
                }
            }
        }

        if ((*png_ptr).transformations & PNG_GRAY_TO_RGB) != 0 {
            if ((*png_ptr).num_trans != 0 && ((*png_ptr).transformations & PNG_EXPAND) != 0)
                || ((*png_ptr).transformations & PNG_FILLER) != 0
                || (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_GRAY_ALPHA
            {
                if max_pixel_depth <= 16 {
                    max_pixel_depth = 32;
                } else {
                    max_pixel_depth = 64;
                }
            } else {
                if max_pixel_depth <= 8 {
                    if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                        max_pixel_depth = 32;
                    } else {
                        max_pixel_depth = 24;
                    }
                } else if (*png_ptr).color_type as c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                    max_pixel_depth = 64;
                } else {
                    max_pixel_depth = 48;
                }
            }
        }

        if ((*png_ptr).transformations & PNG_USER_TRANSFORM) != 0 {
            let user_pixel_depth: c_uint = (*png_ptr).user_transform_depth as c_uint
                * (*png_ptr).user_transform_channels as c_uint;

            if user_pixel_depth > max_pixel_depth {
                max_pixel_depth = user_pixel_depth;
            }
        }

        /* This value is stored in png_struct and double checked in the row read code. */
        (*png_ptr).maximum_pixel_depth = max_pixel_depth as png_byte;
        (*png_ptr).transformed_pixel_depth = 0; /* calculated on demand */

        /* Align the width on the next larger 8 pixels. */
        row_bytes = (((*png_ptr).width + 7) & !(7 as png_uint_32)) as usize;
        /* Calculate the maximum bytes needed. */
        row_bytes = PNG_ROWBYTES(max_pixel_depth as usize, row_bytes)
            + 1
            + ((max_pixel_depth as usize + 7) >> 3);

        if row_bytes + 48 > (*png_ptr).old_big_row_buf_size {
            png_free(png_ptr, (*png_ptr).big_row_buf as png_voidp);
            png_free(png_ptr, (*png_ptr).big_prev_row as png_voidp);
            (*png_ptr).big_row_buf = core::ptr::null_mut();
            (*png_ptr).big_prev_row = core::ptr::null_mut();

            if (*png_ptr).interlaced != 0 {
                (*png_ptr).big_row_buf =
                    png_calloc(png_ptr, (row_bytes + 48) as png_alloc_size_t) as png_bytep;
            } else {
                (*png_ptr).big_row_buf =
                    png_malloc(png_ptr, (row_bytes + 48) as png_alloc_size_t) as png_bytep;
            }

            (*png_ptr).big_prev_row =
                png_malloc(png_ptr, (row_bytes + 48) as png_alloc_size_t) as png_bytep;

            /* Use 16-byte aligned memory for row_buf. */
            {
                let mut temp: png_bytep = (*png_ptr).big_row_buf.add(32);
                let mut extra: usize = (temp as usize) & 0x0f;
                (*png_ptr).row_buf = temp.sub(extra).sub(1 /*filter byte*/);

                temp = (*png_ptr).big_prev_row.add(32);
                extra = (temp as usize) & 0x0f;
                (*png_ptr).prev_row = temp.sub(extra).sub(1 /*filter byte*/);
            }

            (*png_ptr).old_big_row_buf_size = row_bytes + 48;
        }

        if (*png_ptr).rowbytes > (PNG_SIZE_MAX - 1) {
            png_error(png_ptr, c"Row has too many bytes to allocate in memory".as_ptr());
        }

        memset(
            (*png_ptr).prev_row as *mut c_void,
            0,
            (*png_ptr).rowbytes + 1,
        );

        /* The sequential reader needs a buffer for IDAT. */
        if !(*png_ptr).read_buffer.is_null() {
            let buffer: png_bytep = (*png_ptr).read_buffer;

            (*png_ptr).read_buffer_size = 0;
            (*png_ptr).read_buffer = core::ptr::null_mut();
            png_free(png_ptr, buffer as png_voidp);
        }

        /* Finally claim the zstream for the inflate of the IDAT data. */
        if png_inflate_claim(png_ptr, png_IDAT) != Z_OK {
            png_error(png_ptr, (*png_ptr).zstream.msg);
        }

        (*png_ptr).flags |= PNG_FLAG_ROW_INIT;
    }
}
