//! Translation of c_src/src/png.c lines 1..1497
use crate::prelude::*;

/* PNGZ_MSG_CAST(s) -- pngstruct.h macro, plain cast here. */
#[inline]
unsafe fn PNGZ_MSG_CAST(s: *const c_char) -> *const c_char {
    s
}

/* PNG signature bytes (file scope static in png_sig_cmp). */
static png_signature: [png_byte; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/* Tells libpng that we have already handled the first "num_bytes" bytes
 * of the PNG file signature.
 */
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
        png_error(png_ptr, cstr(b"Too many bytes for PNG signature\0"));
    }

    (*png_ptr).sig_bytes = nb as png_byte;
}

/* Checks whether the supplied bytes match the PNG signature. */
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_zalloc(png_ptr: voidpf, items: uInt, size: uInt) -> voidpf {
    let mut num_bytes: png_alloc_size_t = size as png_alloc_size_t;

    if png_ptr == core::ptr::null_mut() {
        return core::ptr::null_mut();
    }

    /* This check against overflow is vestigial. */
    if size != 0
        && items as png_alloc_size_t >= (!(0 as png_alloc_size_t)) / (size as png_alloc_size_t)
    {
        png_warning(
            png_ptr as png_structrp,
            cstr(b"Potential overflow in png_zalloc()\0"),
        );
        return core::ptr::null_mut();
    }

    num_bytes = num_bytes.wrapping_mul(items as png_alloc_size_t);
    png_malloc_warn(png_ptr as png_structrp, num_bytes)
}

/* Function to free memory for zlib */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_zfree(png_ptr: voidpf, ptr: voidpf) {
    png_free(png_ptr as png_const_structrp, ptr);
}

/* Reset the CRC variable to 32 bits of 1's. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_reset_crc(png_ptr: png_structrp) {
    /* The cast is safe because the crc is a 32-bit value. */
    (*png_ptr).crc = crc32(0, core::ptr::null(), 0) as png_uint_32;
}

