//! pngrutil.c lines 1-900: integer readers, signature/chunk-header reading,
//! CRC handling, the read buffer and the zlib inflate helpers.
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_uint_31(
    png_ptr: png_const_structrp,
    buf: png_const_bytep,
) -> png_uint_32 {
    let uval: png_uint_32 = png_get_uint_32(buf);

    if uval > PNG_UINT_31_MAX {
        png_error(png_ptr, c"PNG unsigned integer out of range".as_ptr());
    }

    uval
}

/* NOTE: the read macros will obscure these definitions, so that if
 * PNG_USE_READ_MACROS is set the library will not use them internally,
 * but the APIs will still be available externally.
 *
 * The parentheses around "PNGAPI function_name" in the following three
 * functions are necessary because they allow the macros to co-exist with
 * these (unused but exported) functions.
 */

/* Grab an unsigned 32-bit integer from a buffer in big-endian format. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_uint_32(buf: png_const_bytep) -> png_uint_32 {
    let uval: png_uint_32 = ((*(buf) as png_uint_32) << 24)
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
pub unsafe extern "C-unwind" fn png_get_int_32(buf: png_const_bytep) -> png_int_32 {
    let mut uval: png_uint_32 = png_get_uint_32(buf);
    if (uval & 0x80000000) == 0
    /* non-negative */
    {
        return uval as png_int_32;
    }

    uval = (uval ^ 0xffffffff).wrapping_add(1); /* 2's complement: -x = ~x+1 */
    if (uval & 0x80000000) == 0
    /* no overflow */
    {
        return (uval as png_int_32).wrapping_neg();
    }
    /* The following has to be safe; this function only gets called on PNG data
     * and if we get here that data is invalid.  0 is the most safe value and
     * if not then an attacker would surely just generate a PNG with 0 instead.
     */
    0
}

/* Grab an unsigned 16-bit integer from a buffer in big-endian format. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_uint_16(buf: png_const_bytep) -> png_uint_16 {
    /* ANSI-C requires an int value to accommodate at least 16 bits so this
     * works and allows the compiler not to worry about possible narrowing
     * on 32-bit systems.  (Pre-ANSI systems did not make integers smaller
     * than 16 bits either.)
     */
    let val: c_uint = ((*buf as c_uint) << 8).wrapping_add(*(buf.add(1)) as c_uint);

    val as png_uint_16
}

