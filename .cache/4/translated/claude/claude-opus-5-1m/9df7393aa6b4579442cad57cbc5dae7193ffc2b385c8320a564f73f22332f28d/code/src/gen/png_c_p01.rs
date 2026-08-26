/* png.c lines 1..690 */

/* Tells libpng that we have already handled the first "num_bytes" bytes
 * of the PNG file signature.  If the PNG data is embedded into another
 * stream we can set num_bytes = 8 so that libpng will not attempt to read
 * or write any of the magic bytes before it starts on the IHDR.
 */

/* png_set_sig_bytes */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sig_bytes(png_ptr: png_structrp, num_bytes: c_int) {
    let mut nb: c_uint = num_bytes as c_uint;

    if png_ptr == core::ptr::null_mut() {
        return;
    }

    if num_bytes < 0 {
        nb = 0;
    }

    if nb > 8 {
        png_error(
            png_ptr,
            b"Too many bytes for PNG signature\0".as_ptr() as png_const_charp,
        );
    }

    (*png_ptr).sig_bytes = nb as png_byte;
}

/* `static const png_byte png_signature[8]` from png_sig_cmp */
static png_signature: [png_byte; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/* Checks whether the supplied bytes match the PNG signature.  We allow
 * checking less than the full 8-byte signature so that those apps that
 * already read the first few bytes of a file to determine the file type
 * can simply check the remaining bytes for extra assurance.  Returns
 * an integer less than, equal to, or greater than zero if sig is found,
 * respectively, to be less than, to match, or be greater than the correct
 * PNG signature (this is the same behavior as strcmp, memcmp, etc).
 */
/* png_sig_cmp */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_sig_cmp(
    sig: png_const_bytep,
    start: usize,
    mut num_to_check: usize,
) -> c_int {
    if num_to_check > 8 {
        num_to_check = 8;
    } else if num_to_check < 1 {
        return -1;
    }

    if start > 7 {
        return -1;
    }

    if start + num_to_check > 8 {
        num_to_check = 8 - start;
    }

    memcmp(
        sig.add(start) as *const c_void,
        png_signature.as_ptr().add(start) as *const c_void,
        num_to_check,
    )
}

/* Function to allocate memory for zlib */
/* png_zalloc */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_zalloc(png_ptr: voidpf, items: uInt, size: uInt) -> voidpf {
    let mut num_bytes: png_alloc_size_t = size as png_alloc_size_t;

    if png_ptr == core::ptr::null_mut() {
        return core::ptr::null_mut();
    }

    /* This check against overflow is vestigial, dating back from
     * the old times when png_zalloc used to be an exported function.
     * We're still keeping it here for now, as an extra-cautious
     * prevention against programming errors inside zlib, although it
     * should rather be a debug-time assertion instead.
     */
    if size != 0
        && (items as png_alloc_size_t)
            >= ((!(0 as png_alloc_size_t)) / (size as png_alloc_size_t))
    {
        png_warning(
            png_ptr as png_structrp,
            b"Potential overflow in png_zalloc()\0".as_ptr() as png_const_charp,
        );
        return core::ptr::null_mut();
    }

    num_bytes = num_bytes.wrapping_mul(items as png_alloc_size_t);
    png_malloc_warn(png_ptr as png_structrp, num_bytes)
}

/* Function to free memory for zlib */
/* png_zfree */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_zfree(png_ptr: voidpf, ptr: voidpf) {
    png_free(png_ptr as png_const_structrp, ptr);
}

/* Reset the CRC variable to 32 bits of 1's.  Care must be taken
 * in case CRC is > 32 bits to leave the top bits 0.
 */
/* png_reset_crc */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_reset_crc(png_ptr: png_structrp) {
    /* The cast is safe because the crc is a 32-bit value. */
    (*png_ptr).crc = crc32(0, core::ptr::null(), 0) as png_uint_32;
}

/* Calculate the CRC over a section of data.  We can only pass as
 * much data to this routine as the largest single buffer size.  We
 * also check that this data will actually be used before going to the
 * trouble of calculating it.
 */
/* png_calculate_crc */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_calculate_crc(
    png_ptr: png_structrp,
    mut ptr: png_const_bytep,
    mut length: usize,
) {
    let mut need_crc: c_int = 1;

    if PNG_CHUNK_ANCILLARY((*png_ptr).chunk_name) != 0 {
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

    /* 'uLong' is defined in zlib.h as unsigned long; this means that on some
     * systems it is a 64-bit value.  crc32, however, returns 32 bits so the
     * following cast is safe.  'uInt' may be no more than 16 bits, so it is
     * necessary to perform a loop here.
     */
    if need_crc != 0 && length > 0 {
        let mut crc: uLong = (*png_ptr).crc as uLong; /* Should never issue a warning */

        loop {
            let mut safe_length: uInt = length as uInt;
            if safe_length == 0 {
                safe_length = (-1i32) as uInt; /* evil, but safe */
            }

            crc = crc32(crc, ptr as *const Bytef, safe_length);

            /* The following should never issue compiler warnings; if they do the
             * target system has characteristics that will probably violate other
             * assumptions within the libpng code.
             */
            ptr = ptr.add(safe_length as usize);
            length -= safe_length as usize;

            if !(length > 0) {
                break;
            }
        }

        /* And the following is always safe because the crc is only 32 bits. */
        (*png_ptr).crc = crc as png_uint_32;
    }
}

/* Check a user supplied version number, called from both read and write
 * functions that create a png_struct.
 */
/* png_user_version_check */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_user_version_check(
    png_ptr: png_structrp,
    user_png_ver: png_const_charp,
) -> c_int {
    /* Libpng versions 1.0.0 and later are binary compatible if the version
     * string matches through the second '.'; we must recompile any
     * applications that use any older library version.
     */

    if user_png_ver != core::ptr::null() {
        let mut i: c_int = -1;
        let mut found_dots: c_int = 0;

        loop {
            i += 1;
            if *user_png_ver.offset(i as isize)
                != *PNG_LIBPNG_VER_STRING.as_ptr().offset(i as isize) as c_char
            {
                (*png_ptr).flags |= PNG_FLAG_LIBRARY_MISMATCH;
            }
            if *user_png_ver.offset(i as isize) == b'.' as c_char {
                found_dots += 1;
            }

            if !(found_dots < 2
                && *user_png_ver.offset(i as isize) != 0
                && *PNG_LIBPNG_VER_STRING.as_ptr().offset(i as isize) as c_char != 0)
            {
                break;
            }
        }
    } else {
        (*png_ptr).flags |= PNG_FLAG_LIBRARY_MISMATCH;
    }

    if ((*png_ptr).flags & PNG_FLAG_LIBRARY_MISMATCH) != 0 {
        let mut pos: usize = 0;
        let mut m: [c_char; 128] = [0; 128];

        pos = png_safecat(
            m.as_mut_ptr(),
            128,
            pos,
            b"Application built with libpng-\0".as_ptr() as png_const_charp,
        );
        pos = png_safecat(m.as_mut_ptr(), 128, pos, user_png_ver);
        pos = png_safecat(
            m.as_mut_ptr(),
            128,
            pos,
            b" but running with \0".as_ptr() as png_const_charp,
        );
        pos = png_safecat(
            m.as_mut_ptr(),
            128,
            pos,
            PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
        );

        png_warning(png_ptr, m.as_ptr() as png_const_charp);

        return 0;
    }

    /* Success return. */
    1
}

/* The C code assigns `longjmp` directly to `create_struct.longjmp_fn`; a shim
 * is needed because the Rust declaration of `longjmp` diverges (`-> !`).
 */
unsafe extern "C" fn png_create_longjmp_shim(env: *mut jmp_buf, val: c_int) {
    longjmp(env, val)
}

/* Generic function to create a png_struct for either read or write - this
 * contains the common initialization.
 */
/* png_create_png_struct */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_png_struct(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
    mem_ptr: png_voidp,
    malloc_fn: png_malloc_ptr,
    free_fn: png_free_ptr,
) -> png_structp {
    /* This temporary stack-allocated structure is used to provide a place to
     * build enough context to allow the user provided memory allocator (if any)
     * to be called.
     */
    let mut create_struct: png_struct = core::mem::zeroed();
    let mut create_jmp_buf: jmp_buf = [0; 25];

    create_struct.user_width_max = PNG_USER_WIDTH_MAX;
    create_struct.user_height_max = PNG_USER_HEIGHT_MAX;

    create_struct.user_chunk_cache_max = PNG_USER_CHUNK_CACHE_MAX;

    /* default to compile-time limit */
    create_struct.user_chunk_malloc_max = PNG_USER_CHUNK_MALLOC_MAX;

    /* The following two API calls simply set fields in png_struct, so it is safe
     * to do them now even though error handling is not yet set up.
     */
    png_set_mem_fn(&mut create_struct, mem_ptr, malloc_fn, free_fn);

    /* (*error_fn) can return control to the caller after the error_ptr is set,
     * this will result in a memory leak unless the error_fn does something
     * extremely sophisticated.  The design lacks merit but is implicit in the
     * API.
     */
    png_set_error_fn(&mut create_struct, error_ptr, error_fn, warn_fn);

    if setjmp(&mut create_jmp_buf) == 0 {
        /* Temporarily fake out the longjmp information until we have
         * successfully completed this function.  This only works if we have
         * setjmp() support compiled in, but it is safe - this stuff should
         * never happen.
         */
        create_struct.jmp_buf_ptr = &mut create_jmp_buf;
        create_struct.jmp_buf_size = 0; /*stack allocation*/
        create_struct.longjmp_fn = Some(png_create_longjmp_shim);

        /* Call the general version checker (shared with read and write code):
         */
        if png_user_version_check(&mut create_struct, user_png_ver) != 0 {
            let png_ptr: png_structrp = png_malloc_warn(
                &mut create_struct,
                core::mem::size_of::<png_struct>() as png_alloc_size_t,
            ) as png_structrp;

            if png_ptr != core::ptr::null_mut() {
                /* png_ptr->zstream holds a back-pointer to the png_struct, so
                 * this can only be done now:
                 */
                create_struct.zstream.zalloc = Some(png_zalloc);
                create_struct.zstream.zfree = Some(png_zfree);
                create_struct.zstream.opaque = png_ptr as voidpf;

                /* Eliminate the local error handling: */
                create_struct.jmp_buf_ptr = core::ptr::null_mut();
                create_struct.jmp_buf_size = 0;
                create_struct.longjmp_fn = None;

                core::ptr::write(png_ptr, create_struct);

                /* This is the successful return point */
                return png_ptr;
            }
        }
    }

    /* A longjmp because of a bug in the application storage allocator or a
     * simple failure to allocate the png_struct.
     */
    core::ptr::null_mut()
}

/* Allocate the memory for an info_struct for the application. */
/* png_create_info_struct */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_info_struct(png_ptr: png_const_structrp) -> png_infop {
    let info_ptr: png_inforp;

    if png_ptr == core::ptr::null_mut() {
        return core::ptr::null_mut();
    }

    /* Use the internal API that does not (or at least should not) error out, so
     * that this call always returns ok.  The application typically sets up the
     * error handling *after* creating the info_struct because this is the way it
     * has always been done in 'example.c'.
     */
    info_ptr = png_malloc_base(
        png_ptr,
        core::mem::size_of::<png_info>() as png_alloc_size_t,
    ) as png_inforp;

    if info_ptr != core::ptr::null_mut() {
        memset(
            info_ptr as *mut c_void,
            0,
            core::mem::size_of::<png_info>(),
        );
    }

    info_ptr
}

/* This function frees the memory associated with a single info struct.
 * Normally, one would use either png_destroy_read_struct() or
 * png_destroy_write_struct() to free an info struct, but this may be
 * useful for some applications.  From libpng 1.6.0 this function is also used
 * internally to implement the png_info release part of the 'struct' destroy
 * APIs.  This ensures that all possible approaches free the same data (all of
 * it).
 */
/* png_destroy_info_struct */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_info_struct(
    png_ptr: png_const_structrp,
    info_ptr_ptr: png_infopp,
) {
    let mut info_ptr: png_inforp = core::ptr::null_mut();

    if png_ptr == core::ptr::null_mut() {
        return;
    }

    if info_ptr_ptr != core::ptr::null_mut() {
        info_ptr = *info_ptr_ptr;
    }

    if info_ptr != core::ptr::null_mut() {
        /* Do this first in case of an error below; if the app implements its own
         * memory management this can lead to png_free calling png_error, which
         * will abort this routine and return control to the app error handler.
         * An infinite loop may result if it then tries to free the same info
         * ptr.
         */
        *info_ptr_ptr = core::ptr::null_mut();

        png_free_data(png_ptr, info_ptr, PNG_FREE_ALL, -1);
        memset(
            info_ptr as *mut c_void,
            0,
            core::mem::size_of::<png_info>(),
        );
        png_free(png_ptr, info_ptr as png_voidp);
    }
}

/* Initialize the info structure.  This is now an internal function (0.89)
 * and applications using it are urged to use png_create_info_struct()
 * instead.  Use deprecated in 1.6.0, internal use removed (used internally it
 * is just a memset).
 *
 * NOTE: it is almost inconceivable that this API is used because it bypasses
 * the user-memory mechanism and the user error handling/warning mechanisms in
 * those cases where it does anything other than a memset.
 */
/* png_info_init_3 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_info_init_3(ptr_ptr: png_infopp, png_info_struct_size: usize) {
    let mut info_ptr: png_inforp = *ptr_ptr;

    if info_ptr == core::ptr::null_mut() {
        return;
    }

    if core::mem::size_of::<png_info>() > png_info_struct_size {
        *ptr_ptr = core::ptr::null_mut();
        /* The following line is why this API should not be used: */
        free(info_ptr as *mut c_void);
        info_ptr = png_malloc_base(
            core::ptr::null_mut(),
            core::mem::size_of::<png_info>() as png_alloc_size_t,
        ) as png_inforp;
        if info_ptr == core::ptr::null_mut() {
            return;
        }
        *ptr_ptr = info_ptr;
    }

    /* Set everything to 0 */
    memset(
        info_ptr as *mut c_void,
        0,
        core::mem::size_of::<png_info>(),
    );
}

