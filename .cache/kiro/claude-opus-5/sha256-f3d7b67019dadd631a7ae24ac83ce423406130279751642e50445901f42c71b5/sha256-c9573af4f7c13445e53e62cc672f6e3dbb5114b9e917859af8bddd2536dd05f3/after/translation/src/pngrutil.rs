//! Translation of c_src/src/pngrutil.c lines 1..900
use crate::prelude::*;

/* LZ77Min: minimum zlib stream size. (currently unused in this range) */
#[allow(dead_code)]
const LZ77Min: c_uint = 2u32 + 5u32 + 4u32;

/* Arrays to facilitate interlacing - use pass (0 - 6) as index. */
/* Start of interlace block */
#[allow(dead_code)]
static png_pass_start: [png_byte; 7] = [0, 4, 0, 2, 0, 1, 0];
/* Offset to next interlace block */
#[allow(dead_code)]
static png_pass_inc: [png_byte; 7] = [8, 8, 4, 4, 2, 2, 1];
/* Start of interlace block in the y direction */
#[allow(dead_code)]
static png_pass_ystart: [png_byte; 7] = [0, 0, 4, 0, 2, 0, 1];
/* Offset to next interlace block in the y direction */
#[allow(dead_code)]
static png_pass_yinc: [png_byte; 7] = [8, 8, 8, 4, 4, 2, 2];

/* PNGZ_MSG_CAST(s) -- pngstruct.h macro, plain cast here. */
#[inline]
unsafe fn PNGZ_MSG_CAST(s: *const c_char) -> *const c_char {
    s
}

/* PNGZ_INPUT_CAST(b) -- pngpriv.h macro, plain cast here. */
#[inline]
unsafe fn PNGZ_INPUT_CAST(b: png_const_bytep) -> *const Bytef {
    b as *const Bytef
}

/* PNG_INFLATE(pp, flush) -- ZLIB_VERNUM >= 0x1240 so this maps to
 * png_zlib_inflate(pp, flush).
 */
#[inline]
unsafe fn PNG_INFLATE(pp: png_structrp, flush: c_int) -> c_int {
    png_zlib_inflate(pp, flush)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_uint_31(
    png_ptr: png_const_structrp,
    buf: png_const_bytep,
) -> png_uint_32 {
    let uval: png_uint_32 = png_get_uint_32(buf);

    if uval > PNG_UINT_31_MAX {
        png_error(png_ptr, cstr(b"PNG unsigned integer out of range\0"));
    }

    uval
}

/* Grab an unsigned 32-bit integer from a buffer in big-endian format. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_uint_32(buf: png_const_bytep) -> png_uint_32 {
    let uval: png_uint_32 = ((*(buf.add(0)) as png_uint_32) << 24)
        .wrapping_add((*(buf.add(1)) as png_uint_32) << 16)
        .wrapping_add((*(buf.add(2)) as png_uint_32) << 8)
        .wrapping_add(*(buf.add(3)) as png_uint_32);

    uval
}

/* Grab a signed 32-bit integer from a buffer in big-endian format.  The
 * data is stored in the PNG file in two's complement format and there
 * is no guarantee that a 'png_int_32' is exactly 32 bits, therefore
 * the following code does a two's complement to native conversion.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_int_32(buf: png_const_bytep) -> png_int_32 {
    let mut uval: png_uint_32 = png_get_uint_32(buf);
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

/* Grab an unsigned 16-bit integer from a buffer in big-endian format. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_uint_16(buf: png_const_bytep) -> png_uint_16 {
    /* ANSI-C requires an int value to accommodate at least 16 bits so this
     * works and allows the compiler not to worry about possible narrowing
     * on 32-bit systems.
     */
    let val: c_uint = ((*buf as c_uint) << 8).wrapping_add(*(buf.add(1)) as c_uint);

    val as png_uint_16
}

/* Read and check the PNG file signature */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_sig(png_ptr: png_structrp, info_ptr: png_inforp) {
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
        &mut (*info_ptr).signature[num_checked] as *mut png_byte,
        num_to_check,
    );
    (*png_ptr).sig_bytes = 8;

    if png_sig_cmp((*info_ptr).signature.as_ptr(), num_checked, num_to_check) != 0 {
        if num_checked < 4
            && png_sig_cmp(
                (*info_ptr).signature.as_ptr(),
                num_checked,
                num_to_check - 4,
            ) != 0
        {
            png_error(png_ptr, cstr(b"Not a PNG file\0"));
        } else {
            png_error(png_ptr, cstr(b"PNG file corrupted by ASCII conversion\0"));
        }
    }
    if num_checked < 3 {
        (*png_ptr).mode |= PNG_HAVE_PNG_SIGNATURE;
    }
}

