// pngrutil.c - utilities to read a PNG file
//
// This file contains routines that are only called from within
// libpng itself during the course of reading an image.
//
// Chunk 2: png_inflate_claim .. png_inflate_read

use crate::*;

/* png_inflate_claim: claim the zstream for some nefarious purpose that involves
 * decompression.  Returns Z_OK on success, else a zlib error code.  It checks
 * the owner but, in final release builds, just issues a warning if some other
 * chunk apparently owns the stream.  Prior to release it does a png_error.
 */
unsafe fn png_inflate_claim(png_ptr: png_structrp, owner: png_uint_32) -> c_int {
    if (*png_ptr).zowner != 0 {
        let mut msg: [c_char; 64] = [0; 64];

        PNG_STRING_FROM_CHUNK(msg.as_mut_ptr(), (*png_ptr).zowner);
        /* So the message that results is "<chunk> using zstream"; this is an
         * internal error, but is very useful for debugging.  i18n requirements
         * are minimal.
         */
        png_safecat(
            msg.as_mut_ptr(),
            core::mem::size_of_val(&msg),
            4,
            cstr!(" using zstream"),
        );

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
        let ret: c_int; /* zlib return code */

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
            ret = inflateReset2(core::ptr::addr_of_mut!((*png_ptr).zstream), window_bits);
        } else {
            ret = inflateInit2(core::ptr::addr_of_mut!((*png_ptr).zstream), window_bits);

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
pub unsafe extern "C" fn png_zlib_inflate(png_ptr: png_structrp, flush: c_int) -> c_int {
    if (*png_ptr).zstream_start != 0 && (*png_ptr).zstream.avail_in > 0 {
        if ((*(*png_ptr).zstream.next_in as c_int) >> 4) > 7 {
            (*png_ptr).zstream.msg = cstr!("invalid window size (libpng)");
            return Z_DATA_ERROR;
        }

        (*png_ptr).zstream_start = 0;
    }

    inflate(core::ptr::addr_of_mut!((*png_ptr).zstream), flush)
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
unsafe fn png_inflate(
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

        /* NOTE: in C 'local_buffer' is declared inside the loop below; because
         * 'next_out' is reassigned to it on every iteration this is exactly
         * equivalent to hoisting the (single, reused) stack buffer out here.
         */
        let mut local_buffer: [png_byte; PNG_INFLATE_BUF_SIZE] = [0; PNG_INFLATE_BUF_SIZE];

        loop {
            let mut avail: uInt;

            /* zlib INPUT BUFFER */
            /* The setting of 'avail_in' used to be outside the loop; by setting it
             * inside it is possible to chunk the input to zlib and simply rely on
             * zlib to advance the 'next_in' pointer.  This allows arbitrary
             * amounts of data to be passed through zlib at the unavoidable cost of
             * requiring a window save (memcpy of up to 32768 output bytes)
             * every ZLIB_IO_MAX input bytes.
             */
            avail_in = avail_in.wrapping_add((*png_ptr).zstream.avail_in); /* not consumed last time */

            avail = uInt::MAX; /* ZLIB_IO_MAX */

            if avail_in < avail {
                avail = avail_in as uInt; /* safe: < than ZLIB_IO_MAX */
            }

            avail_in = avail_in.wrapping_sub(avail);
            (*png_ptr).zstream.avail_in = avail;

            /* zlib OUTPUT BUFFER */
            avail_out = avail_out.wrapping_add((*png_ptr).zstream.avail_out as png_alloc_size_t); /* not written last time */

            avail = uInt::MAX; /* maximum zlib can process */

            if output.is_null() {
                /* Reset the output buffer each time round if output is NULL and
                 * make available the full buffer, up to 'remaining_space'
                 */
                (*png_ptr).zstream.next_out = local_buffer.as_mut_ptr();
                if (core::mem::size_of_val(&local_buffer) as png_alloc_size_t)
                    < avail as png_alloc_size_t
                {
                    avail = core::mem::size_of_val(&local_buffer) as uInt;
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
        return ret;
    } else {
        /* This is a bad internal error.  The recovery assigns to the zstream msg
         * pointer, which is not owned by the caller, but this is safe; it's only
         * used on errors!
         */
        (*png_ptr).zstream.msg = cstr!("zstream unclaimed");
        return Z_STREAM_ERROR;
    }
}

/*
 * Decompress trailing data in a chunk.  The assumption is that read_buffer
 * points at an allocated area holding the contents of a chunk with a
 * trailing compressed part.  What we get back is an allocated area
 * holding the original prefix part and an uncompressed version of the
 * trailing part (the malloc area passed in is freed).
 */
unsafe fn png_decompress_chunk(
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
    let mut limit: png_alloc_size_t = png_chunk_max(png_ptr);

    /* 'prefix_size + (terminate != 0)' is computed in 'unsigned int' in C */
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
                (*png_ptr).read_buffer.offset(prefix_size as isize),
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
                if inflateReset(core::ptr::addr_of_mut!((*png_ptr).zstream)) == Z_OK {
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
                        memset(text as *mut c_void, 0, buffer_size);

                        ret = png_inflate(
                            png_ptr,
                            (*png_ptr).chunk_name,
                            1, /*finish*/
                            (*png_ptr).read_buffer.offset(prefix_size as isize),
                            &mut lzsize,
                            text.offset(prefix_size as isize),
                            newlength,
                        );

                        if ret == Z_STREAM_END {
                            if new_size == *newlength {
                                if terminate != 0 {
                                    *text.add(
                                        (prefix_size as png_alloc_size_t).wrapping_add(*newlength),
                                    ) = 0;
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
                            png_chunk_benign_error(png_ptr, cstr!("extra compressed data"));
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

        return ret;
    } else {
        /* Application/configuration limits exceeded */
        png_zstream_error(png_ptr, Z_MEM_ERROR);
        return Z_MEM_ERROR;
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
                let mut avail: uInt = uInt::MAX; /* ZLIB_IO_MAX */
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
        return ret;
    } else {
        (*png_ptr).zstream.msg = cstr!("zstream unclaimed");
        return Z_STREAM_ERROR;
    }
}
