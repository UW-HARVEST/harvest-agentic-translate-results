/* pngrutil.c lines 1..375 */

/* png_get_uint_31 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_uint_31(
    png_ptr: png_const_structrp,
    buf: png_const_bytep,
) -> png_uint_32 {
    let uval: png_uint_32 = PNG_get_uint_32(buf);

    if uval > PNG_UINT_31_MAX {
        png_error(
            png_ptr,
            b"PNG unsigned integer out of range\0".as_ptr() as png_const_charp,
        );
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
/* png_get_uint_32 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_uint_32(buf: png_const_bytep) -> png_uint_32 {
    let uval: png_uint_32 = ((*buf as png_uint_32) << 24)
        .wrapping_add((*buf.add(1) as png_uint_32) << 16)
        .wrapping_add((*buf.add(2) as png_uint_32) << 8)
        .wrapping_add(*buf.add(3) as png_uint_32);

    uval
}

/* Grab a signed 32-bit integer from a buffer in big-endian format.  The
 * data is stored in the PNG file in two's complement format and there
 * is no guarantee that a 'png_int_32' is exactly 32 bits, therefore
 * the following code does a two's complement to native conversion.
 */
/* png_get_int_32 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_int_32(buf: png_const_bytep) -> png_int_32 {
    let mut uval: png_uint_32 = PNG_get_uint_32(buf);
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
/* png_get_uint_16 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_uint_16(buf: png_const_bytep) -> png_uint_16 {
    /* ANSI-C requires an int value to accommodate at least 16 bits so this
     * works and allows the compiler not to worry about possible narrowing
     * on 32-bit systems.  (Pre-ANSI systems did not make integers smaller
     * than 16 bits either.)
     */
    let val: c_uint = ((*buf as c_uint) << 8).wrapping_add(*buf.add(1) as c_uint);

    val as png_uint_16
}

/* Read and check the PNG file signature */
/* png_read_sig */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_sig(png_ptr: png_structrp, info_ptr: png_inforp) {
    let num_checked: usize;
    let num_to_check: usize;

    /* Exit if the user application does not expect a signature. */
    if (*png_ptr).sig_bytes >= 8 {
        return;
    }

    num_checked = (*png_ptr).sig_bytes as usize;
    num_to_check = 8usize.wrapping_sub(num_checked);

    (*png_ptr).io_state = PNG_IO_READING | PNG_IO_SIGNATURE;

    /* The signature must be serialized in a single I/O call. */
    png_read_data(
        png_ptr,
        (core::ptr::addr_of_mut!((*info_ptr).signature) as png_bytep).add(num_checked),
        num_to_check,
    );
    (*png_ptr).sig_bytes = 8;

    if png_sig_cmp(
        core::ptr::addr_of!((*info_ptr).signature) as png_const_bytep,
        num_checked,
        num_to_check,
    ) != 0
    {
        if num_checked < 4
            && png_sig_cmp(
                core::ptr::addr_of!((*info_ptr).signature) as png_const_bytep,
                num_checked,
                num_to_check.wrapping_sub(4),
            ) != 0
        {
            png_error(png_ptr, b"Not a PNG file\0".as_ptr() as png_const_charp);
        } else {
            png_error(
                png_ptr,
                b"PNG file corrupted by ASCII conversion\0".as_ptr() as png_const_charp,
            );
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
unsafe fn check_chunk_name(mut name: png_uint_32) -> c_int {
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
/* png_read_chunk_header */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_chunk_header(png_ptr: png_structrp) -> png_uint_32 {
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
        png_chunk_error(
            png_ptr,
            b"bad header (invalid length)\0".as_ptr() as png_const_charp,
        );
    }

    /* Check to see if chunk name is valid. */
    if check_chunk_name(chunk_name) == 0 {
        png_chunk_error(
            png_ptr,
            b"bad header (invalid type)\0".as_ptr() as png_const_charp,
        );
    }

    (*png_ptr).io_state = PNG_IO_READING | PNG_IO_CHUNK_DATA;

    length
}

/* Read data, and (optionally) run it through the CRC. */
/* png_crc_read */
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
unsafe fn png_crc_error(png_ptr: png_structrp, handle_as_ancillary: c_int) -> c_int {
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
        crc = PNG_get_uint_32(crc_bytes.as_ptr());
        return (crc != (*png_ptr).crc) as c_int;
    } else {
        return 0;
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
unsafe fn png_crc_finish_critical(
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
            png_chunk_warning(png_ptr, b"CRC error\0".as_ptr() as png_const_charp);
        } else {
            png_chunk_error(png_ptr, b"CRC error\0".as_ptr() as png_const_charp);
        }

        return 1;
    }

    0
}

/* png_crc_finish */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_crc_finish(png_ptr: png_structrp, skip: png_uint_32) -> c_int {
    png_crc_finish_critical(png_ptr, skip, 0 /*critical handling*/)
}