/* This function is called to verify that a chunk name is valid. */
pub unsafe extern "C" fn check_chunk_name(mut name: png_uint_32) -> c_int {
    let mut t: png_uint_32;

    /* Remove bit 5 from all but the reserved byte. */
    name &= !PNG_U32(32, 32, 0, 32);
    t = (name & !0x1f1f1f1fu32) ^ 0x40404040u32;

    /* Subtract 65 for each 8-bit quantity. */
    name = name.wrapping_sub(PNG_U32(65, 65, 65, 65));
    t |= name;

    /* Subtract 26, handling the overflow. */
    name = name.wrapping_sub(PNG_U32(25, 25, 25, 26));
    t |= !name;

    ((t & 0xe0e0e0e0u32) == 0u32) as c_int
}

/* Read the chunk header (length + type name). */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_chunk_header(png_ptr: png_structrp) -> png_uint_32 {
    let mut buf: [png_byte; 8] = [0; 8];
    let chunk_name: png_uint_32;
    let length: png_uint_32;

    (*png_ptr).io_state = PNG_IO_READING | PNG_IO_CHUNK_HDR;

    /* Read the length and the chunk name. */
    png_read_data(png_ptr, buf.as_mut_ptr(), 8);
    length = png_get_uint_31(png_ptr, buf.as_ptr());
    chunk_name = PNG_CHUNK_FROM_STRING(buf.as_ptr().add(4) as *const c_char);
    (*png_ptr).chunk_name = chunk_name;

    /* Reset the crc and run it over the chunk name. */
    png_reset_crc(png_ptr);
    png_calculate_crc(png_ptr, buf.as_ptr().add(4), 4);

    /* Sanity check the length (first by <= 0x80) and the chunk name. */
    if buf[0] >= 0x80u8 {
        png_chunk_error(png_ptr, cstr(b"bad header (invalid length)\0"));
    }

    /* Check to see if chunk name is valid. */
    if check_chunk_name(chunk_name) == 0 {
        png_chunk_error(png_ptr, cstr(b"bad header (invalid type)\0"));
    }

    (*png_ptr).io_state = PNG_IO_READING | PNG_IO_CHUNK_DATA;

    length
}

/* Read data, and (optionally) run it through the CRC. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_crc_read(png_ptr: png_structrp, buf: png_bytep, length: png_uint_32) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    png_read_data(png_ptr, buf, length as usize);
    png_calculate_crc(png_ptr, buf, length as usize);
}

/* Compare the CRC stored in the PNG file with that calculated by libpng from
 * the data it has read thus far.
 */