/* png_data_freer */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_data_freer(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    freer: c_int,
    mask: png_uint_32,
) {
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    if freer == PNG_DESTROY_WILL_FREE_DATA {
        (*info_ptr).free_me |= mask;
    } else if freer == PNG_USER_WILL_FREE_DATA {
        (*info_ptr).free_me &= !mask;
    } else {
        png_error(
            png_ptr,
            b"Unknown freer parameter in png_data_freer\0".as_ptr() as png_const_charp,
        );
    }
}

/* png_free_data */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_data(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mut mask: png_uint_32,
    num: c_int,
) {
    if png_ptr == core::ptr::null_mut() || info_ptr == core::ptr::null_mut() {
        return;
    }

    /* Free text item num or (if num == -1) all text items */
    if (*info_ptr).text != core::ptr::null_mut()
        && ((mask & PNG_FREE_TEXT) & (*info_ptr).free_me) != 0
    {
        if num != -1 {
            png_free(png_ptr, (*(*info_ptr).text.offset(num as isize)).key as png_voidp);
            (*(*info_ptr).text.offset(num as isize)).key = core::ptr::null_mut();
        } else {
            let mut i: c_int = 0;

            while i < (*info_ptr).num_text {
                png_free(png_ptr, (*(*info_ptr).text.offset(i as isize)).key as png_voidp);
                i += 1;
            }

            png_free(png_ptr, (*info_ptr).text as png_voidp);
            (*info_ptr).text = core::ptr::null_mut();
            (*info_ptr).num_text = 0;
            (*info_ptr).max_text = 0;
        }
    }

    /* Free any tRNS entry */
    if ((mask & PNG_FREE_TRNS) & (*info_ptr).free_me) != 0 {
        (*info_ptr).valid &= !PNG_INFO_tRNS;
        png_free(png_ptr, (*info_ptr).trans_alpha as png_voidp);
        (*info_ptr).trans_alpha = core::ptr::null_mut();
        (*info_ptr).num_trans = 0;
    }

    /* Free any sCAL entry */
    if ((mask & PNG_FREE_SCAL) & (*info_ptr).free_me) != 0 {
        png_free(png_ptr, (*info_ptr).scal_s_width as png_voidp);
        png_free(png_ptr, (*info_ptr).scal_s_height as png_voidp);
        (*info_ptr).scal_s_width = core::ptr::null_mut();
        (*info_ptr).scal_s_height = core::ptr::null_mut();
        (*info_ptr).valid &= !PNG_INFO_sCAL;
    }

    /* Free any pCAL entry */
    if ((mask & PNG_FREE_PCAL) & (*info_ptr).free_me) != 0 {
        png_free(png_ptr, (*info_ptr).pcal_purpose as png_voidp);
        png_free(png_ptr, (*info_ptr).pcal_units as png_voidp);
        (*info_ptr).pcal_purpose = core::ptr::null_mut();
        (*info_ptr).pcal_units = core::ptr::null_mut();

        if (*info_ptr).pcal_params != core::ptr::null_mut() {
            let mut i: c_int = 0;

            while i < (*info_ptr).pcal_nparams as c_int {
                png_free(png_ptr, *(*info_ptr).pcal_params.offset(i as isize) as png_voidp);
                i += 1;
            }

            png_free(png_ptr, (*info_ptr).pcal_params as png_voidp);
            (*info_ptr).pcal_params = core::ptr::null_mut();
        }
        (*info_ptr).valid &= !PNG_INFO_pCAL;
    }

    /* Free any profile entry */
    if ((mask & PNG_FREE_ICCP) & (*info_ptr).free_me) != 0 {
        png_free(png_ptr, (*info_ptr).iccp_name as png_voidp);
        png_free(png_ptr, (*info_ptr).iccp_profile as png_voidp);
        (*info_ptr).iccp_name = core::ptr::null_mut();
        (*info_ptr).iccp_profile = core::ptr::null_mut();
        (*info_ptr).valid &= !PNG_INFO_iCCP;
    }

    /* Free a given sPLT entry, or (if num == -1) all sPLT entries */
    if (*info_ptr).splt_palettes != core::ptr::null_mut()
        && ((mask & PNG_FREE_SPLT) & (*info_ptr).free_me) != 0
    {
        if num != -1 {
            png_free(
                png_ptr,
                (*(*info_ptr).splt_palettes.offset(num as isize)).name as png_voidp,
            );
            png_free(
                png_ptr,
                (*(*info_ptr).splt_palettes.offset(num as isize)).entries as png_voidp,
            );
            (*(*info_ptr).splt_palettes.offset(num as isize)).name = core::ptr::null_mut();
            (*(*info_ptr).splt_palettes.offset(num as isize)).entries = core::ptr::null_mut();
        } else {
            let mut i: c_int = 0;

            while i < (*info_ptr).splt_palettes_num {
                png_free(
                    png_ptr,
                    (*(*info_ptr).splt_palettes.offset(i as isize)).name as png_voidp,
                );
                png_free(
                    png_ptr,
                    (*(*info_ptr).splt_palettes.offset(i as isize)).entries as png_voidp,
                );
                i += 1;
            }

            png_free(png_ptr, (*info_ptr).splt_palettes as png_voidp);
            (*info_ptr).splt_palettes = core::ptr::null_mut();
            (*info_ptr).splt_palettes_num = 0;
            (*info_ptr).valid &= !PNG_INFO_sPLT;
        }
    }

    if (*info_ptr).unknown_chunks != core::ptr::null_mut()
        && ((mask & PNG_FREE_UNKN) & (*info_ptr).free_me) != 0
    {
        if num != -1 {
            png_free(
                png_ptr,
                (*(*info_ptr).unknown_chunks.offset(num as isize)).data as png_voidp,
            );
            (*(*info_ptr).unknown_chunks.offset(num as isize)).data = core::ptr::null_mut();
        } else {
            let mut i: c_int = 0;

            while i < (*info_ptr).unknown_chunks_num {
                png_free(
                    png_ptr,
                    (*(*info_ptr).unknown_chunks.offset(i as isize)).data as png_voidp,
                );
                i += 1;
            }

            png_free(png_ptr, (*info_ptr).unknown_chunks as png_voidp);
            (*info_ptr).unknown_chunks = core::ptr::null_mut();
            (*info_ptr).unknown_chunks_num = 0;
        }
    }

    /* Free any eXIf entry */
    if ((mask & PNG_FREE_EXIF) & (*info_ptr).free_me) != 0 {
        if !(*info_ptr).exif.is_null() {
            png_free(png_ptr, (*info_ptr).exif as png_voidp);
            (*info_ptr).exif = core::ptr::null_mut();
        }
        (*info_ptr).valid &= !PNG_INFO_eXIf;
    }

    /* Free any hIST entry */
    if ((mask & PNG_FREE_HIST) & (*info_ptr).free_me) != 0 {
        png_free(png_ptr, (*info_ptr).hist as png_voidp);
        (*info_ptr).hist = core::ptr::null_mut();
        (*info_ptr).valid &= !PNG_INFO_hIST;
    }

    /* Free any PLTE entry that was internally allocated */
    if ((mask & PNG_FREE_PLTE) & (*info_ptr).free_me) != 0 {
        png_free(png_ptr, (*info_ptr).palette as png_voidp);
        (*info_ptr).palette = core::ptr::null_mut();
        (*info_ptr).valid &= !PNG_INFO_PLTE;
        (*info_ptr).num_palette = 0;
    }

    /* Free any image bits attached to the info structure */
    if ((mask & PNG_FREE_ROWS) & (*info_ptr).free_me) != 0 {
        if (*info_ptr).row_pointers != core::ptr::null_mut() {
            let mut row: png_uint_32 = 0;
            while row < (*info_ptr).height {
                png_free(
                    png_ptr,
                    *(*info_ptr).row_pointers.offset(row as isize) as png_voidp,
                );
                row += 1;
            }

            png_free(png_ptr, (*info_ptr).row_pointers as png_voidp);
            (*info_ptr).row_pointers = core::ptr::null_mut();
        }
        (*info_ptr).valid &= !PNG_INFO_IDAT;
    }

    if num != -1 {
        mask &= !PNG_FREE_MUL;
    }

    (*info_ptr).free_me &= !mask;
}