/* Calculate the CRC over a section of data. */
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
    } else
    /* critical */
    {
        if ((*png_ptr).flags & PNG_FLAG_CRC_CRITICAL_IGNORE) != 0 {
            need_crc = 0;
        }
    }

    if need_crc != 0 && length > 0 {
        let mut crc: uLong = (*png_ptr).crc as uLong; /* Should never issue a warning */

        loop {
            let mut safe_length: uInt = length as uInt;
            if safe_length == 0 {
                safe_length = (-1i32) as uInt; /* evil, but safe */
            }

            crc = crc32(crc, ptr, safe_length);

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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_user_version_check(
    png_ptr: png_structrp,
    user_png_ver: png_const_charp,
) -> c_int {
    let ver_string: *const c_char = PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char;

    if user_png_ver != core::ptr::null() {
        let mut i: c_int = -1;
        let mut found_dots: c_int = 0;

        loop {
            i += 1;
            if *user_png_ver.add(i as usize) != *ver_string.add(i as usize) {
                (*png_ptr).flags |= PNG_FLAG_LIBRARY_MISMATCH;
            }
            if *user_png_ver.add(i as usize) == b'.' as c_char {
                found_dots += 1;
            }
            if !(found_dots < 2
                && *user_png_ver.add(i as usize) != 0
                && *ver_string.add(i as usize) != 0)
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
            core::mem::size_of::<[c_char; 128]>(),
            pos,
            cstr(b"Application built with libpng-\0"),
        );
        pos = png_safecat(
            m.as_mut_ptr(),
            core::mem::size_of::<[c_char; 128]>(),
            pos,
            user_png_ver,
        );
        pos = png_safecat(
            m.as_mut_ptr(),
            core::mem::size_of::<[c_char; 128]>(),
            pos,
            cstr(b" but running with \0"),
        );
        pos = png_safecat(
            m.as_mut_ptr(),
            core::mem::size_of::<[c_char; 128]>(),
            pos,
            ver_string,
        );
        let _ = pos;

        png_warning(png_ptr, m.as_ptr());

        return 0;
    }

    /* Success return. */
    1
}

/* Generic function to create a png_struct for either read or write. */
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
    let mut create_struct: png_struct = core::mem::zeroed();
    let mut create_jmp_buf: jmp_buf = jmp_buf::new();

    /* This temporary stack-allocated structure is used to provide a place to
     * build enough context to allow the user provided memory allocator (if any)
     * to be called.
     */
    memset(
        &mut create_struct as *mut png_struct as *mut c_void,
        0,
        core::mem::size_of::<png_struct>(),
    );

    create_struct.user_width_max = PNG_USER_WIDTH_MAX;
    create_struct.user_height_max = PNG_USER_HEIGHT_MAX;

    create_struct.user_chunk_cache_max = PNG_USER_CHUNK_CACHE_MAX;

    /* PNG_USER_CHUNK_MALLOC_MAX > 0: default to the compile-time limit */
    create_struct.user_chunk_malloc_max = PNG_USER_CHUNK_MALLOC_MAX;

    /* The following two API calls simply set fields in png_struct. */
    png_set_mem_fn(&mut create_struct, mem_ptr, malloc_fn, free_fn);

    png_set_error_fn(&mut create_struct, error_ptr, error_fn, warn_fn);

    if setjmp(&mut create_jmp_buf) == 0 {
        /* Temporarily fake out the longjmp information until we have
         * successfully completed this function.
         */
        create_struct.jmp_buf_ptr = &mut create_jmp_buf;
        create_struct.jmp_buf_size = 0; /*stack allocation*/
        create_struct.longjmp_fn = Some(longjmp);

        /* Call the general version checker (shared with read and write code): */
        if png_user_version_check(&mut create_struct, user_png_ver) != 0 {
            let png_ptr: png_structrp =
                png_malloc_warn(&mut create_struct, core::mem::size_of::<png_struct>())
                    as png_structrp;

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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_info_struct(png_ptr: png_const_structrp) -> png_infop {
    let info_ptr: png_inforp;

    if png_ptr == core::ptr::null() {
        return core::ptr::null_mut();
    }

    info_ptr = png_malloc_base(png_ptr, core::mem::size_of::<png_info>()) as png_inforp;

    if info_ptr != core::ptr::null_mut() {
        memset(info_ptr as *mut c_void, 0, core::mem::size_of::<png_info>());
    }

    info_ptr
}

/* This function frees the memory associated with a single info struct. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_info_struct(
    png_ptr: png_const_structrp,
    info_ptr_ptr: png_infopp,
) {
    let mut info_ptr: png_inforp = core::ptr::null_mut();

    if png_ptr == core::ptr::null() {
        return;
    }

    if info_ptr_ptr != core::ptr::null_mut() {
        info_ptr = *info_ptr_ptr;
    }

    if info_ptr != core::ptr::null_mut() {
        /* Do this first in case of an error below. */
        *info_ptr_ptr = core::ptr::null_mut();

        png_free_data(png_ptr, info_ptr, PNG_FREE_ALL, -1);
        memset(info_ptr as *mut c_void, 0, core::mem::size_of::<png_info>());
        png_free(png_ptr, info_ptr as png_voidp);
    }
}

/* Initialize the info structure.  This is now an internal function. */
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
        info_ptr =
            png_malloc_base(core::ptr::null(), core::mem::size_of::<png_info>()) as png_inforp;
        if info_ptr == core::ptr::null_mut() {
            return;
        }
        *ptr_ptr = info_ptr;
    }

    /* Set everything to 0 */
    memset(info_ptr as *mut c_void, 0, core::mem::size_of::<png_info>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_data_freer(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    freer: c_int,
    mask: png_uint_32,
) {
    if png_ptr == core::ptr::null() || info_ptr == core::ptr::null_mut() {
        return;
    }

    if freer == PNG_DESTROY_WILL_FREE_DATA {
        (*info_ptr).free_me |= mask;
    } else if freer == PNG_USER_WILL_FREE_DATA {
        (*info_ptr).free_me &= !mask;
    } else {
        png_error(
            png_ptr,
            cstr(b"Unknown freer parameter in png_data_freer\0"),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_data(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mut mask: png_uint_32,
    num: c_int,
) {
    if png_ptr == core::ptr::null() || info_ptr == core::ptr::null_mut() {
        return;
    }

    /* Free text item num or (if num == -1) all text items */
    if (*info_ptr).text != core::ptr::null_mut()
        && ((mask & PNG_FREE_TEXT) & (*info_ptr).free_me) != 0
    {
        if num != -1 {
            png_free(
                png_ptr,
                (*(*info_ptr).text.add(num as usize)).key as png_voidp,
            );
            (*(*info_ptr).text.add(num as usize)).key = core::ptr::null_mut();
        } else {
            let mut i: c_int = 0;

            while i < (*info_ptr).num_text {
                png_free(
                    png_ptr,
                    (*(*info_ptr).text.add(i as usize)).key as png_voidp,
                );
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
                png_free(
                    png_ptr,
                    *(*info_ptr).pcal_params.add(i as usize) as png_voidp,
                );
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
                (*(*info_ptr).splt_palettes.add(num as usize)).name as png_voidp,
            );
            png_free(
                png_ptr,
                (*(*info_ptr).splt_palettes.add(num as usize)).entries as png_voidp,
            );
            (*(*info_ptr).splt_palettes.add(num as usize)).name = core::ptr::null_mut();
            (*(*info_ptr).splt_palettes.add(num as usize)).entries = core::ptr::null_mut();
        } else {
            let mut i: c_int = 0;

            while i < (*info_ptr).splt_palettes_num {
                png_free(
                    png_ptr,
                    (*(*info_ptr).splt_palettes.add(i as usize)).name as png_voidp,
                );
                png_free(
                    png_ptr,
                    (*(*info_ptr).splt_palettes.add(i as usize)).entries as png_voidp,
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
                (*(*info_ptr).unknown_chunks.add(num as usize)).data as png_voidp,
            );
            (*(*info_ptr).unknown_chunks.add(num as usize)).data = core::ptr::null_mut();
        } else {
            let mut i: c_int = 0;

            while i < (*info_ptr).unknown_chunks_num {
                png_free(
                    png_ptr,
                    (*(*info_ptr).unknown_chunks.add(i as usize)).data as png_voidp,
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
        if (*info_ptr).exif != core::ptr::null_mut() {
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
                    *(*info_ptr).row_pointers.add(row as usize) as png_voidp,
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

/* Returns a pointer to the io_ptr associated with the user functions. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_io_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr == core::ptr::null() {
        return core::ptr::null_mut();
    }

    (*png_ptr).io_ptr
}

/* Initialize the default input/output functions for the PNG file. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_init_io(png_ptr: png_structrp, fp: *mut FILE) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    (*png_ptr).io_ptr = fp as png_voidp;
}

/* PNG signed integers are saved in 32-bit 2's complement format. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_save_int_32(buf: png_bytep, i: png_int_32) {
    png_save_uint_32(buf, i as png_uint_32);
}

/* Convert the supplied time into an RFC 1123 string. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_convert_to_rfc1123_buffer(
    out: *mut c_char,
    ptime: png_const_timep,
) -> c_int {
    static short_months: [[u8; 4]; 12] = [
        *b"Jan\0", *b"Feb\0", *b"Mar\0", *b"Apr\0", *b"May\0", *b"Jun\0", *b"Jul\0", *b"Aug\0",
        *b"Sep\0", *b"Oct\0", *b"Nov\0", *b"Dec\0",
    ];

    if out == core::ptr::null_mut() {
        return 0;
    }

    if (*ptime).year > 9999 /* RFC1123 limitation */
        || (*ptime).month == 0
        || (*ptime).month > 12
        || (*ptime).day == 0
        || (*ptime).day > 31
        || (*ptime).hour > 23
        || (*ptime).minute > 59
        || (*ptime).second > 60
    {
        return 0;
    }

    {
        let mut pos: usize = 0;
        let mut number_buf: [c_char; 5] = [0, 0, 0, 0, 0]; /* enough for a four-digit year */

        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_u, (unsigned)ptime->day) */
        pos = png_safecat(
            out,
            29,
            pos,
            png_format_number(
                number_buf.as_ptr(),
                number_buf.as_mut_ptr().add(5),
                PNG_NUMBER_FORMAT_u,
                (*ptime).day as png_alloc_size_t,
            ),
        );
        /* APPEND(' ') */
        if pos < 28 {
            *out.add(pos) = b' ' as c_char;
            pos += 1;
        }
        /* APPEND_STRING(short_months[ptime->month - 1]) */
        pos = png_safecat(
            out,
            29,
            pos,
            short_months[((*ptime).month - 1) as usize].as_ptr() as *const c_char,
        );
        if pos < 28 {
            *out.add(pos) = b' ' as c_char;
            pos += 1;
        }
        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_u, ptime->year) */
        pos = png_safecat(
            out,
            29,
            pos,
            png_format_number(
                number_buf.as_ptr(),
                number_buf.as_mut_ptr().add(5),
                PNG_NUMBER_FORMAT_u,
                (*ptime).year as png_alloc_size_t,
            ),
        );
        if pos < 28 {
            *out.add(pos) = b' ' as c_char;
            pos += 1;
        }
        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_02u, (unsigned)ptime->hour) */
        pos = png_safecat(
            out,
            29,
            pos,
            png_format_number(
                number_buf.as_ptr(),
                number_buf.as_mut_ptr().add(5),
                PNG_NUMBER_FORMAT_02u,
                (*ptime).hour as png_alloc_size_t,
            ),
        );
        if pos < 28 {
            *out.add(pos) = b':' as c_char;
            pos += 1;
        }
        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_02u, (unsigned)ptime->minute) */
        pos = png_safecat(
            out,
            29,
            pos,
            png_format_number(
                number_buf.as_ptr(),
                number_buf.as_mut_ptr().add(5),
                PNG_NUMBER_FORMAT_02u,
                (*ptime).minute as png_alloc_size_t,
            ),
        );
        if pos < 28 {
            *out.add(pos) = b':' as c_char;
            pos += 1;
        }
        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_02u, (unsigned)ptime->second) */
        pos = png_safecat(
            out,
            29,
            pos,
            png_format_number(
                number_buf.as_ptr(),
                number_buf.as_mut_ptr().add(5),
                PNG_NUMBER_FORMAT_02u,
                (*ptime).second as png_alloc_size_t,
            ),
        );
        /* APPEND_STRING(" +0000") -- reliably terminates the buffer */
        pos = png_safecat(out, 29, pos, cstr(b" +0000\0"));
        let _ = pos;
    }

    1
}

/* Original API that uses a private buffer in png_struct. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_convert_to_rfc1123(
    png_ptr: png_structrp,
    ptime: png_const_timep,
) -> png_const_charp {
    if png_ptr != core::ptr::null_mut() {
        /* The only failure above if png_ptr != NULL is from an invalid ptime */
        if png_convert_to_rfc1123_buffer((*png_ptr).time_buffer.as_mut_ptr(), ptime) == 0 {
            png_warning(png_ptr, cstr(b"Ignoring invalid time value\0"));
        } else {
            return (*png_ptr).time_buffer.as_ptr();
        }
    }

    core::ptr::null()
}

/* File-scope copyright string (assembled from PNG_STRING_NEWLINE concatenation). */
static png_libpng_copyright: [u8; 223] = *b"\n\
libpng version 1.6.59.git\n\
Copyright (c) 2018-2026 Cosmin Truta\n\
Copyright (c) 1998-2002,2004,2006-2018 Glenn Randers-Pehrson\n\
Copyright (c) 1996-1997 Andreas Dilger\n\
Copyright (c) 1995-1996 Guy Eric Schalnat, Group 42, Inc.\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_copyright(png_ptr: png_const_structrp) -> png_const_charp {
    let _ = png_ptr;
    png_libpng_copyright.as_ptr() as *const c_char
}

/* Return the library version as a short string. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_libpng_ver(png_ptr: png_const_structrp) -> png_const_charp {
    /* Version of *.c files used when building libpng */
    png_get_header_ver(png_ptr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_header_ver(png_ptr: png_const_structrp) -> png_const_charp {
    /* Version of *.h files used when building libpng */
    let _ = png_ptr;
    PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char
}

/* File-scope: PNG_HEADER_VERSION_STRING PNG_STRING_NEWLINE (__STDC__ defined). */
static png_header_version_string: [u8; 29] = *b" libpng version 1.6.59.git\n\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_header_version(png_ptr: png_const_structrp) -> png_const_charp {
    /* Returns longer string containing both version and date */
    let _ = png_ptr;
    png_header_version_string.as_ptr() as *const c_char
}

/* Build a grayscale palette.  NOTE: this routine is not used internally! */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_build_grayscale_palette(bit_depth: c_int, palette: png_colorp) {
    let num_palette: c_int;
    let color_inc: c_int;

    if palette == core::ptr::null_mut() {
        return;
    }

    match bit_depth {
        1 => {
            num_palette = 2;
            color_inc = 0xff;
        }
        2 => {
            num_palette = 4;
            color_inc = 0x55;
        }
        4 => {
            num_palette = 16;
            color_inc = 0x11;
        }
        8 => {
            num_palette = 256;
            color_inc = 1;
        }
        _ => {
            num_palette = 0;
            color_inc = 0;
        }
    }

    let mut i: c_int = 0;
    let mut v: c_int = 0;
    while i < num_palette {
        (*palette.add(i as usize)).red = (v & 0xff) as png_byte;
        (*palette.add(i as usize)).green = (v & 0xff) as png_byte;
        (*palette.add(i as usize)).blue = (v & 0xff) as png_byte;
        i += 1;
        v += color_inc;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_handle_as_unknown(
    png_ptr: png_const_structrp,
    chunk_name: png_const_bytep,
) -> c_int {
    /* Check chunk_name and return "keep" value if it's on the list, else 0 */
    let p_end: png_const_bytep;
    let mut p: png_const_bytep;

    if png_ptr == core::ptr::null()
        || chunk_name == core::ptr::null()
        || (*png_ptr).num_chunk_list == 0
    {
        return PNG_HANDLE_CHUNK_AS_DEFAULT;
    }

    p_end = (*png_ptr).chunk_list;
    p = p_end.add(((*png_ptr).num_chunk_list * 5) as usize); /* beyond end */

    /* The code is the fifth byte after each four byte string. */
    loop {
        p = p.sub(5);

        if memcmp(chunk_name as *const c_void, p as *const c_void, 4) == 0 {
            return *p.add(4) as c_int;
        }

        if !(p > p_end) {
            break;
        }
    }

    PNG_HANDLE_CHUNK_AS_DEFAULT
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_unknown_handling(
    png_ptr: png_const_structrp,
    chunk_name: png_uint_32,
) -> c_int {
    let mut chunk_string: [png_byte; 5] = [0; 5];

    PNG_CSTRING_FROM_CHUNK(chunk_string.as_mut_ptr() as *mut c_char, chunk_name);
    png_handle_as_unknown(png_ptr, chunk_string.as_ptr())
}

/* This function, added to libpng-1.0.6g, is untested. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_reset_zstream(png_ptr: png_structrp) -> c_int {
    if png_ptr == core::ptr::null_mut() {
        return Z_STREAM_ERROR;
    }

    /* WARNING: this resets the window bits to the maximum! */
    inflateReset(&mut (*png_ptr).zstream)
}

/* This function was added to libpng-1.0.7 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_access_version_number() -> png_uint_32 {
    /* Version of *.c files used when building libpng */
    PNG_LIBPNG_VER as png_uint_32
}

/* Ensure that png_ptr->zstream.msg holds some appropriate error message. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_zstream_error(png_ptr: png_structrp, ret: c_int) {
    if (*png_ptr).zstream.msg == core::ptr::null() {
        match ret {
            Z_STREAM_END => {
                /* Normal exit */
                (*png_ptr).zstream.msg = PNGZ_MSG_CAST(cstr(b"unexpected end of LZ stream\0"));
            }
            Z_NEED_DICT => {
                (*png_ptr).zstream.msg = PNGZ_MSG_CAST(cstr(b"missing LZ dictionary\0"));
            }
            Z_ERRNO => {
                /* gz APIs only: should not happen */
                (*png_ptr).zstream.msg = PNGZ_MSG_CAST(cstr(b"zlib IO error\0"));
            }
            Z_STREAM_ERROR => {
                /* internal libpng error */
                (*png_ptr).zstream.msg = PNGZ_MSG_CAST(cstr(b"bad parameters to zlib\0"));
            }
            Z_DATA_ERROR => {
                (*png_ptr).zstream.msg = PNGZ_MSG_CAST(cstr(b"damaged LZ stream\0"));
            }
            Z_MEM_ERROR => {
                (*png_ptr).zstream.msg = PNGZ_MSG_CAST(cstr(b"insufficient memory\0"));
            }
            Z_BUF_ERROR => {
                (*png_ptr).zstream.msg = PNGZ_MSG_CAST(cstr(b"truncated\0"));
            }
            Z_VERSION_ERROR => {
                (*png_ptr).zstream.msg = PNGZ_MSG_CAST(cstr(b"unsupported zlib version\0"));
            }
            PNG_UNEXPECTED_ZLIB_RETURN => {
                (*png_ptr).zstream.msg = PNGZ_MSG_CAST(cstr(b"unexpected zlib return\0"));
            }
            /* default and Z_OK */
            _ => {
                (*png_ptr).zstream.msg = PNGZ_MSG_CAST(cstr(b"unexpected zlib return code\0"));
            }
        }
    }
}

