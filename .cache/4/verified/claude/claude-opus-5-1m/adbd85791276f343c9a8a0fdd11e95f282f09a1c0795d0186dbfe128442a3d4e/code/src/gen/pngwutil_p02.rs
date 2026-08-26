/* pngwutil.c lines 309..688 */

/* Initialize the compressor for the appropriate type of compression. */
/* png_deflate_claim (static) */
unsafe fn png_deflate_claim(
    png_ptr: png_structrp,
    owner: png_uint_32,
    data_size: png_alloc_size_t,
) -> c_int {
    if (*png_ptr).zowner != 0 {
        /* defined(PNG_WARNINGS_SUPPORTED) || defined(PNG_ERROR_TEXT_SUPPORTED) */
        let mut msg: [c_char; 64] = [0; 64];

        PNG_STRING_FROM_CHUNK(msg.as_mut_ptr() as png_bytep, owner);
        msg[4] = b':' as c_char;
        msg[5] = b' ' as c_char;
        PNG_STRING_FROM_CHUNK(msg.as_mut_ptr().add(6) as png_bytep, (*png_ptr).zowner);
        /* So the message that results is "<chunk> using zstream"; this is an
         * internal error, but is very useful for debugging.  i18n requirements
         * are minimal.
         */
        png_safecat(
            msg.as_mut_ptr(),
            64,
            10,
            b" using zstream\0".as_ptr() as png_const_charp,
        );
        /* !PNG_RELEASE_BUILD */
        png_error(png_ptr, msg.as_ptr() as png_const_charp);
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

            while data_size.wrapping_add(262) <= half_window_size as png_alloc_size_t {
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
            if deflateEnd(core::ptr::addr_of_mut!((*png_ptr).zstream)) != Z_OK {
                png_warning(
                    png_ptr,
                    b"deflateEnd failed (ignored)\0".as_ptr() as png_const_charp,
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
            ret = deflateReset(core::ptr::addr_of_mut!((*png_ptr).zstream));
        } else {
            ret = deflateInit2(
                core::ptr::addr_of_mut!((*png_ptr).zstream),
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
/* png_free_buffer_list */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_buffer_list(
    png_ptr: png_structrp,
    listp: *mut png_compression_bufferp,
) {
    let mut list: png_compression_bufferp = *listp;

    if list != core::ptr::null_mut() {
        *listp = core::ptr::null_mut();

        loop {
            let next: png_compression_bufferp = (*list).next;

            png_free(png_ptr, list as png_voidp);
            list = next;

            if !(list != core::ptr::null_mut()) {
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
/* png_text_compress_init (static) */
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
/* png_text_compress (static) */
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
        let mut end: *mut png_compression_bufferp =
            core::ptr::addr_of_mut!((*png_ptr).zbuffer_list);
        let mut input_len: png_alloc_size_t = (*comp).input_len; /* may be zero! */
        let mut output_len: png_uint_32;

        /* zlib updates these for us: */
        (*png_ptr).zstream.next_in = (*comp).input as *const Bytef;
        (*png_ptr).zstream.avail_in = 0; /* Set below */
        (*png_ptr).zstream.next_out = (*comp).output.as_mut_ptr();
        (*png_ptr).zstream.avail_out = 1024; /* (sizeof comp->output) */

        output_len = (*png_ptr).zstream.avail_out;

        loop {
            let mut avail_in: uInt = ZLIB_IO_MAX;

            if avail_in as png_alloc_size_t > input_len {
                avail_in = input_len as uInt;
            }

            input_len = input_len.wrapping_sub(avail_in as png_alloc_size_t);

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
                if next == core::ptr::null_mut() {
                    next = png_malloc_base(
                        png_ptr,
                        PNG_COMPRESSION_BUFFER_SIZE(png_ptr) as png_alloc_size_t,
                    ) as png_compression_bufferp;

                    if next == core::ptr::null_mut() {
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
                end = core::ptr::addr_of_mut!((*next).next);
            }

            /* Compress the data */
            ret = deflate(
                core::ptr::addr_of_mut!((*png_ptr).zstream),
                if input_len > 0 { Z_NO_FLUSH } else { Z_FINISH },
            );

            /* Claw back input data that was not consumed (because avail_in is
             * reset above every time round the loop).
             */
            input_len = input_len.wrapping_add((*png_ptr).zstream.avail_in as png_alloc_size_t);
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
            (*png_ptr).zstream.msg = b"compressed data too long\0".as_ptr() as *const c_char;
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
            /* PNG_WRITE_OPTIMIZE_CMF_SUPPORTED */
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
/* png_write_compressed_data_out (static) */
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

        output_len = output_len.wrapping_sub(avail);

        if output_len == 0 || next == core::ptr::null_mut() {
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
            b"error writing ancillary chunked compressed data\0".as_ptr() as png_const_charp,
        );
    }
}