pub unsafe extern "C" fn png_crc_error(png_ptr: png_structrp, handle_as_ancillary: c_int) -> c_int {
    let mut crc_bytes: [png_byte; 4] = [0; 4];
    let crc: png_uint_32;
    let mut need_crc: c_int = 1;

    if handle_as_ancillary != 0 || PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name) != 0 {
        if ((*png_ptr).flags & PNG_FLAG_CRC_ANCILLARY_MASK)
            == (PNG_FLAG_CRC_ANCILLARY_USE | PNG_FLAG_CRC_ANCILLARY_NOWARN)
        {
            need_crc = 0;
        }
    } else {
        /* critical */
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

/* Optionally skip data and then check the CRC. */
pub unsafe extern "C" fn png_crc_finish_critical(
    png_ptr: png_structrp,
    mut skip: png_uint_32,
    mut handle_as_ancillary: c_int,
) -> c_int {
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
        let cond = if handle_as_ancillary != 0 || PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name) != 0 {
            ((*png_ptr).flags & PNG_FLAG_CRC_ANCILLARY_NOWARN) == 0
        } else {
            ((*png_ptr).flags & PNG_FLAG_CRC_CRITICAL_USE) != 0
        };

        if cond {
            png_chunk_warning(png_ptr, cstr(b"CRC error\0"));
        } else {
            png_chunk_error(png_ptr, cstr(b"CRC error\0"));
        }

        return 1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_crc_finish(png_ptr: png_structrp, skip: png_uint_32) -> c_int {
    png_crc_finish_critical(png_ptr, skip, 0 /*critical handling*/)
}

/* Manage the read buffer. */
pub unsafe extern "C" fn png_read_buffer(
    png_ptr: png_structrp,
    new_size: png_alloc_size_t,
) -> png_bytep {
    let mut buffer: png_bytep = (*png_ptr).read_buffer;

    if new_size > png_chunk_max(png_ptr) {
        return core::ptr::null_mut();
    }

    if buffer != core::ptr::null_mut() && new_size > (*png_ptr).read_buffer_size {
        (*png_ptr).read_buffer = core::ptr::null_mut();
        (*png_ptr).read_buffer_size = 0;
        png_free(png_ptr, buffer as png_voidp);
        buffer = core::ptr::null_mut();
    }

    if buffer == core::ptr::null_mut() {
        buffer = png_malloc_base(png_ptr, new_size) as png_bytep;

        if buffer != core::ptr::null_mut() {
            memset(buffer as *mut c_void, 0, new_size); /* just in case */
            (*png_ptr).read_buffer = buffer;
            (*png_ptr).read_buffer_size = new_size;
        }
    }

    buffer
}

/* png_inflate_claim: claim the zstream for some nefarious purpose that involves
 * decompression.  Returns Z_OK on success, else a zlib error code.
 */
pub unsafe extern "C" fn png_inflate_claim(png_ptr: png_structrp, owner: png_uint_32) -> c_int {
    if (*png_ptr).zowner != 0 {
        let mut msg: [c_char; 64] = [0; 64];

        PNG_STRING_FROM_CHUNK(msg.as_mut_ptr(), (*png_ptr).zowner);
        /* So the message that results is "<chunk> using zstream". */
        let _ = png_safecat(
            msg.as_mut_ptr(),
            core::mem::size_of_val(&msg),
            4,
            cstr(b" using zstream\0"),
        );
        /* PNG_RELEASE_BUILD == 0 -> png_chunk_error branch. */
        png_chunk_error(png_ptr, msg.as_ptr());
    }

    {
        let mut ret: c_int; /* zlib return code */
        let mut window_bits: c_int = 0;

        if (((*png_ptr).options >> PNG_MAXIMUM_INFLATE_WINDOW) & 3) == PNG_OPTION_ON as png_uint_32
        {
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
            ret = inflateReset2(&mut (*png_ptr).zstream, window_bits);
        } else {
            ret = inflateInit2(&mut (*png_ptr).zstream, window_bits);

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

/* Handle the start of the inflate stream if we called inflateInit2(strm,0). */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_zlib_inflate(png_ptr: png_structrp, flush: c_int) -> c_int {
    if (*png_ptr).zstream_start != 0 && (*png_ptr).zstream.avail_in > 0 {
        if (*(*png_ptr).zstream.next_in >> 4) > 7 {
            (*png_ptr).zstream.msg = cstr(b"invalid window size (libpng)\0");
            return Z_DATA_ERROR;
        }

        (*png_ptr).zstream_start = 0;
    }

    inflate(&mut (*png_ptr).zstream, flush)
}

/* png_inflate now returns zlib error codes including Z_OK and Z_STREAM_END. */
pub unsafe extern "C" fn png_inflate(
    png_ptr: png_structrp,
    owner: png_uint_32,
    finish: c_int,
    /* INPUT: */ input: png_const_bytep,
    input_size_ptr: png_uint_32p,
    /* OUTPUT: */ output: png_bytep,
    output_size_ptr: *mut png_alloc_size_t,
) -> c_int {
    if (*png_ptr).zowner == owner {
        /* Else not claimed */
        let mut ret: c_int;
        let mut avail_out: png_alloc_size_t = *output_size_ptr;
        let mut avail_in: png_uint_32 = *input_size_ptr;

        (*png_ptr).zstream.next_in = PNGZ_INPUT_CAST(input);
        /* avail_in and avail_out are set below from 'size' */
        (*png_ptr).zstream.avail_in = 0;
        (*png_ptr).zstream.avail_out = 0;

        /* Read directly into the output if it is available. */
        if output != core::ptr::null_mut() {
            (*png_ptr).zstream.next_out = output;
        }

        loop {
            let mut avail: uInt;
            let mut local_buffer: [png_byte; PNG_INFLATE_BUF_SIZE] = [0; PNG_INFLATE_BUF_SIZE];

            /* zlib INPUT BUFFER */
            avail_in += (*png_ptr).zstream.avail_in; /* not consumed last time */

            avail = ZLIB_IO_MAX;

            if avail_in < avail {
                avail = avail_in as uInt; /* safe: < than ZLIB_IO_MAX */
            }

            avail_in -= avail;
            (*png_ptr).zstream.avail_in = avail;

            /* zlib OUTPUT BUFFER */
            avail_out += (*png_ptr).zstream.avail_out as png_alloc_size_t; /* not written last time */

            avail = ZLIB_IO_MAX; /* maximum zlib can process */

            if output == core::ptr::null_mut() {
                /* Reset the output buffer each time round if output is NULL. */
                (*png_ptr).zstream.next_out = local_buffer.as_mut_ptr();
                if core::mem::size_of_val(&local_buffer) < avail as usize {
                    avail = core::mem::size_of_val(&local_buffer) as uInt;
                }
            }

            if avail_out < avail as png_alloc_size_t {
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
        if output == core::ptr::null_mut() {
            (*png_ptr).zstream.next_out = core::ptr::null_mut();
        }

        /* Claw back the 'size' and 'remaining_space' byte counts. */
        avail_in += (*png_ptr).zstream.avail_in;
        avail_out += (*png_ptr).zstream.avail_out as png_alloc_size_t;

        /* Update the input and output sizes. */
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
        (*png_ptr).zstream.msg = PNGZ_MSG_CAST(cstr(b"zstream unclaimed\0"));
        Z_STREAM_ERROR
    }
}

/* Decompress trailing data in a chunk. */
pub unsafe extern "C" fn png_decompress_chunk(
    png_ptr: png_structrp,
    chunklength: png_uint_32,
    prefix_size: png_uint_32,
    newlength: *mut png_alloc_size_t, /* must be initialized to the maximum! */
    terminate: c_int,                 /*add a '\0' to the end of the uncompressed data*/
) -> c_int {
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
                &mut lzsize,
                /* output: */ core::ptr::null_mut(),
                newlength,
            );

            if ret == Z_STREAM_END {
                /* Use 'inflateReset' here, not 'inflateReset2'. */
                if inflateReset(&mut (*png_ptr).zstream) == Z_OK {
                    let new_size: png_alloc_size_t = *newlength;
                    let buffer_size: png_alloc_size_t = prefix_size as png_alloc_size_t
                        + new_size
                        + (terminate != 0) as png_alloc_size_t;
                    let mut text: png_bytep = png_malloc_base(png_ptr, buffer_size) as png_bytep;

                    if text != core::ptr::null_mut() {
                        memset(text as *mut c_void, 0, buffer_size);

                        ret = png_inflate(
                            png_ptr,
                            (*png_ptr).chunk_name,
                            1, /*finish*/
                            (*png_ptr).read_buffer.add(prefix_size as usize),
                            &mut lzsize,
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

                        /* This really is very benign. */
                        if ret == Z_STREAM_END && chunklength - prefix_size != lzsize {
                            png_chunk_benign_error(png_ptr, cstr(b"extra compressed data\0"));
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
        } else if ret == Z_STREAM_END {
            /* the claim failed */
            /* impossible! */
            ret = PNG_UNEXPECTED_ZLIB_RETURN;
        }

        ret
    } else {
        /* Application/configuration limits exceeded */
        png_zstream_error(png_ptr, Z_MEM_ERROR);
        Z_MEM_ERROR
    }
}

/* Perform a partial read and decompress, producing 'avail_out' bytes and
 * reading from the current chunk as required.
 */
pub unsafe extern "C" fn png_inflate_read(
    png_ptr: png_structrp,
    read_buffer: png_bytep,
    mut read_size: uInt,
    chunk_bytes: png_uint_32p,
    next_out: png_bytep,
    out_size: *mut png_alloc_size_t,
    finish: c_int,
) -> c_int {
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
                if avail as png_alloc_size_t > *out_size {
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
        (*png_ptr).zstream.msg = PNGZ_MSG_CAST(cstr(b"zstream unclaimed\0"));
        Z_STREAM_ERROR
    }
}