/* Read and check the PNG file signature */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_read_sig(png_ptr: png_structrp, info_ptr: png_inforp) {
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
        (*info_ptr).signature.as_mut_ptr().add(num_checked),
        num_to_check,
    );
    (*png_ptr).sig_bytes = 8;

    if png_sig_cmp(
        (*info_ptr).signature.as_ptr(),
        num_checked,
        num_to_check,
    ) != 0
    {
        if num_checked < 4
            && png_sig_cmp(
                (*info_ptr).signature.as_ptr(),
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

/* This function is called to verify that a chunk name is valid.
 * Do this using the bit-whacking approach from contrib/tools/pngfix.c
 *
 * Copied from libpng 1.7.
 */
pub unsafe fn check_chunk_name(name_in: png_uint_32) -> c_int {
    let mut name: png_uint_32 = name_in;
    let mut t: png_uint_32;

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
    t |= name;

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
    chunk_name = PNG_CHUNK_FROM_STRING(buf.as_ptr().add(4));
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

/* Read data, and (optionally) run it through the CRC. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_crc_read(
    png_ptr: png_structrp,
    buf: png_bytep,
    length: png_uint_32,
) {
    if png_ptr.is_null() {
        return;
    }

    png_read_data(png_ptr, buf, length as usize);
    png_calculate_crc(png_ptr, buf, length as usize);
}

/* Compare the CRC stored in the PNG file with that calculated by libpng from
 * the data it has read thus far.
 */
pub unsafe fn png_crc_error(png_ptr: png_structrp, handle_as_ancillary: c_int) -> c_int {
    let mut crc_bytes: [png_byte; 4] = [0; 4];
    let crc: png_uint_32;
    let mut need_crc: c_int = 1;

    /* There are four flags two for ancillary and two for critical chunks.  The
     * default setting of these flags is all zero.
     *
     * PNG_FLAG_CRC_ANCILLARY_USE
     * PNG_FLAG_CRC_ANCILLARY_NOWARN
     *  USE+NOWARN: no CRC calculation (implemented here), else;
     *  NOWARN:     png_chunk_error on error (implemented in png_crc_finish)
     *  else:       png_chunk_warning on error (implemented in png_crc_finish)
     *              This is the default.
     *
     *    I.e. NOWARN without USE produces png_chunk_error.  The default setting
     *    where neither are set does the same thing.
     *
     * PNG_FLAG_CRC_CRITICAL_USE
     * PNG_FLAG_CRC_CRITICAL_IGNORE
     *  IGNORE: no CRC calculation (implemented here), else;
     *  USE:    png_chunk_warning on error (implemented in png_crc_finish)
     *  else:   png_chunk_error on error (implemented in png_crc_finish)
     *          This is the default.
     *
     * This arose because of original mis-implementation and has persisted for
     * compatibility reasons.
     *
     * TODO: the flag names are internal so maybe this can be changed to
     * something comprehensible.
     */
    if handle_as_ancillary != 0 || PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name) != 0 {
        if ((*png_ptr).flags & PNG_FLAG_CRC_ANCILLARY_MASK)
            == (PNG_FLAG_CRC_ANCILLARY_USE | PNG_FLAG_CRC_ANCILLARY_NOWARN)
        {
            need_crc = 0;
        }
    }
    /* critical */
    else {
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

/* Optionally skip data and then check the CRC.  Depending on whether we
 * are reading an ancillary or critical chunk, and how the program has set
 * things up, we may calculate the CRC on the data and print a message.
 * Returns '1' if there was a CRC error, '0' otherwise.
 *
 * There is one public version which is used in most places and another which
 * takes the value for the 'critical' flag to check.  This allows PLTE and IEND
 * handling code to ignore the CRC error and removes some confusing code
 * duplication.
 */
pub unsafe fn png_crc_finish_critical(
    png_ptr: png_structrp,
    mut skip: png_uint_32,
    mut handle_as_ancillary: c_int,
) -> c_int {
    /* The size of the local buffer for inflate is a good guess as to a
     * reasonable size to use for buffering reads from the application.
     */
    while skip > 0 {
        let mut len: png_uint_32;
        let mut tmpbuf: [png_byte; PNG_INFLATE_BUF_SIZE] = [0; PNG_INFLATE_BUF_SIZE];

        len = PNG_INFLATE_BUF_SIZE as png_uint_32;
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

    /* TODO: this might be more comprehensible if png_crc_error was inlined here.
     */
    if png_crc_error(png_ptr, handle_as_ancillary) != 0 {
        /* See above for the explanation of how the flags work. */
        if if handle_as_ancillary != 0 || PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name) != 0 {
            ((*png_ptr).flags & PNG_FLAG_CRC_ANCILLARY_NOWARN) == 0
        } else {
            ((*png_ptr).flags & PNG_FLAG_CRC_CRITICAL_USE) != 0
        } {
            png_chunk_warning(png_ptr, c"CRC error".as_ptr());
        } else {
            png_chunk_error(png_ptr, c"CRC error".as_ptr());
        }

        return 1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_crc_finish(
    png_ptr: png_structrp,
    skip: png_uint_32,
) -> c_int {
    png_crc_finish_critical(png_ptr, skip, 0 /*critical handling*/)
}

/* Manage the read buffer; this simply reallocates the buffer if it is not small
 * enough (or if it is not allocated).  The routine returns a pointer to the
 * buffer; if an error occurs and 'warn' is set the routine returns NULL, else
 * it will call png_error on failure.
 */
pub unsafe fn png_read_buffer(png_ptr: png_structrp, new_size: png_alloc_size_t) -> png_bytep {
    let mut buffer: png_bytep = (*png_ptr).read_buffer;

    if new_size > (*png_ptr).user_chunk_malloc_max {
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
            memset(buffer, 0, new_size); /* just in case */
            (*png_ptr).read_buffer = buffer;
            (*png_ptr).read_buffer_size = new_size;
        }
    }

    buffer
}

/* png_inflate_claim: claim the zstream for some nefarious purpose that involves
 * decompression.  Returns Z_OK on success, else a zlib error code.  It checks
 * the owner but, in final release builds, just issues a warning if some other
 * chunk apparently owns the stream.  Prior to release it does a png_error.
 */
pub unsafe fn png_inflate_claim(png_ptr: png_structrp, owner: png_uint_32) -> c_int {
    if (*png_ptr).zowner != 0 {
        let mut msg: [c_char; 64] = [0; 64];

        PNG_STRING_FROM_CHUNK(msg.as_mut_ptr() as *mut png_byte, (*png_ptr).zowner);
        /* So the message that results is "<chunk> using zstream"; this is an
         * internal error, but is very useful for debugging.  i18n requirements
         * are minimal.
         */
        png_safecat(msg.as_mut_ptr(), 64, 4, c" using zstream".as_ptr());
        png_chunk_error(png_ptr, msg.as_ptr());
    }

    /* Implementation note: unlike 'png_deflate_claim' this internal function
     * does not take the size of the data as an argument.  Some efficiency could
     * be gained by using this when it is known *if* the zlib stream itself does
     * not record the number; however, this is an illusion: the original writer
     * of the PNG may have selected a lower window size, and we really must
     * follow that because, for systems with limited capabilities, we
     * would otherwise reject the application's attempts to use a smaller window
     * size (zlib doesn't have an interface to say "this or lower"!).
     *
     * inflateReset2 was added to zlib 1.2.4; before this the window could not be
     * reset, therefore it is necessary to always allocate the maximum window
     * size with earlier zlibs just in case later compressed chunks need it.
     */
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

        return ret;
    }
}

/* Handle the start of the inflate stream if we called inflateInit2(strm,0);
 * in this case some zlib versions skip validation of the CINFO field and, in
 * certain circumstances, libpng may end up displaying an invalid image, in
 * contrast to implementations that call zlib in the normal way (e.g. libpng
 * 1.5).
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_zlib_inflate(png_ptr: png_structrp, flush: c_int) -> c_int {
    if (*png_ptr).zstream_start != 0 && (*png_ptr).zstream.avail_in > 0 {
        if (*(*png_ptr).zstream.next_in >> 4) > 7 {
            (*png_ptr).zstream.msg = c"invalid window size (libpng)".as_ptr();
            return Z_DATA_ERROR;
        }

        (*png_ptr).zstream_start = 0;
    }

    inflate(&mut (*png_ptr).zstream, flush)
}

/* png_inflate now returns zlib error codes including Z_OK and Z_STREAM_END to
 * allow the caller to do multiple calls if required.  If the 'finish' flag is
 * set Z_FINISH will be passed to the final inflate() call and Z_STREAM_END must
 * be returned or there has been a problem, otherwise Z_SYNC_FLUSH is used and
 * Z_OK or Z_STREAM_END will be returned on success.
 *
 * The input and output sizes are updated to the actual amounts of data consumed
 * or written, not the amount available (as in a z_stream).  The data pointers
 * are not changed, so the next input is (data+input_size) and the next
 * available output is (output+output_size).
 */
pub unsafe fn png_inflate(
    png_ptr: png_structrp,
    owner: png_uint_32,
    finish: c_int,
    /* INPUT: */ input: png_const_bytep,
    input_size_ptr: png_uint_32p,
    /* OUTPUT: */ output: png_bytep,
    output_size_ptr: *mut png_alloc_size_t,
) -> c_int {
    if (*png_ptr).zowner == owner
    /* Else not claimed */
    {
        let mut ret: c_int = Z_OK;
        let mut avail_out: png_alloc_size_t = *output_size_ptr;
        let mut avail_in: png_uint_32 = *input_size_ptr;

        /* zlib can't necessarily handle more than 65535 bytes at once (i.e. it
         * can't even necessarily handle 65536 bytes) because the type uInt is
         * "16 bits or more".  Consequently it is necessary to chunk the input to
         * zlib.  This code uses ZLIB_IO_MAX, from pngpriv.h, as the maximum (the
         * maximum value that can be stored in a uInt.)  It is possible to set
         * ZLIB_IO_MAX to a lower value in pngpriv.h and this may sometimes have
         * a performance advantage, because it reduces the amount of data accessed
         * at each step and that may give the OS more time to page it in.
         */
        (*png_ptr).zstream.next_in = input as *const u8;
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
            let mut local_buffer: [png_byte; PNG_INFLATE_BUF_SIZE] = [0; PNG_INFLATE_BUF_SIZE];

            /* zlib INPUT BUFFER */
            /* The setting of 'avail_in' used to be outside the loop; by setting it
             * inside it is possible to chunk the input to zlib and simply rely on
             * zlib to advance the 'next_in' pointer.  This allows arbitrary
             * amounts of data to be passed through zlib at the unavoidable cost of
             * requiring a window save (memcpy of up to 32768 output bytes)
             * every ZLIB_IO_MAX input bytes.
             */
            avail_in = avail_in.wrapping_add((*png_ptr).zstream.avail_in); /* not consumed last time */

            avail = crate::zlib::ZLIB_IO_MAX;

            if avail_in < avail {
                avail = avail_in as uInt; /* safe: < than ZLIB_IO_MAX */
            }

            avail_in = avail_in.wrapping_sub(avail);
            (*png_ptr).zstream.avail_in = avail;

            /* zlib OUTPUT BUFFER */
            avail_out = avail_out.wrapping_add((*png_ptr).zstream.avail_out as png_alloc_size_t); /* not written last time */

            avail = crate::zlib::ZLIB_IO_MAX; /* maximum zlib can process */

            if output.is_null() {
                /* Reset the output buffer each time round if output is NULL and
                 * make available the full buffer, up to 'remaining_space'
                 */
                (*png_ptr).zstream.next_out = local_buffer.as_mut_ptr();
                if PNG_INFLATE_BUF_SIZE < avail as usize {
                    avail = PNG_INFLATE_BUF_SIZE as uInt;
                }
            }

            if avail_out < avail as png_alloc_size_t {
                avail = avail_out as uInt; /* safe: < ZLIB_IO_MAX */
            }

            (*png_ptr).zstream.avail_out = avail;
            avail_out = avail_out.wrapping_sub(avail as png_alloc_size_t);

            /* zlib inflate call */
            /* In fact 'avail_out' may be 0 at this point, that happens at the end
             * of the read when the final LZ end code was not passed at the end of
             * the previous chunk of input data.  Tell zlib if we have reached the
             * end of the output buffer.
             */
            ret = png_zlib_inflate(
                png_ptr,
                if avail_out > 0 {
                    Z_NO_FLUSH
                } else if finish != 0 {
                    Z_FINISH
                } else {
                    Z_SYNC_FLUSH
                },
            );

            if !(ret == Z_OK) {
                break;
            }
        }

        /* For safety kill the local buffer pointer now */
        if output.is_null() {
            (*png_ptr).zstream.next_out = core::ptr::null_mut();
        }

        /* Claw back the 'size' and 'remaining_space' byte counts. */
        avail_in = avail_in.wrapping_add((*png_ptr).zstream.avail_in);
        avail_out = avail_out.wrapping_add((*png_ptr).zstream.avail_out as png_alloc_size_t);

        /* Update the input and output sizes; the updated values are the amount
         * consumed or written, effectively the inverse of what zlib uses.
         */
        if avail_out > 0 {
            *output_size_ptr = (*output_size_ptr).wrapping_sub(avail_out);
        }

        if avail_in > 0 {
            *input_size_ptr = (*input_size_ptr).wrapping_sub(avail_in);
        }

        /* Ensure png_ptr->zstream.msg is set (even in the success case!) */
        png_zstream_error(png_ptr, ret);
        ret
    } else {
        /* This is a bad internal error.  The recovery assigns to the zstream msg
         * pointer, which is not owned by the caller, but this is safe; it's only
         * used on errors!
         */
        (*png_ptr).zstream.msg = c"zstream unclaimed".as_ptr();
        Z_STREAM_ERROR
    }
}

/*
 * Decompress trailing data in a chunk.  The assumption is that read_buffer
 * points at an allocated area holding the contents of a chunk with a
 * trailing compressed part.  What we get back is an allocated area
 * holding the original prefix part and an uncompressed version of the
 * trailing part (the malloc area passed in is freed).
 */
pub unsafe fn png_decompress_chunk(
    png_ptr: png_structrp,
    chunklength: png_uint_32,
    prefix_size: png_uint_32,
    newlength: *mut png_alloc_size_t, /* must be initialized to the maximum! */
    terminate: c_int,                 /*add a '\0' to the end of the uncompressed data*/
) -> c_int {
    /* TODO: implement different limits for different types of chunk.
     *
     * The caller supplies *newlength set to the maximum length of the
     * uncompressed data, but this routine allocates space for the prefix and
     * maybe a '\0' terminator too.  We have to assume that 'prefix_size' is
     * limited only by the maximum chunk size.
     */
    let mut limit: png_alloc_size_t = (*png_ptr).user_chunk_malloc_max;

    /* prefix_size + (terminate != 0) evaluated in 'unsigned int' arithmetic */
    let prefix_and_terminator: png_uint_32 =
        prefix_size.wrapping_add(if terminate != 0 { 1 } else { 0 });

    if limit >= prefix_and_terminator as png_alloc_size_t {
        let mut ret: c_int;

        limit = limit.wrapping_sub(prefix_and_terminator as png_alloc_size_t);

        if limit < *newlength {
            *newlength = limit;
        }

        /* Now try to claim the stream. */
        ret = png_inflate_claim(png_ptr, (*png_ptr).chunk_name);

        if ret == Z_OK {
            let mut lzsize: png_uint_32 = chunklength.wrapping_sub(prefix_size);

            ret = png_inflate(
                png_ptr,
                (*png_ptr).chunk_name,
                1, /*finish*/
                /* input: */
                (*png_ptr).read_buffer.add(prefix_size as usize) as png_const_bytep,
                &mut lzsize,
                /* output: */ core::ptr::null_mut(),
                newlength,
            );

            if ret == Z_STREAM_END {
                /* Use 'inflateReset' here, not 'inflateReset2' because this
                 * preserves the previously decided window size (otherwise it would
                 * be necessary to store the previous window size.)  In practice
                 * this doesn't matter anyway, because png_inflate will call inflate
                 * with Z_FINISH in almost all cases, so the window will not be
                 * maintained.
                 */
                if inflateReset(&mut (*png_ptr).zstream) == Z_OK {
                    /* Because of the limit checks above we know that the new,
                     * expanded, size will fit in a size_t (let alone an
                     * png_alloc_size_t).  Use png_malloc_base here to avoid an
                     * extra OOM message.
                     */
                    let new_size: png_alloc_size_t = *newlength;
                    let buffer_size: png_alloc_size_t = (prefix_size as png_alloc_size_t)
                        .wrapping_add(new_size)
                        .wrapping_add(if terminate != 0 { 1 } else { 0 });
                    let mut text: png_bytep = png_malloc_base(png_ptr, buffer_size) as png_bytep;

                    if !text.is_null() {
                        memset(text, 0, buffer_size);

                        ret = png_inflate(
                            png_ptr,
                            (*png_ptr).chunk_name,
                            1, /*finish*/
                            (*png_ptr).read_buffer.add(prefix_size as usize) as png_const_bytep,
                            &mut lzsize,
                            text.add(prefix_size as usize),
                            newlength,
                        );

                        if ret == Z_STREAM_END {
                            if new_size == *newlength {
                                if terminate != 0 {
                                    *text.add((prefix_size as usize).wrapping_add(*newlength)) = 0;
                                }

                                if prefix_size > 0 {
                                    memcpy(
                                        text,
                                        (*png_ptr).read_buffer,
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
                                /* The size changed on the second read, there can be no
                                 * guarantee that anything is correct at this point.
                                 * The 'msg' pointer has been set to "unexpected end of
                                 * LZ stream", which is fine, but return an error code
                                 * that the caller won't accept.
                                 */
                                ret = PNG_UNEXPECTED_ZLIB_RETURN;
                            }
                        } else if ret == Z_OK {
                            ret = PNG_UNEXPECTED_ZLIB_RETURN; /* for safety */
                        }

                        /* Free the text pointer (this is the old read_buffer on
                         * success)
                         */
                        png_free(png_ptr, text as png_voidp);

                        /* This really is very benign, but it's still an error because
                         * the extra space may otherwise be used as a Trojan Horse.
                         */
                        if ret == Z_STREAM_END && chunklength.wrapping_sub(prefix_size) != lzsize {
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
        }
        /* the claim failed */
        else if ret == Z_STREAM_END
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

/* Perform a partial read and decompress, producing 'avail_out' bytes and
 * reading from the current chunk as required.
 */
pub unsafe fn png_inflate_read(
    png_ptr: png_structrp,
    read_buffer: png_bytep,
    mut read_size: uInt,
    chunk_bytes: png_uint_32p,
    next_out: png_bytep,
    out_size: *mut png_alloc_size_t,
    finish: c_int,
) -> c_int {
    if (*png_ptr).zowner == (*png_ptr).chunk_name {
        let mut ret: c_int = Z_OK;

        /* next_in and avail_in must have been initialized by the caller. */
        (*png_ptr).zstream.next_out = next_out;
        (*png_ptr).zstream.avail_out = 0; /* set in the loop */

        loop {
            if (*png_ptr).zstream.avail_in == 0 {
                if read_size > *chunk_bytes {
                    read_size = *chunk_bytes as uInt;
                }
                *chunk_bytes = (*chunk_bytes).wrapping_sub(read_size);

                if read_size > 0 {
                    png_crc_read(png_ptr, read_buffer, read_size as png_uint_32);
                }

                (*png_ptr).zstream.next_in = read_buffer;
                (*png_ptr).zstream.avail_in = read_size;
            }

            if (*png_ptr).zstream.avail_out == 0 {
                let mut avail: uInt = crate::zlib::ZLIB_IO_MAX;
                if avail as png_alloc_size_t > *out_size {
                    avail = *out_size as uInt;
                }
                *out_size = (*out_size).wrapping_sub(avail as png_alloc_size_t);

                (*png_ptr).zstream.avail_out = avail;
            }

            /* Use Z_SYNC_FLUSH when there is no more chunk data to ensure that all
             * the available output is produced; this allows reading of truncated
             * streams.
             */
            ret = png_zlib_inflate(
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

        *out_size = (*out_size).wrapping_add((*png_ptr).zstream.avail_out as png_alloc_size_t);
        (*png_ptr).zstream.avail_out = 0; /* Should not be required, but is safe */

        /* Ensure the error message pointer is always set: */
        png_zstream_error(png_ptr, ret);
        ret
    } else {
        (*png_ptr).zstream.msg = c"zstream unclaimed".as_ptr();
        Z_STREAM_ERROR
    }
}