/* ---------------- COLORSPACE ---------------- */

pub unsafe extern "C" fn png_fp_add(
    addend0: png_int_32,
    addend1: png_int_32,
    error: *mut c_int,
) -> png_int_32 {
    /* Safely add two fixed point values setting an error flag and returning 0.5
     * on overflow.
     */
    if addend0 > 0 {
        if 0x7fffffff - addend0 >= addend1 {
            return addend0 + addend1;
        }
    } else if addend0 < 0 {
        if -0x7fffffff - addend0 <= addend1 {
            return addend0 + addend1;
        }
    } else {
        return addend1;
    }

    *error = 1;
    PNG_FP_1 / 2
}

pub unsafe extern "C" fn png_fp_sub(
    addend0: png_int_32,
    addend1: png_int_32,
    error: *mut c_int,
) -> png_int_32 {
    /* As above but calculate addend0-addend1. */
    if addend1 > 0 {
        if -0x7fffffff + addend1 <= addend0 {
            return addend0 - addend1;
        }
    } else if addend1 < 0 {
        if 0x7fffffff + addend1 >= addend0 {
            return addend0 - addend1;
        }
    } else {
        return addend0;
    }

    *error = 1;
    PNG_FP_1 / 2
}

pub unsafe extern "C" fn png_safe_add(
    addend0_and_result: *mut png_int_32,
    addend1: png_int_32,
    addend2: png_int_32,
) -> c_int {
    /* Safely add three integers.  Returns 0 on success, 1 on overflow. */
    let mut error: c_int = 0;
    let result: png_int_32 = png_fp_add(
        *addend0_and_result,
        png_fp_add(addend1, addend2, &mut error),
        &mut error,
    );
    if error == 0 {
        *addend0_and_result = result;
    }
    error
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_xy_from_XYZ(xy: *mut png_xy, XYZ: *const png_XYZ) -> c_int {
    /* NOTE: returns 0 on success, 1 means error. */
    let mut d: png_int_32;
    let dred: png_int_32;
    let dgreen: png_int_32;
    let dblue: png_int_32;
    let dwhite: png_int_32;
    let whiteX: png_int_32;
    let whiteY: png_int_32;

    d = (*XYZ).red_X;
    if png_safe_add(&mut d, (*XYZ).red_Y, (*XYZ).red_Z) != 0 {
        return 1;
    }
    dred = d;
    if png_muldiv(&mut (*xy).redx, (*XYZ).red_X, PNG_FP_1, dred) == 0 {
        return 1;
    }
    if png_muldiv(&mut (*xy).redy, (*XYZ).red_Y, PNG_FP_1, dred) == 0 {
        return 1;
    }

    d = (*XYZ).green_X;
    if png_safe_add(&mut d, (*XYZ).green_Y, (*XYZ).green_Z) != 0 {
        return 1;
    }
    dgreen = d;
    if png_muldiv(&mut (*xy).greenx, (*XYZ).green_X, PNG_FP_1, dgreen) == 0 {
        return 1;
    }
    if png_muldiv(&mut (*xy).greeny, (*XYZ).green_Y, PNG_FP_1, dgreen) == 0 {
        return 1;
    }

    d = (*XYZ).blue_X;
    if png_safe_add(&mut d, (*XYZ).blue_Y, (*XYZ).blue_Z) != 0 {
        return 1;
    }
    dblue = d;
    if png_muldiv(&mut (*xy).bluex, (*XYZ).blue_X, PNG_FP_1, dblue) == 0 {
        return 1;
    }
    if png_muldiv(&mut (*xy).bluey, (*XYZ).blue_Y, PNG_FP_1, dblue) == 0 {
        return 1;
    }

    /* The reference white is the sum of the end-point (X,Y,Z) vectors. */
    d = dblue;
    if png_safe_add(&mut d, dred, dgreen) != 0 {
        return 1;
    }
    dwhite = d;

    /* Find the white X,Y values from the sum of the red, green and blue X,Y. */
    d = (*XYZ).red_X;
    if png_safe_add(&mut d, (*XYZ).green_X, (*XYZ).blue_X) != 0 {
        return 1;
    }
    whiteX = d;

    d = (*XYZ).red_Y;
    if png_safe_add(&mut d, (*XYZ).green_Y, (*XYZ).blue_Y) != 0 {
        return 1;
    }
    whiteY = d;

    if png_muldiv(&mut (*xy).whitex, whiteX, PNG_FP_1, dwhite) == 0 {
        return 1;
    }
    if png_muldiv(&mut (*xy).whitey, whiteY, PNG_FP_1, dwhite) == 0 {
        return 1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_XYZ_from_xy(XYZ: *mut png_XYZ, xy: *const png_xy) -> c_int {
    /* NOTE: returns 0 on success, 1 means error. */
    let red_inverse: png_fixed_point;
    let green_inverse: png_fixed_point;
    let blue_scale: png_fixed_point;
    let mut left: png_fixed_point = 0;
    let mut right: png_fixed_point = 0;
    let denominator: png_fixed_point;

    /* Check xy and, implicitly, z. */
    let fpLimit: png_fixed_point = PNG_FP_1 + (PNG_FP_1 / 10);
    if (*xy).redx < 0 || (*xy).redx > fpLimit {
        return 1;
    }
    if (*xy).redy < 0 || (*xy).redy > fpLimit - (*xy).redx {
        return 1;
    }
    if (*xy).greenx < 0 || (*xy).greenx > fpLimit {
        return 1;
    }
    if (*xy).greeny < 0 || (*xy).greeny > fpLimit - (*xy).greenx {
        return 1;
    }
    if (*xy).bluex < 0 || (*xy).bluex > fpLimit {
        return 1;
    }
    if (*xy).bluey < 0 || (*xy).bluey > fpLimit - (*xy).bluex {
        return 1;
    }
    if (*xy).whitex < 0 || (*xy).whitex > fpLimit {
        return 1;
    }
    if (*xy).whitey < 5 || (*xy).whitey > fpLimit - (*xy).whitex {
        return 1;
    }

    {
        let mut error: c_int = 0;

        if png_muldiv(
            &mut left,
            (*xy).greenx - (*xy).bluex,
            (*xy).redy - (*xy).bluey,
            8,
        ) == 0
        {
            return 1;
        }
        if png_muldiv(
            &mut right,
            (*xy).greeny - (*xy).bluey,
            (*xy).redx - (*xy).bluex,
            8,
        ) == 0
        {
            return 1;
        }
        denominator = png_fp_sub(left, right, &mut error);
        if error != 0 {
            return 1;
        }

        /* Now find the red numerator. */
        if png_muldiv(
            &mut left,
            (*xy).greenx - (*xy).bluex,
            (*xy).whitey - (*xy).bluey,
            8,
        ) == 0
        {
            return 1;
        }
        if png_muldiv(
            &mut right,
            (*xy).greeny - (*xy).bluey,
            (*xy).whitex - (*xy).bluex,
            8,
        ) == 0
        {
            return 1;
        }

        /* Overflow is possible here. */
        let mut red_inverse_tmp: png_fixed_point = 0;
        if png_muldiv(
            &mut red_inverse_tmp,
            (*xy).whitey,
            denominator,
            png_fp_sub(left, right, &mut error),
        ) == 0
            || error != 0
            || red_inverse_tmp <= (*xy).whitey
        {
            return 1;
        }
        red_inverse = red_inverse_tmp;

        /* Similarly for green_inverse: */
        if png_muldiv(
            &mut left,
            (*xy).redy - (*xy).bluey,
            (*xy).whitex - (*xy).bluex,
            8,
        ) == 0
        {
            return 1;
        }
        if png_muldiv(
            &mut right,
            (*xy).redx - (*xy).bluex,
            (*xy).whitey - (*xy).bluey,
            8,
        ) == 0
        {
            return 1;
        }
        let mut green_inverse_tmp: png_fixed_point = 0;
        if png_muldiv(
            &mut green_inverse_tmp,
            (*xy).whitey,
            denominator,
            png_fp_sub(left, right, &mut error),
        ) == 0
            || error != 0
            || green_inverse_tmp <= (*xy).whitey
        {
            return 1;
        }
        green_inverse = green_inverse_tmp;

        /* And the blue scale. */
        blue_scale = png_fp_sub(
            png_fp_sub(
                png_reciprocal((*xy).whitey),
                png_reciprocal(red_inverse),
                &mut error,
            ),
            png_reciprocal(green_inverse),
            &mut error,
        );
        if error != 0 || blue_scale <= 0 {
            return 1;
        }
    }

    /* And fill in the png_XYZ. */
    if png_muldiv(&mut (*XYZ).red_X, (*xy).redx, PNG_FP_1, red_inverse) == 0 {
        return 1;
    }
    if png_muldiv(&mut (*XYZ).red_Y, (*xy).redy, PNG_FP_1, red_inverse) == 0 {
        return 1;
    }
    if png_muldiv(
        &mut (*XYZ).red_Z,
        PNG_FP_1 - (*xy).redx - (*xy).redy,
        PNG_FP_1,
        red_inverse,
    ) == 0
    {
        return 1;
    }

    if png_muldiv(&mut (*XYZ).green_X, (*xy).greenx, PNG_FP_1, green_inverse) == 0 {
        return 1;
    }
    if png_muldiv(&mut (*XYZ).green_Y, (*xy).greeny, PNG_FP_1, green_inverse) == 0 {
        return 1;
    }
    if png_muldiv(
        &mut (*XYZ).green_Z,
        PNG_FP_1 - (*xy).greenx - (*xy).greeny,
        PNG_FP_1,
        green_inverse,
    ) == 0
    {
        return 1;
    }

    if png_muldiv(&mut (*XYZ).blue_X, (*xy).bluex, blue_scale, PNG_FP_1) == 0 {
        return 1;
    }
    if png_muldiv(&mut (*XYZ).blue_Y, (*xy).bluey, blue_scale, PNG_FP_1) == 0 {
        return 1;
    }
    if png_muldiv(
        &mut (*XYZ).blue_Z,
        PNG_FP_1 - (*xy).bluex - (*xy).bluey,
        blue_scale,
        PNG_FP_1,
    ) == 0
    {
        return 1;
    }

    0 /*success*/
}
