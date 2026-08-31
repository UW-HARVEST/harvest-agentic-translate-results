//! Translation of png.c

use crate::*;

/* float.h constants for x86-64 glibc double. */
const DBL_DIG: c_int = 15;
const DBL_MIN: f64 = f64::MIN_POSITIVE;
const DBL_MAX: f64 = f64::MAX;
const DBL_MIN_10_EXP: c_int = -307;

/* setjmp/longjmp bindings (from the C library). */
unsafe extern "C-unwind" {
    fn setjmp(env: *mut jmp_buf) -> c_int;
    #[link_name = "longjmp"]
    fn c_longjmp(env: *mut jmp_buf, val: c_int) -> !;
}

/* Wrapper matching png_longjmp_ptr's signature. */
unsafe extern "C-unwind" fn longjmp(env: *mut jmp_buf, val: c_int) -> ! {
    unsafe { c_longjmp(env, val) }
}

/* pngpriv.h macro: png_chunk_max(png_ptr) -> user_chunk_malloc_max. */
#[inline]
unsafe fn png_chunk_max(png_ptr: png_const_structrp) -> png_alloc_size_t {
    unsafe { (*png_ptr).user_chunk_malloc_max }
}

/* Tells libpng that we have already handled the first "num_bytes" bytes
 * of the PNG file signature.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sig_bytes(png_ptr: png_structrp, num_bytes: c_int) {
    unsafe {
        let mut nb: c_uint = num_bytes as c_uint;

        if png_ptr.is_null() {
            return;
        }

        if num_bytes < 0 {
            nb = 0;
        }

        if nb > 8 {
            png_error(png_ptr, c"Too many bytes for PNG signature".as_ptr());
        }

        (*png_ptr).sig_bytes = nb as png_byte;
    }
}

/* Checks whether the supplied bytes match the PNG signature. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_sig_cmp(
    sig: png_const_bytep,
    start: usize,
    mut num_to_check: usize,
) -> c_int {
    unsafe {
        static png_signature: [png_byte; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

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
}

/* Function to allocate memory for zlib */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_zalloc(png_ptr: voidpf, items: uInt, size: uInt) -> voidpf {
    unsafe {
        let mut num_bytes: png_alloc_size_t = size as png_alloc_size_t;

        if png_ptr.is_null() {
            return core::ptr::null_mut();
        }

        /* This check against overflow is vestigial, dating back from
         * the old times when png_zalloc used to be an exported function.
         */
        if size != 0 && items as png_alloc_size_t >= (!(0 as png_alloc_size_t)) / size as png_alloc_size_t {
            png_warning(
                png_ptr as png_structrp,
                c"Potential overflow in png_zalloc()".as_ptr(),
            );
            return core::ptr::null_mut();
        }

        num_bytes *= items as png_alloc_size_t;
        png_malloc_warn(png_ptr as png_structrp, num_bytes)
    }
}

/* Function to free memory for zlib */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_zfree(png_ptr: voidpf, ptr: voidpf) {
    unsafe {
        png_free(png_ptr as png_const_structrp, ptr);
    }
}

/* Reset the CRC variable to 32 bits of 1's. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_reset_crc(png_ptr: png_structrp) {
    unsafe {
        /* The cast is safe because the crc is a 32-bit value. */
        (*png_ptr).crc = crc32(0, core::ptr::null(), 0) as png_uint_32;
    }
}

/* Calculate the CRC over a section of data. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_calculate_crc(
    png_ptr: png_structrp,
    mut ptr: png_const_bytep,
    mut length: usize,
) {
    unsafe {
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

                if length == 0 {
                    break;
                }
            }

            /* And the following is always safe because the crc is only 32 bits. */
            (*png_ptr).crc = crc as png_uint_32;
        }
    }
}

/* Check a user supplied version number, called from both read and write
 * functions that create a png_struct.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_user_version_check(
    png_ptr: png_structrp,
    user_png_ver: png_const_charp,
) -> c_int {
    unsafe {
        if !user_png_ver.is_null() {
            let mut i: c_int = -1;
            let mut found_dots: c_int = 0;

            loop {
                i += 1;
                if *user_png_ver.offset(i as isize)
                    != PNG_LIBPNG_VER_STRING[i as usize] as c_char
                {
                    (*png_ptr).flags |= PNG_FLAG_LIBRARY_MISMATCH;
                }
                if *user_png_ver.offset(i as isize) == '.' as c_char {
                    found_dots += 1;
                }

                if !(found_dots < 2
                    && *user_png_ver.offset(i as isize) != 0
                    && PNG_LIBPNG_VER_STRING[i as usize] != 0)
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
                core::mem::size_of_val(&m),
                pos,
                c"Application built with libpng-".as_ptr(),
            );
            pos = png_safecat(m.as_mut_ptr(), core::mem::size_of_val(&m), pos, user_png_ver);
            pos = png_safecat(
                m.as_mut_ptr(),
                core::mem::size_of_val(&m),
                pos,
                c" but running with ".as_ptr(),
            );
            pos = png_safecat(
                m.as_mut_ptr(),
                core::mem::size_of_val(&m),
                pos,
                PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
            );
            PNG_UNUSED(pos);

            png_warning(png_ptr, m.as_ptr());

            return 0;
        }

        /* Success return. */
        1
    }
}

/* Generic function to create a png_struct for either read or write. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_create_png_struct(
    user_png_ver: png_const_charp,
    error_ptr: png_voidp,
    error_fn: png_error_ptr,
    warn_fn: png_error_ptr,
    mem_ptr: png_voidp,
    malloc_fn: png_malloc_ptr,
    free_fn: png_free_ptr,
) -> png_structp {
    unsafe {
        let mut create_struct: png_struct = core::mem::zeroed();
        let mut create_jmp_buf: jmp_buf = jmp_buf([0u8; 200]);

        /* This temporary stack-allocated structure is used to provide a place to
         * build enough context to allow the user provided memory allocator (if any)
         * to be called.
         */
        memset(
            &raw mut create_struct as *mut c_void,
            0,
            core::mem::size_of_val(&create_struct),
        );

        create_struct.user_width_max = PNG_USER_WIDTH_MAX;
        create_struct.user_height_max = PNG_USER_HEIGHT_MAX;

        create_struct.user_chunk_cache_max = PNG_USER_CHUNK_CACHE_MAX;

        create_struct.user_chunk_malloc_max = PNG_USER_CHUNK_MALLOC_MAX;

        /* The following two API calls simply set fields in png_struct. */
        png_set_mem_fn(&raw mut create_struct, mem_ptr, malloc_fn, free_fn);

        png_set_error_fn(&raw mut create_struct, error_ptr, error_fn, warn_fn);

        if setjmp(&raw mut create_jmp_buf) == 0 {
            /* Temporarily fake out the longjmp information until we have
             * successfully completed this function.
             */
            create_struct.jmp_buf_ptr = &raw mut create_jmp_buf;
            create_struct.jmp_buf_size = 0; /*stack allocation*/
            create_struct.longjmp_fn = Some(longjmp);

            /* Call the general version checker (shared with read and write code): */
            if png_user_version_check(&raw mut create_struct, user_png_ver) != 0 {
                let png_ptr: png_structrp = png_malloc_warn(
                    &raw mut create_struct,
                    core::mem::size_of::<png_struct>(),
                ) as png_structrp;

                if !png_ptr.is_null() {
                    /* png_ptr->zstream holds a back-pointer to the png_struct, so
                     * this can only be done now:
                     */
                    create_struct.zstream.zalloc = Some(png_zalloc);
                    create_struct.zstream.zfree = Some(png_zfree);
                    create_struct.zstream.opaque = png_ptr as png_voidp;

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
}

/* Allocate the memory for an info_struct for the application. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_create_info_struct(png_ptr: png_const_structrp) -> png_infop {
    unsafe {
        let info_ptr: png_inforp;

        if png_ptr.is_null() {
            return core::ptr::null_mut();
        }

        info_ptr =
            png_malloc_base(png_ptr, core::mem::size_of::<png_info>()) as png_inforp;

        if !info_ptr.is_null() {
            memset(info_ptr as *mut c_void, 0, core::mem::size_of::<png_info>());
        }

        info_ptr
    }
}

/* This function frees the memory associated with a single info struct. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_destroy_info_struct(
    png_ptr: png_const_structrp,
    info_ptr_ptr: png_infopp,
) {
    unsafe {
        let mut info_ptr: png_inforp = core::ptr::null_mut();

        if png_ptr.is_null() {
            return;
        }

        if !info_ptr_ptr.is_null() {
            info_ptr = *info_ptr_ptr;
        }

        if !info_ptr.is_null() {
            /* Do this first in case of an error below. */
            *info_ptr_ptr = core::ptr::null_mut();

            png_free_data(png_ptr, info_ptr, PNG_FREE_ALL, -1);
            memset(info_ptr as *mut c_void, 0, core::mem::size_of::<png_info>());
            png_free(png_ptr, info_ptr as png_voidp);
        }
    }
}

/* Initialize the info structure. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_info_init_3(
    ptr_ptr: png_infopp,
    png_info_struct_size: usize,
) {
    unsafe {
        let mut info_ptr: png_inforp = *ptr_ptr;

        if info_ptr.is_null() {
            return;
        }

        if core::mem::size_of::<png_info>() > png_info_struct_size {
            *ptr_ptr = core::ptr::null_mut();
            /* The following line is why this API should not be used: */
            free(info_ptr as *mut c_void);
            info_ptr = png_malloc_base(core::ptr::null(), core::mem::size_of::<png_info>())
                as png_inforp;
            if info_ptr.is_null() {
                return;
            }
            *ptr_ptr = info_ptr;
        }

        /* Set everything to 0 */
        memset(info_ptr as *mut c_void, 0, core::mem::size_of::<png_info>());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_data_freer(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    freer: c_int,
    mask: png_uint_32,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        if freer == PNG_DESTROY_WILL_FREE_DATA {
            (*info_ptr).free_me |= mask;
        } else if freer == PNG_USER_WILL_FREE_DATA {
            (*info_ptr).free_me &= !mask;
        } else {
            png_error(png_ptr, c"Unknown freer parameter in png_data_freer".as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_free_data(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mut mask: png_uint_32,
    num: c_int,
) {
    unsafe {
        if png_ptr.is_null() || info_ptr.is_null() {
            return;
        }

        /* Free text item num or (if num == -1) all text items */
        if !(*info_ptr).text.is_null()
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

            if !(*info_ptr).pcal_params.is_null() {
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
        if !(*info_ptr).splt_palettes.is_null()
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
                (*(*info_ptr).splt_palettes.offset(num as isize)).entries =
                    core::ptr::null_mut();
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

        if !(*info_ptr).unknown_chunks.is_null()
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
            if !(*info_ptr).row_pointers.is_null() {
                let mut row: png_uint_32 = 0;
                while row < (*info_ptr).height {
                    png_free(png_ptr, *(*info_ptr).row_pointers.offset(row as isize) as png_voidp);
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
}

/* This function returns a pointer to the io_ptr associated with the user
 * functions.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_io_ptr(png_ptr: png_const_structrp) -> png_voidp {
    unsafe {
        if png_ptr.is_null() {
            return core::ptr::null_mut();
        }

        (*png_ptr).io_ptr
    }
}

/* Initialize the default input/output functions for the PNG file. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_init_io(png_ptr: png_structrp, fp: *mut FILE) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        (*png_ptr).io_ptr = fp as png_voidp;
    }
}

/* PNG signed integers are saved in 32-bit 2's complement format. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_save_int_32(buf: png_bytep, i: png_int_32) {
    unsafe {
        png_save_uint_32(buf, i as png_uint_32);
    }
}

/* Convert the supplied time into an RFC 1123 string. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_convert_to_rfc1123_buffer(
    out: *mut c_char,
    ptime: png_const_timep,
) -> c_int {
    unsafe {
        static short_months: [[c_char; 4]; 12] = [
            [74, 97, 110, 0],
            [70, 101, 98, 0],
            [77, 97, 114, 0],
            [65, 112, 114, 0],
            [77, 97, 121, 0],
            [74, 117, 110, 0],
            [74, 117, 108, 0],
            [65, 117, 103, 0],
            [83, 101, 112, 0],
            [79, 99, 116, 0],
            [78, 111, 118, 0],
            [68, 101, 99, 0],
        ];

        if out.is_null() {
            return 0;
        }

        if (*ptime).year > 9999
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

            /* APPEND_NUMBER(PNG_NUMBER_FORMAT_u, (unsigned)ptime->day); */
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
            /* APPEND(' '); */
            if pos < 28 {
                *out.add(pos) = ' ' as c_char;
                pos += 1;
            }
            /* APPEND_STRING(short_months[(ptime->month - 1)]); */
            pos = png_safecat(
                out,
                29,
                pos,
                short_months[((*ptime).month - 1) as usize].as_ptr(),
            );
            if pos < 28 {
                *out.add(pos) = ' ' as c_char;
                pos += 1;
            }
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
                *out.add(pos) = ' ' as c_char;
                pos += 1;
            }
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
                *out.add(pos) = ':' as c_char;
                pos += 1;
            }
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
                *out.add(pos) = ':' as c_char;
                pos += 1;
            }
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
            /* APPEND_STRING(" +0000"); */
            pos = png_safecat(out, 29, pos, c" +0000".as_ptr());
            PNG_UNUSED(pos);
        }

        1
    }
}

/* Original API that uses a private buffer in png_struct. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_convert_to_rfc1123(
    png_ptr: png_structrp,
    ptime: png_const_timep,
) -> png_const_charp {
    unsafe {
        if !png_ptr.is_null() {
            /* The only failure above if png_ptr != NULL is from an invalid ptime */
            if png_convert_to_rfc1123_buffer((*png_ptr).time_buffer.as_mut_ptr(), ptime) == 0 {
                png_warning(png_ptr, c"Ignoring invalid time value".as_ptr());
            } else {
                return (*png_ptr).time_buffer.as_ptr();
            }
        }

        core::ptr::null()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_copyright(png_ptr: png_const_structrp) -> png_const_charp {
    unsafe {
        PNG_UNUSED(png_ptr);
        c"\nlibpng version 1.6.59.git\nCopyright (c) 2018-2026 Cosmin Truta\nCopyright (c) 1998-2002,2004,2006-2018 Glenn Randers-Pehrson\nCopyright (c) 1996-1997 Andreas Dilger\nCopyright (c) 1995-1996 Guy Eric Schalnat, Group 42, Inc.\n".as_ptr()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_libpng_ver(png_ptr: png_const_structrp) -> png_const_charp {
    unsafe {
        /* Version of *.c files used when building libpng */
        png_get_header_ver(png_ptr)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_header_ver(png_ptr: png_const_structrp) -> png_const_charp {
    unsafe {
        PNG_UNUSED(png_ptr);
        PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_header_version(
    png_ptr: png_const_structrp,
) -> png_const_charp {
    unsafe {
        /* Returns longer string containing both version and date */
        PNG_UNUSED(png_ptr);
        /* PNG_HEADER_VERSION_STRING PNG_STRING_NEWLINE with __STDC__ and READ support. */
        c" libpng version 1.6.59.git\n\n".as_ptr()
    }
}

/* Build a grayscale palette. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_build_grayscale_palette(
    bit_depth: c_int,
    palette: png_colorp,
) {
    unsafe {
        let num_palette: c_int;
        let color_inc: c_int;
        let mut i: c_int;
        let mut v: c_int;

        if palette.is_null() {
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

        i = 0;
        v = 0;
        while i < num_palette {
            (*palette.offset(i as isize)).red = (v & 0xff) as png_byte;
            (*palette.offset(i as isize)).green = (v & 0xff) as png_byte;
            (*palette.offset(i as isize)).blue = (v & 0xff) as png_byte;
            i += 1;
            v += color_inc;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_handle_as_unknown(
    png_ptr: png_const_structrp,
    chunk_name: png_const_bytep,
) -> c_int {
    unsafe {
        /* Check chunk_name and return "keep" value if it's on the list, else 0 */
        let p_end: png_const_bytep;
        let mut p: png_const_bytep;

        if png_ptr.is_null() || chunk_name.is_null() || (*png_ptr).num_chunk_list == 0 {
            return PNG_HANDLE_CHUNK_AS_DEFAULT;
        }

        p_end = (*png_ptr).chunk_list;
        p = p_end.add(((*png_ptr).num_chunk_list * 5) as usize); /* beyond end */

        loop {
            p = p.sub(5);

            if memcmp(chunk_name as *const c_void, p as *const c_void, 4) == 0 {
                return *p.add(4) as c_int;
            }

            if !((p as usize) > (p_end as usize)) {
                break;
            }
        }

        PNG_HANDLE_CHUNK_AS_DEFAULT
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_chunk_unknown_handling(
    png_ptr: png_const_structrp,
    chunk_name: png_uint_32,
) -> c_int {
    unsafe {
        let mut chunk_string: [png_byte; 5] = [0; 5];

        PNG_CSTRING_FROM_CHUNK(chunk_string.as_mut_ptr() as *mut c_char, chunk_name);
        png_handle_as_unknown(png_ptr, chunk_string.as_ptr())
    }
}

/* This function, added to libpng-1.0.6g, is untested. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_reset_zstream(png_ptr: png_structrp) -> c_int {
    unsafe {
        if png_ptr.is_null() {
            return Z_STREAM_ERROR;
        }

        /* WARNING: this resets the window bits to the maximum! */
        inflateReset(&raw mut (*png_ptr).zstream)
    }
}

/* This function was added to libpng-1.0.7 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_access_version_number() -> png_uint_32 {
    unsafe { PNG_LIBPNG_VER as png_uint_32 }
}

/* Ensure that png_ptr->zstream.msg holds some appropriate error message string. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_zstream_error(png_ptr: png_structrp, ret: c_int) {
    unsafe {
        if (*png_ptr).zstream.msg.is_null() {
            match ret {
                Z_STREAM_END => {
                    /* Normal exit */
                    (*png_ptr).zstream.msg = c"unexpected end of LZ stream".as_ptr();
                }

                Z_NEED_DICT => {
                    (*png_ptr).zstream.msg = c"missing LZ dictionary".as_ptr();
                }

                Z_ERRNO => {
                    /* gz APIs only: should not happen */
                    (*png_ptr).zstream.msg = c"zlib IO error".as_ptr();
                }

                Z_STREAM_ERROR => {
                    /* internal libpng error */
                    (*png_ptr).zstream.msg = c"bad parameters to zlib".as_ptr();
                }

                Z_DATA_ERROR => {
                    (*png_ptr).zstream.msg = c"damaged LZ stream".as_ptr();
                }

                Z_MEM_ERROR => {
                    (*png_ptr).zstream.msg = c"insufficient memory".as_ptr();
                }

                Z_BUF_ERROR => {
                    /* End of input or output. */
                    (*png_ptr).zstream.msg = c"truncated".as_ptr();
                }

                Z_VERSION_ERROR => {
                    (*png_ptr).zstream.msg = c"unsupported zlib version".as_ptr();
                }

                PNG_UNEXPECTED_ZLIB_RETURN => {
                    (*png_ptr).zstream.msg = c"unexpected zlib return".as_ptr();
                }

                /* default, Z_OK */
                _ => {
                    (*png_ptr).zstream.msg = c"unexpected zlib return code".as_ptr();
                }
            }
        }
    }
}

/* Safely add two fixed point values setting an error flag and returning 0.5
 * on overflow.
 */
unsafe fn png_fp_add(addend0: png_int_32, addend1: png_int_32, error: *mut c_int) -> png_int_32 {
    unsafe {
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
}

/* As above but calculate addend0-addend1. */
unsafe fn png_fp_sub(addend0: png_int_32, addend1: png_int_32, error: *mut c_int) -> png_int_32 {
    unsafe {
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
}

/* Safely add three integers.  Returns 0 on success, 1 on overflow. */
unsafe fn png_safe_add(
    addend0_and_result: *mut png_int_32,
    addend1: png_int_32,
    addend2: png_int_32,
) -> c_int {
    unsafe {
        let mut error: c_int = 0;
        let result: c_int = png_fp_add(
            *addend0_and_result,
            png_fp_add(addend1, addend2, &raw mut error),
            &raw mut error,
        );
        if error == 0 {
            *addend0_and_result = result;
        }
        error
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_xy_from_XYZ(xy: *mut png_xy, XYZ: *const png_XYZ) -> c_int {
    unsafe {
        /* NOTE: returns 0 on success, 1 means error. */
        let mut d: png_int_32;
        let dred: png_int_32;
        let dgreen: png_int_32;
        let dblue: png_int_32;
        let dwhite: png_int_32;
        let whiteX: png_int_32;
        let whiteY: png_int_32;

        d = (*XYZ).red_X;
        if png_safe_add(&raw mut d, (*XYZ).red_Y, (*XYZ).red_Z) != 0 {
            return 1;
        }
        dred = d;
        if png_muldiv(&raw mut (*xy).redx, (*XYZ).red_X, PNG_FP_1, dred) == 0 {
            return 1;
        }
        if png_muldiv(&raw mut (*xy).redy, (*XYZ).red_Y, PNG_FP_1, dred) == 0 {
            return 1;
        }

        d = (*XYZ).green_X;
        if png_safe_add(&raw mut d, (*XYZ).green_Y, (*XYZ).green_Z) != 0 {
            return 1;
        }
        dgreen = d;
        if png_muldiv(&raw mut (*xy).greenx, (*XYZ).green_X, PNG_FP_1, dgreen) == 0 {
            return 1;
        }
        if png_muldiv(&raw mut (*xy).greeny, (*XYZ).green_Y, PNG_FP_1, dgreen) == 0 {
            return 1;
        }

        d = (*XYZ).blue_X;
        if png_safe_add(&raw mut d, (*XYZ).blue_Y, (*XYZ).blue_Z) != 0 {
            return 1;
        }
        dblue = d;
        if png_muldiv(&raw mut (*xy).bluex, (*XYZ).blue_X, PNG_FP_1, dblue) == 0 {
            return 1;
        }
        if png_muldiv(&raw mut (*xy).bluey, (*XYZ).blue_Y, PNG_FP_1, dblue) == 0 {
            return 1;
        }

        d = dblue;
        if png_safe_add(&raw mut d, dred, dgreen) != 0 {
            return 1;
        }
        dwhite = d;

        d = (*XYZ).red_X;
        if png_safe_add(&raw mut d, (*XYZ).green_X, (*XYZ).blue_X) != 0 {
            return 1;
        }
        whiteX = d;

        d = (*XYZ).red_Y;
        if png_safe_add(&raw mut d, (*XYZ).green_Y, (*XYZ).blue_Y) != 0 {
            return 1;
        }
        whiteY = d;

        if png_muldiv(&raw mut (*xy).whitex, whiteX, PNG_FP_1, dwhite) == 0 {
            return 1;
        }
        if png_muldiv(&raw mut (*xy).whitey, whiteY, PNG_FP_1, dwhite) == 0 {
            return 1;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_XYZ_from_xy(XYZ: *mut png_XYZ, xy: *const png_xy) -> c_int {
    unsafe {
        /* NOTE: returns 0 on success, 1 means error. */
        let mut red_inverse: png_fixed_point = 0;
        let mut green_inverse: png_fixed_point = 0;
        let blue_scale: png_fixed_point;
        let mut left: png_fixed_point = 0;
        let mut right: png_fixed_point = 0;
        let denominator: png_fixed_point;

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

            if png_muldiv(&raw mut left, (*xy).greenx - (*xy).bluex, (*xy).redy - (*xy).bluey, 8)
                == 0
            {
                return 1;
            }
            if png_muldiv(&raw mut right, (*xy).greeny - (*xy).bluey, (*xy).redx - (*xy).bluex, 8)
                == 0
            {
                return 1;
            }
            denominator = png_fp_sub(left, right, &raw mut error);
            if error != 0 {
                return 1;
            }

            /* Now find the red numerator. */
            if png_muldiv(&raw mut left, (*xy).greenx - (*xy).bluex, (*xy).whitey - (*xy).bluey, 8)
                == 0
            {
                return 1;
            }
            if png_muldiv(&raw mut right, (*xy).greeny - (*xy).bluey, (*xy).whitex - (*xy).bluex, 8)
                == 0
            {
                return 1;
            }

            if png_muldiv(
                &raw mut red_inverse,
                (*xy).whitey,
                denominator,
                png_fp_sub(left, right, &raw mut error),
            ) == 0
                || error != 0
                || red_inverse <= (*xy).whitey
            {
                return 1;
            }

            /* Similarly for green_inverse: */
            if png_muldiv(&raw mut left, (*xy).redy - (*xy).bluey, (*xy).whitex - (*xy).bluex, 8)
                == 0
            {
                return 1;
            }
            if png_muldiv(&raw mut right, (*xy).redx - (*xy).bluex, (*xy).whitey - (*xy).bluey, 8)
                == 0
            {
                return 1;
            }
            if png_muldiv(
                &raw mut green_inverse,
                (*xy).whitey,
                denominator,
                png_fp_sub(left, right, &raw mut error),
            ) == 0
                || error != 0
                || green_inverse <= (*xy).whitey
            {
                return 1;
            }

            blue_scale = png_fp_sub(
                png_fp_sub(
                    png_reciprocal((*xy).whitey),
                    png_reciprocal(red_inverse),
                    &raw mut error,
                ),
                png_reciprocal(green_inverse),
                &raw mut error,
            );
            if error != 0 || blue_scale <= 0 {
                return 1;
            }
        }

        if png_muldiv(&raw mut (*XYZ).red_X, (*xy).redx, PNG_FP_1, red_inverse) == 0 {
            return 1;
        }
        if png_muldiv(&raw mut (*XYZ).red_Y, (*xy).redy, PNG_FP_1, red_inverse) == 0 {
            return 1;
        }
        if png_muldiv(
            &raw mut (*XYZ).red_Z,
            PNG_FP_1 - (*xy).redx - (*xy).redy,
            PNG_FP_1,
            red_inverse,
        ) == 0
        {
            return 1;
        }

        if png_muldiv(&raw mut (*XYZ).green_X, (*xy).greenx, PNG_FP_1, green_inverse) == 0 {
            return 1;
        }
        if png_muldiv(&raw mut (*XYZ).green_Y, (*xy).greeny, PNG_FP_1, green_inverse) == 0 {
            return 1;
        }
        if png_muldiv(
            &raw mut (*XYZ).green_Z,
            PNG_FP_1 - (*xy).greenx - (*xy).greeny,
            PNG_FP_1,
            green_inverse,
        ) == 0
        {
            return 1;
        }

        if png_muldiv(&raw mut (*XYZ).blue_X, (*xy).bluex, blue_scale, PNG_FP_1) == 0 {
            return 1;
        }
        if png_muldiv(&raw mut (*XYZ).blue_Y, (*xy).bluey, blue_scale, PNG_FP_1) == 0 {
            return 1;
        }
        if png_muldiv(
            &raw mut (*XYZ).blue_Z,
            PNG_FP_1 - (*xy).bluex - (*xy).bluey,
            blue_scale,
            PNG_FP_1,
        ) == 0
        {
            return 1;
        }

        0 /*success*/
    }
}

/* Error message generation */
unsafe fn png_icc_tag_char(mut byte: png_uint_32) -> c_char {
    byte &= 0xff;
    if byte >= 32 && byte <= 126 {
        byte as c_char
    } else {
        '?' as c_char
    }
}

unsafe fn png_icc_tag_name(name: *mut c_char, tag: png_uint_32) {
    unsafe {
        *name.add(0) = '\'' as c_char;
        *name.add(1) = png_icc_tag_char(tag >> 24);
        *name.add(2) = png_icc_tag_char(tag >> 16);
        *name.add(3) = png_icc_tag_char(tag >> 8);
        *name.add(4) = png_icc_tag_char(tag);
        *name.add(5) = '\'' as c_char;
    }
}

unsafe fn is_ICC_signature_char(it: png_alloc_size_t) -> c_int {
    (it == 32
        || (it >= 48 && it <= 57)
        || (it >= 65 && it <= 90)
        || (it >= 97 && it <= 122)) as c_int
}

unsafe fn is_ICC_signature(it: png_alloc_size_t) -> c_int {
    unsafe {
        (is_ICC_signature_char(it >> 24) != 0
            && is_ICC_signature_char((it >> 16) & 0xff) != 0
            && is_ICC_signature_char((it >> 8) & 0xff) != 0
            && is_ICC_signature_char(it & 0xff) != 0) as c_int
    }
}

unsafe fn png_icc_profile_error(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    value: png_alloc_size_t,
    reason: png_const_charp,
) -> c_int {
    unsafe {
        let mut pos: usize;
        let mut message: [c_char; 196] = [0; 196]; /* see below for calculation */

        pos = png_safecat(
            message.as_mut_ptr(),
            core::mem::size_of_val(&message),
            0,
            c"profile '".as_ptr(),
        ); /* 9 chars */
        pos = png_safecat(message.as_mut_ptr(), pos + 79, pos, name); /* Truncate to 79 chars */
        pos = png_safecat(
            message.as_mut_ptr(),
            core::mem::size_of_val(&message),
            pos,
            c"': ".as_ptr(),
        ); /* +2 = 90 */
        if is_ICC_signature(value) != 0 {
            /* So 'value' is at most 4 bytes and the following cast is safe */
            png_icc_tag_name(message.as_mut_ptr().add(pos), value as png_uint_32);
            pos += 6; /* total +8; less than the else clause */
            message[pos] = ':' as c_char;
            pos += 1;
            message[pos] = ' ' as c_char;
            pos += 1;
        } else {
            let mut number: [c_char; PNG_NUMBER_BUFFER_SIZE] = [0; PNG_NUMBER_BUFFER_SIZE]; /* +24 = 114 */

            pos = png_safecat(
                message.as_mut_ptr(),
                core::mem::size_of_val(&message),
                pos,
                png_format_number(
                    number.as_ptr(),
                    number.as_mut_ptr().add(core::mem::size_of_val(&number)),
                    PNG_NUMBER_FORMAT_x,
                    value,
                ),
            );
            pos = png_safecat(
                message.as_mut_ptr(),
                core::mem::size_of_val(&message),
                pos,
                c"h: ".as_ptr(),
            ); /* +2 = 116 */
        }
        /* The 'reason' is an arbitrary message, allow +79 maximum 195 */
        pos = png_safecat(
            message.as_mut_ptr(),
            core::mem::size_of_val(&message),
            pos,
            reason,
        );
        PNG_UNUSED(pos);

        png_chunk_benign_error(png_ptr, message.as_ptr());

        0
    }
}

/* Encoded value of D50 as an ICC XYZNumber. */
static D50_nCIEXYZ: [png_byte; 12] = [
    0x00, 0x00, 0xf6, 0xd6, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xd3, 0x2d,
];

unsafe fn icc_check_length(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    profile_length: png_uint_32,
) -> c_int {
    unsafe {
        if profile_length < 132 {
            return png_icc_profile_error(
                png_ptr,
                name,
                profile_length as png_alloc_size_t,
                c"too short".as_ptr(),
            );
        }
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_icc_check_length(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    profile_length: png_uint_32,
) -> c_int {
    unsafe {
        if icc_check_length(png_ptr, name, profile_length) == 0 {
            return 0;
        }

        if profile_length as png_alloc_size_t > png_chunk_max(png_ptr) {
            return png_icc_profile_error(
                png_ptr,
                name,
                profile_length as png_alloc_size_t,
                c"profile too long".as_ptr(),
            );
        }

        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_icc_check_header(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    profile_length: png_uint_32,
    profile: png_const_bytep,
    color_type: c_int,
) -> c_int {
    unsafe {
        let mut temp: png_uint_32;

        temp = png_get_uint_32(profile);
        if temp != profile_length {
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                c"length does not match profile".as_ptr(),
            );
        }

        temp = *(profile.add(8)) as png_uint_32;
        if temp > 3 && (profile_length & 3) != 0 {
            return png_icc_profile_error(
                png_ptr,
                name,
                profile_length as png_alloc_size_t,
                c"invalid length".as_ptr(),
            );
        }

        temp = png_get_uint_32(profile.add(128)); /* tag count: 12 bytes/tag */
        if temp > 357913930 || profile_length < 132 + 12 * temp {
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                c"tag count too large".as_ptr(),
            );
        }

        temp = png_get_uint_32(profile.add(64));
        if temp >= 0xffff {
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                c"invalid rendering intent".as_ptr(),
            );
        }

        if temp >= PNG_sRGB_INTENT_LAST as png_uint_32 {
            png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                c"intent outside defined range".as_ptr(),
            );
        }

        temp = png_get_uint_32(profile.add(36)); /* signature 'ascp' */
        if temp != 0x61637370 {
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                c"invalid signature".as_ptr(),
            );
        }

        if memcmp(
            profile.add(68) as *const c_void,
            D50_nCIEXYZ.as_ptr() as *const c_void,
            12,
        ) != 0
        {
            png_icc_profile_error(
                png_ptr,
                name,
                0, /*no tag value*/
                c"PCS illuminant is not D50".as_ptr(),
            );
        }

        temp = png_get_uint_32(profile.add(16)); /* data colour space field */
        match temp {
            0x52474220 => {
                /* 'RGB ' */
                if (color_type & PNG_COLOR_MASK_COLOR) == 0 {
                    return png_icc_profile_error(
                        png_ptr,
                        name,
                        temp as png_alloc_size_t,
                        c"RGB color space not permitted on grayscale PNG".as_ptr(),
                    );
                }
            }

            0x47524159 => {
                /* 'GRAY' */
                if (color_type & PNG_COLOR_MASK_COLOR) != 0 {
                    return png_icc_profile_error(
                        png_ptr,
                        name,
                        temp as png_alloc_size_t,
                        c"Gray color space not permitted on RGB PNG".as_ptr(),
                    );
                }
            }

            _ => {
                return png_icc_profile_error(
                    png_ptr,
                    name,
                    temp as png_alloc_size_t,
                    c"invalid ICC profile color space".as_ptr(),
                );
            }
        }

        temp = png_get_uint_32(profile.add(12)); /* profile/device class */
        match temp {
            0x73636e72 | 0x6d6e7472 | 0x70727472 | 0x73706163 => {
                /* 'scnr' 'mntr' 'prtr' 'spac' - All supported */
            }

            0x61627374 => {
                /* 'abst' - May not be embedded in an image */
                return png_icc_profile_error(
                    png_ptr,
                    name,
                    temp as png_alloc_size_t,
                    c"invalid embedded Abstract ICC profile".as_ptr(),
                );
            }

            0x6c696e6b => {
                /* 'link' */
                return png_icc_profile_error(
                    png_ptr,
                    name,
                    temp as png_alloc_size_t,
                    c"unexpected DeviceLink ICC profile class".as_ptr(),
                );
            }

            0x6e6d636c => {
                /* 'nmcl' */
                png_icc_profile_error(
                    png_ptr,
                    name,
                    temp as png_alloc_size_t,
                    c"unexpected NamedColor ICC profile class".as_ptr(),
                );
            }

            _ => {
                png_icc_profile_error(
                    png_ptr,
                    name,
                    temp as png_alloc_size_t,
                    c"unrecognized ICC profile class".as_ptr(),
                );
            }
        }

        temp = png_get_uint_32(profile.add(20));
        match temp {
            0x58595a20 | 0x4c616220 => {
                /* 'XYZ ' 'Lab ' */
            }

            _ => {
                return png_icc_profile_error(
                    png_ptr,
                    name,
                    temp as png_alloc_size_t,
                    c"unexpected ICC PCS encoding".as_ptr(),
                );
            }
        }

        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_icc_check_tag_table(
    png_ptr: png_const_structrp,
    name: png_const_charp,
    profile_length: png_uint_32,
    profile: png_const_bytep,
) -> c_int {
    unsafe {
        let tag_count: png_uint_32 = png_get_uint_32(profile.add(128));
        let mut itag: png_uint_32;
        let mut tag: png_const_bytep = profile.add(132); /* The first tag */

        itag = 0;
        while itag < tag_count {
            let tag_id: png_uint_32 = png_get_uint_32(tag.add(0));
            let tag_start: png_uint_32 = png_get_uint_32(tag.add(4)); /* must be aligned */
            let tag_length: png_uint_32 = png_get_uint_32(tag.add(8)); /* not padded */

            if tag_start > profile_length || tag_length > profile_length - tag_start {
                return png_icc_profile_error(
                    png_ptr,
                    name,
                    tag_id as png_alloc_size_t,
                    c"ICC profile tag outside profile".as_ptr(),
                );
            }

            if (tag_start & 3) != 0 {
                png_icc_profile_error(
                    png_ptr,
                    name,
                    tag_id as png_alloc_size_t,
                    c"ICC profile tag start not a multiple of 4".as_ptr(),
                );
            }

            itag += 1;
            tag = tag.add(12);
        }

        1 /* success, maybe with warnings */
    }
}

unsafe fn have_chromaticities(png_ptr: png_const_structrp) -> c_int {
    unsafe {
        /* Handle new PNGv3 chunks and the precedence rules. */
        if png_file_has_chunk(png_ptr, PNG_INDEX_mDCV) {
            return 1;
        }

        if png_file_has_chunk(png_ptr, PNG_INDEX_sRGB) {
            return 0;
        }

        if png_file_has_chunk(png_ptr, PNG_INDEX_cHRM) {
            return 1;
        }

        0 /* sRGB defaults */
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_rgb_coefficients(png_ptr: png_structrp) {
    unsafe {
        /* Set the rgb_to_gray coefficients from the colorspace if available. */
        if (*png_ptr).rgb_to_gray_coefficients_set == 0 {
            let mut xyz: png_XYZ = png_XYZ::default();

            if have_chromaticities(png_ptr) != 0
                && png_XYZ_from_xy(&raw mut xyz, &raw const (*png_ptr).chromaticities) == 0
            {
                /* Get them from the Y values of the colorspace colorants. */
                let mut r: png_fixed_point = xyz.red_Y;
                let mut g: png_fixed_point = xyz.green_Y;
                let mut b: png_fixed_point = xyz.blue_Y;
                let total: png_fixed_point = r + g + b;

                if total > 0
                    && r >= 0
                    && png_muldiv(&raw mut r, r, 32768, total) != 0
                    && r >= 0
                    && r <= 32768
                    && g >= 0
                    && png_muldiv(&raw mut g, g, 32768, total) != 0
                    && g >= 0
                    && g <= 32768
                    && b >= 0
                    && png_muldiv(&raw mut b, b, 32768, total) != 0
                    && b >= 0
                    && b <= 32768
                    && r + g + b <= 32769
                {
                    let mut add: c_int = 0;

                    if r + g + b > 32768 {
                        add = -1;
                    } else if r + g + b < 32768 {
                        add = 1;
                    }

                    if add != 0 {
                        if g >= r && g >= b {
                            g += add;
                        } else if r >= g && r >= b {
                            r += add;
                        } else {
                            b += add;
                        }
                    }

                    /* Check for an internal error. */
                    if r + g + b != 32768 {
                        png_error(png_ptr, c"internal error handling cHRM coefficients".as_ptr());
                    } else {
                        (*png_ptr).rgb_to_gray_red_coeff = r as png_uint_16;
                        (*png_ptr).rgb_to_gray_green_coeff = g as png_uint_16;
                    }
                }
            } else {
                /* Use the historical REC 709 (etc) values: */
                (*png_ptr).rgb_to_gray_red_coeff = 6968;
                (*png_ptr).rgb_to_gray_green_coeff = 23434;
                /* png_ptr->rgb_to_gray_blue_coeff  = 2366; */
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_check_IHDR(
    png_ptr: png_const_structrp,
    width: png_uint_32,
    height: png_uint_32,
    bit_depth: c_int,
    color_type: c_int,
    interlace_type: c_int,
    compression_type: c_int,
    filter_type: c_int,
) {
    unsafe {
        let mut error: c_int = 0;

        /* Check for width and height valid values */
        if width == 0 {
            png_warning(png_ptr, c"Image width is zero in IHDR".as_ptr());
            error = 1;
        }

        if width > PNG_UINT_31_MAX {
            png_warning(png_ptr, c"Invalid image width in IHDR".as_ptr());
            error = 1;
        }

        if ((width as png_alloc_size_t + 7) & !(7 as png_alloc_size_t))
            > ((((PNG_SIZE_MAX - 48 - 1) / 8) - 1))
        {
            png_warning(png_ptr, c"Image width is too large for this architecture".as_ptr());
            error = 1;
        }

        if width > (*png_ptr).user_width_max {
            png_warning(png_ptr, c"Image width exceeds user limit in IHDR".as_ptr());
            error = 1;
        }

        if height == 0 {
            png_warning(png_ptr, c"Image height is zero in IHDR".as_ptr());
            error = 1;
        }

        if height > PNG_UINT_31_MAX {
            png_warning(png_ptr, c"Invalid image height in IHDR".as_ptr());
            error = 1;
        }

        if height > (*png_ptr).user_height_max {
            png_warning(png_ptr, c"Image height exceeds user limit in IHDR".as_ptr());
            error = 1;
        }

        /* Check other values */
        if bit_depth != 1
            && bit_depth != 2
            && bit_depth != 4
            && bit_depth != 8
            && bit_depth != 16
        {
            png_warning(png_ptr, c"Invalid bit depth in IHDR".as_ptr());
            error = 1;
        }

        if color_type < 0 || color_type == 1 || color_type == 5 || color_type > 6 {
            png_warning(png_ptr, c"Invalid color type in IHDR".as_ptr());
            error = 1;
        }

        if ((color_type == PNG_COLOR_TYPE_PALETTE) && bit_depth > 8)
            || ((color_type == PNG_COLOR_TYPE_RGB
                || color_type == PNG_COLOR_TYPE_GRAY_ALPHA
                || color_type == PNG_COLOR_TYPE_RGB_ALPHA)
                && bit_depth < 8)
        {
            png_warning(png_ptr, c"Invalid color type/bit depth combination in IHDR".as_ptr());
            error = 1;
        }

        if interlace_type >= PNG_INTERLACE_LAST {
            png_warning(png_ptr, c"Unknown interlace method in IHDR".as_ptr());
            error = 1;
        }

        if compression_type != PNG_COMPRESSION_TYPE_BASE {
            png_warning(png_ptr, c"Unknown compression method in IHDR".as_ptr());
            error = 1;
        }

        /* PNG_MNG_FEATURES_SUPPORTED */
        if ((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) != 0
            && (*png_ptr).mng_features_permitted != 0
        {
            png_warning(png_ptr, c"MNG features are not allowed in a PNG datastream".as_ptr());
        }

        if filter_type != PNG_FILTER_TYPE_BASE {
            if !(((*png_ptr).mng_features_permitted & PNG_FLAG_MNG_FILTER_64) != 0
                && (filter_type == PNG_INTRAPIXEL_DIFFERENCING)
                && (((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) == 0)
                && (color_type == PNG_COLOR_TYPE_RGB
                    || color_type == PNG_COLOR_TYPE_RGB_ALPHA))
            {
                png_warning(png_ptr, c"Unknown filter method in IHDR".as_ptr());
                error = 1;
            }

            if ((*png_ptr).mode & PNG_HAVE_PNG_SIGNATURE) != 0 {
                png_warning(png_ptr, c"Invalid filter method in IHDR".as_ptr());
                error = 1;
            }
        }

        if error == 1 {
            png_error(png_ptr, c"Invalid IHDR data".as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_check_fp_number(
    string: png_const_charp,
    size: usize,
    statep: *mut c_int,
    whereami: *mut usize,
) -> c_int {
    unsafe {
        let mut state: c_int = *statep;
        let mut i: usize = *whereami;

        'fp_end: while i < size {
            let r#type: c_int;
            /* First find the type of the next character */
            match *string.add(i) as u8 as c_int {
                43 => r#type = PNG_FP_SAW_SIGN,
                45 => r#type = PNG_FP_SAW_SIGN + PNG_FP_NEGATIVE,
                46 => r#type = PNG_FP_SAW_DOT,
                48 => r#type = PNG_FP_SAW_DIGIT,
                49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 | 57 => {
                    r#type = PNG_FP_SAW_DIGIT + PNG_FP_NONZERO
                }
                69 | 101 => r#type = PNG_FP_SAW_E,
                _ => break 'fp_end,
            }

            /* Now deal with this type according to the current state. */
            match (state & PNG_FP_STATE) + (r#type & PNG_FP_SAW_ANY) {
                x if x == PNG_FP_INTEGER + PNG_FP_SAW_SIGN => {
                    if (state & PNG_FP_SAW_ANY) != 0 {
                        break 'fp_end; /* not a part of the number */
                    }

                    state |= r#type;
                }

                x if x == PNG_FP_INTEGER + PNG_FP_SAW_DOT => {
                    /* Ok as trailer, ok as lead of fraction. */
                    if (state & PNG_FP_SAW_DOT) != 0 {
                        break 'fp_end;
                    } else if (state & PNG_FP_SAW_DIGIT) != 0 {
                        state |= r#type;
                    } else {
                        state = (PNG_FP_FRACTION | r#type) | (state & PNG_FP_STICKY);
                    }
                }

                x if x == PNG_FP_INTEGER + PNG_FP_SAW_DIGIT => {
                    if (state & PNG_FP_SAW_DOT) != 0 {
                        state = (PNG_FP_FRACTION | PNG_FP_SAW_DOT) | (state & PNG_FP_STICKY);
                    }

                    state |= r#type | PNG_FP_WAS_VALID;
                }

                x if x == PNG_FP_INTEGER + PNG_FP_SAW_E => {
                    if (state & PNG_FP_SAW_DIGIT) == 0 {
                        break 'fp_end;
                    }

                    state = PNG_FP_EXPONENT | (state & PNG_FP_STICKY);
                }

                x if x == PNG_FP_FRACTION + PNG_FP_SAW_DIGIT => {
                    state |= r#type | PNG_FP_WAS_VALID;
                }

                x if x == PNG_FP_FRACTION + PNG_FP_SAW_E => {
                    if (state & PNG_FP_SAW_DIGIT) == 0 {
                        break 'fp_end;
                    }

                    state = PNG_FP_EXPONENT | (state & PNG_FP_STICKY);
                }

                x if x == PNG_FP_EXPONENT + PNG_FP_SAW_SIGN => {
                    if (state & PNG_FP_SAW_ANY) != 0 {
                        break 'fp_end; /* not a part of the number */
                    }

                    state |= PNG_FP_SAW_SIGN;
                }

                x if x == PNG_FP_EXPONENT + PNG_FP_SAW_DIGIT => {
                    state |= PNG_FP_SAW_DIGIT | PNG_FP_WAS_VALID;
                }

                _ => break 'fp_end, /* I.e. break 2 */
            }

            /* The character seems ok, continue. */
            i += 1;
        }

        /* Here at the end, update the state and return the correct return code. */
        *statep = state;
        *whereami = i;

        ((state & PNG_FP_SAW_DIGIT) != 0) as c_int
    }
}

/* The same but for a complete string. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_check_fp_string(string: png_const_charp, size: usize) -> c_int {
    unsafe {
        let mut state: c_int = 0;
        let mut char_index: usize = 0;

        if png_check_fp_number(string, size, &raw mut state, &raw mut char_index) != 0
            && (char_index == size || *string.add(char_index) == 0)
        {
            return state; /* must be non-zero - see above */
        }

        0 /* i.e. fail */
    }
}

/* Utility used below - a simple accurate power of ten from an integral exponent. */
unsafe fn png_pow10(mut power: c_int) -> f64 {
    let mut recip: c_int = 0;
    let mut d: f64 = 1.0;

    if power < 0 {
        if power < DBL_MIN_10_EXP {
            return 0.0;
        }
        recip = 1;
        power = -power;
    }

    if power > 0 {
        /* Decompose power bitwise. */
        let mut mult: f64 = 10.0;
        loop {
            if (power & 1) != 0 {
                d *= mult;
            }
            mult *= mult;
            power >>= 1;

            if !(power > 0) {
                break;
            }
        }

        if recip != 0 {
            d = 1.0 / d;
        }
    }

    d
}

/* Function to format a floating point value in ASCII with a given precision. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_ascii_from_fp(
    png_ptr: png_const_structrp,
    mut ascii: png_charp,
    mut size: usize,
    mut fp: f64,
    mut precision: c_uint,
) {
    unsafe {
        if precision < 1 {
            precision = DBL_DIG as c_uint;
        }

        if precision > (DBL_DIG + 1) as c_uint {
            precision = (DBL_DIG + 1) as c_uint;
        }

        /* Basic sanity checks */
        if size >= (precision + 5) as usize {
            if fp < 0.0 {
                fp = -fp;
                *ascii = 45;
                ascii = ascii.add(1); /* '-'  PLUS 1 TOTAL 1 */
                size -= 1;
            }

            if fp >= DBL_MIN && fp <= DBL_MAX {
                let mut exp_b10: c_int = 0; /* A base 10 exponent */
                let mut base: f64; /* 10^exp_b10 */

                frexp(fp, &raw mut exp_b10); /* exponent to base 2 */

                exp_b10 = (exp_b10 * 77) >> 8; /* <= exponent to base 10 */

                base = png_pow10(exp_b10); /* May underflow */

                while base < DBL_MIN || base < fp {
                    let test: f64 = png_pow10(exp_b10 + 1);

                    if test <= DBL_MAX {
                        exp_b10 += 1;
                        base = test;
                    } else {
                        break;
                    }
                }

                fp /= base;
                while fp >= 1.0 {
                    fp /= 10.0;
                    exp_b10 += 1;
                }

                {
                    let mut czero: c_uint;
                    let mut clead: c_uint;
                    let mut cdigits: c_uint;
                    let mut exponent: [c_char; 10] = [0; 10];

                    if exp_b10 < 0 && exp_b10 > -3 {
                        czero = (0u32).wrapping_sub(exp_b10 as c_uint); /* PLUS 2 digits: TOTAL 3 */
                        exp_b10 = 0; /* Dot added below before first output. */
                    } else {
                        czero = 0; /* No zeros to add */
                    }

                    clead = czero; /* Count of leading zeros */
                    cdigits = 0; /* Count of digits in list. */

                    loop {
                        let mut d: f64 = 0.0;

                        fp *= 10.0;
                        if (cdigits + czero + 1) < (precision + clead) {
                            fp = modf(fp, &raw mut d);
                        } else {
                            d = floor(fp + 0.5);

                            if d > 9.0 {
                                /* Rounding up to 10, handle that here. */
                                if czero > 0 {
                                    czero -= 1;
                                    d = 1.0;
                                    if cdigits == 0 {
                                        clead -= 1;
                                    }
                                } else {
                                    while cdigits > 0 && d > 9.0 {
                                        ascii = ascii.sub(1);
                                        let mut ch: c_int = *ascii as c_int;

                                        if exp_b10 != -1 {
                                            exp_b10 += 1;
                                        } else if ch == 46 {
                                            ascii = ascii.sub(1);
                                            ch = *ascii as c_int;
                                            size += 1;
                                            /* Advance exp_b10 to '1'. */
                                            exp_b10 = 1;
                                        }

                                        cdigits -= 1;
                                        d = (ch - 47) as f64; /* I.e. 1+(ch-48) */
                                    }

                                    if d > 9.0
                                    /* cdigits == 0 */
                                    {
                                        if exp_b10 == -1 {
                                            ascii = ascii.sub(1);
                                            let ch: c_int = *ascii as c_int;

                                            if ch == 46 {
                                                size += 1;
                                                exp_b10 = 1;
                                            }

                                            /* Else lost a leading zero, so 'exp_b10' is
                                             * still ok at (-1)
                                             */
                                        } else {
                                            exp_b10 += 1;
                                        }

                                        /* In all cases we output a '1' */
                                        d = 1.0;
                                    }
                                }
                            }
                            fp = 0.0; /* Guarantees termination below. */
                        }

                        if d == 0.0 {
                            czero += 1;
                            if cdigits == 0 {
                                clead += 1;
                            }
                        } else {
                            /* Included embedded zeros in the digit count. */
                            cdigits += czero - clead;
                            clead = 0;

                            while czero > 0 {
                                if exp_b10 != -1 {
                                    if exp_b10 == 0 {
                                        *ascii = 46;
                                        ascii = ascii.add(1);
                                        size -= 1;
                                    }
                                    /* PLUS 1: TOTAL 4 */
                                    exp_b10 -= 1;
                                }
                                *ascii = 48;
                                ascii = ascii.add(1);
                                czero -= 1;
                            }

                            if exp_b10 != -1 {
                                if exp_b10 == 0 {
                                    *ascii = 46;
                                    ascii = ascii.add(1);
                                    size -= 1; /* counted above */
                                }

                                exp_b10 -= 1;
                            }
                            *ascii = (48 + d as c_int) as c_char;
                            ascii = ascii.add(1);
                            cdigits += 1;
                        }

                        if !((cdigits + czero) < (precision + clead) && fp > DBL_MIN) {
                            break;
                        }
                    }

                    /* Check for an exponent. */
                    if exp_b10 >= -1 && exp_b10 <= 2 {
                        while exp_b10 > 0 {
                            *ascii = 48;
                            ascii = ascii.add(1);
                            exp_b10 -= 1;
                        }

                        *ascii = 0;

                        return;
                    }

                    /* Here if an exponent is required. */
                    size -= cdigits as usize;

                    *ascii = 69;
                    ascii = ascii.add(1);
                    size -= 1; /* 'E': PLUS 1 TOTAL 2+precision */

                    {
                        let uexp_b10: c_uint;

                        if exp_b10 < 0 {
                            *ascii = 45;
                            ascii = ascii.add(1);
                            size -= 1; /* '-': PLUS 1 TOTAL 3+precision */
                            uexp_b10 = (0u32).wrapping_sub(exp_b10 as c_uint);
                        } else {
                            uexp_b10 = (0u32).wrapping_add(exp_b10 as c_uint);
                        }

                        cdigits = 0;

                        let mut ue = uexp_b10;
                        while ue > 0 {
                            exponent[cdigits as usize] = (48 + ue % 10) as c_char;
                            cdigits += 1;
                            ue /= 10;
                        }
                    }

                    /* Need another size check here for the exponent digits. */
                    if size > cdigits as usize {
                        while cdigits > 0 {
                            cdigits -= 1;
                            *ascii = exponent[cdigits as usize];
                            ascii = ascii.add(1);
                        }

                        *ascii = 0;

                        return;
                    }
                }
            } else if !(fp >= DBL_MIN) {
                *ascii = 48; /* '0' */
                ascii = ascii.add(1);
                *ascii = 0;
                return;
            } else {
                *ascii = 105; /* 'i' */
                ascii = ascii.add(1);
                *ascii = 110; /* 'n' */
                ascii = ascii.add(1);
                *ascii = 102; /* 'f' */
                ascii = ascii.add(1);
                *ascii = 0;
                return;
            }
        }

        /* Here on buffer too small. */
        png_error(png_ptr, c"ASCII conversion buffer too small".as_ptr());
    }
}

/* Function to format a fixed point value in ASCII. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_ascii_from_fixed(
    png_ptr: png_const_structrp,
    mut ascii: png_charp,
    size: usize,
    fp: png_fixed_point,
) {
    unsafe {
        if size > 12 {
            let mut num: png_uint_32;

            /* Avoid overflow here on the minimum integer. */
            if fp < 0 {
                *ascii = 45;
                ascii = ascii.add(1);
                num = (-(fp as i64)) as png_uint_32;
            } else {
                num = fp as png_uint_32;
            }

            if num <= 0x80000000 {
                let mut ndigits: c_uint = 0;
                let mut first: c_uint = 16; /* flag value */
                let mut digits: [c_char; 10] = [0; 10];

                while num != 0 {
                    /* Split the low digit off num: */
                    let tmp: c_uint = num / 10;
                    num -= tmp * 10;
                    digits[ndigits as usize] = (48 + num) as c_char;
                    ndigits += 1;
                    if first == 16 && num > 0 {
                        first = ndigits;
                    }
                    num = tmp;
                }

                if ndigits > 0 {
                    while ndigits > 5 {
                        ndigits -= 1;
                        *ascii = digits[ndigits as usize];
                        ascii = ascii.add(1);
                    }
                    if first <= 5 {
                        let mut i: c_uint;
                        *ascii = 46; /* decimal point */
                        ascii = ascii.add(1);
                        i = 5;
                        while ndigits < i {
                            *ascii = 48;
                            ascii = ascii.add(1);
                            i -= 1;
                        }
                        while ndigits >= first {
                            ndigits -= 1;
                            *ascii = digits[ndigits as usize];
                            ascii = ascii.add(1);
                        }
                        /* Don't output the trailing zeros! */
                    }
                } else {
                    *ascii = 48;
                    ascii = ascii.add(1);
                }

                /* And null terminate the string: */
                *ascii = 0;
                return;
            }
        }

        /* Here on buffer too small. */
        png_error(png_ptr, c"ASCII conversion buffer too small".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_fixed(
    png_ptr: png_const_structrp,
    fp: f64,
    text: png_const_charp,
) -> png_fixed_point {
    unsafe {
        let r: f64 = floor(100000.0 * fp + 0.5);

        if r > 2147483647.0 || r < -2147483648.0 {
            png_fixed_error(png_ptr, text);
        }

        r as png_fixed_point
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_fixed_ITU(
    png_ptr: png_const_structrp,
    fp: f64,
    text: png_const_charp,
) -> png_uint_32 {
    unsafe {
        let r: f64 = floor(10000.0 * fp + 0.5);

        if r > 2147483647.0 || r < 0.0 {
            png_fixed_error(png_ptr, text);
        }

        r as png_uint_32
    }
}

/* This API takes signed arguments and rounds the result to the nearest integer. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_muldiv(
    res: png_fixed_point_p,
    a: png_fixed_point,
    times: png_int_32,
    divisor: png_int_32,
) -> c_int {
    unsafe {
        /* Return a * times / divisor, rounded. */
        if divisor != 0 {
            if a == 0 || times == 0 {
                *res = 0;
                return 1;
            } else {
                let mut r: f64 = a as f64;
                r *= times as f64;
                r /= divisor as f64;
                r = floor(r + 0.5);

                /* A png_fixed_point is a 32-bit integer. */
                if r <= 2147483647.0 && r >= -2147483648.0 {
                    *res = r as png_fixed_point;
                    return 1;
                }
            }
        }

        0
    }
}

/* Calculate a reciprocal, return 0 on div-by-zero or overflow. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_reciprocal(a: png_fixed_point) -> png_fixed_point {
    unsafe {
        let r: f64 = floor(1E10 / a as f64 + 0.5);

        if r <= 2147483647.0 && r >= -2147483648.0 {
            return r as png_fixed_point;
        }

        0 /* error/overflow */
    }
}

/* This is the shared test on whether a gamma value is 'significant'. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_gamma_significant(gamma_val: png_fixed_point) -> c_int {
    unsafe {
        (gamma_val < PNG_FP_1 - PNG_GAMMA_THRESHOLD_FIXED
            || gamma_val > PNG_FP_1 + PNG_GAMMA_THRESHOLD_FIXED) as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_reciprocal2(
    a: png_fixed_point,
    b: png_fixed_point,
) -> png_fixed_point {
    unsafe {
        /* The required result is 1/a * 1/b; the following preserves accuracy. */
        if a != 0 && b != 0 {
            let mut r: f64 = 1E15 / a as f64;
            r /= b as f64;
            r = floor(r + 0.5);

            if r <= 2147483647.0 && r >= -2147483648.0 {
                return r as png_fixed_point;
            }
        }

        0 /* overflow */
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_gamma_8bit_correct(
    value: c_uint,
    gamma_val: png_fixed_point,
) -> png_byte {
    unsafe {
        if value > 0 && value < 255 {
            let r: f64 = floor(
                255.0 * pow(value as c_int as f64 / 255.0, gamma_val as f64 * 0.00001) + 0.5,
            );
            return r as png_byte;
        }

        (value & 0xff) as png_byte
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_gamma_16bit_correct(
    value: c_uint,
    gamma_val: png_fixed_point,
) -> png_uint_16 {
    unsafe {
        if value > 0 && value < 65535 {
            let r: f64 = floor(
                65535.0 * pow(value as png_int_32 as f64 / 65535.0, gamma_val as f64 * 0.00001)
                    + 0.5,
            );
            return r as png_uint_16;
        }

        value as png_uint_16
    }
}

/* This does the right thing based on the bit_depth field of the png_struct. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_gamma_correct(
    png_ptr: png_structrp,
    value: c_uint,
    gamma_val: png_fixed_point,
) -> png_uint_16 {
    unsafe {
        if (*png_ptr).bit_depth == 8 {
            png_gamma_8bit_correct(value, gamma_val) as png_uint_16
        } else {
            png_gamma_16bit_correct(value, gamma_val)
        }
    }
}

/* Internal function to build a single 16-bit table. */
unsafe fn png_build_16bit_table(
    png_ptr: png_structrp,
    ptable: *mut png_uint_16pp,
    shift: c_uint,
    gamma_val: png_fixed_point,
) {
    unsafe {
        /* Various values derived from 'shift': */
        let num: c_uint = 1u32 << (8u32 - shift);
        let fmax: f64 = 1.0 / (((1 as png_int_32) << (16u32 - shift)) - 1) as f64;
        let max: c_uint = (1u32 << (16u32 - shift)) - 1u32;
        let max_by_2: c_uint = 1u32 << (15u32 - shift);
        let mut i: c_uint;

        let table: png_uint_16pp =
            png_calloc(png_ptr, num as usize * core::mem::size_of::<png_uint_16p>())
                as png_uint_16pp;
        *ptable = table;

        i = 0;
        while i < num {
            let sub_table: png_uint_16p =
                png_malloc(png_ptr, 256 * core::mem::size_of::<png_uint_16>()) as png_uint_16p;
            *table.offset(i as isize) = sub_table;

            if png_gamma_significant(gamma_val) != 0 {
                let mut j: c_uint = 0;
                while j < 256 {
                    let ig: png_uint_32 = (j << (8 - shift)) + i;
                    let d: f64 =
                        floor(65535.0 * pow(ig as f64 * fmax, gamma_val as f64 * 0.00001) + 0.5);
                    *sub_table.offset(j as isize) = d as png_uint_16;
                    j += 1;
                }
            } else {
                /* We must still build a table, but do it the fast way. */
                let mut j: c_uint = 0;

                while j < 256 {
                    let mut ig: png_uint_32 = (j << (8 - shift)) + i;

                    if shift != 0 {
                        ig = (ig * 65535u32 + max_by_2) / max;
                    }

                    *sub_table.offset(j as isize) = ig as png_uint_16;
                    j += 1;
                }
            }

            i += 1;
        }
    }
}

/* NOTE: this function expects the *inverse* of the overall gamma transformation. */
unsafe fn png_build_16to8_table(
    png_ptr: png_structrp,
    ptable: *mut png_uint_16pp,
    shift: c_uint,
    gamma_val: png_fixed_point,
) {
    unsafe {
        let num: c_uint = 1u32 << (8u32 - shift);
        let max: c_uint = (1u32 << (16u32 - shift)) - 1u32;
        let mut i: c_uint;
        let mut last: png_uint_32;

        let table: png_uint_16pp =
            png_calloc(png_ptr, num as usize * core::mem::size_of::<png_uint_16p>())
                as png_uint_16pp;
        *ptable = table;

        i = 0;
        while i < num {
            *table.offset(i as isize) =
                png_malloc(png_ptr, 256 * core::mem::size_of::<png_uint_16>()) as png_uint_16p;
            i += 1;
        }

        last = 0;
        i = 0;
        while i < 255 {
            /* 8-bit output value */
            /* Find the corresponding maximum input value */
            let out: png_uint_16 = (i * 257u32) as png_uint_16; /* 16-bit output value */

            /* Find the boundary value in 16 bits: */
            let mut bound: png_uint_32 =
                png_gamma_16bit_correct(out as c_uint + 128u32, gamma_val) as png_uint_32;

            /* Adjust (round) to (16-shift) bits: */
            bound = (bound * max + 32768u32) / 65535u32 + 1u32;

            while last < bound {
                *(*table.offset((last & (0xffu32 >> shift)) as isize))
                    .offset((last >> (8u32 - shift)) as isize) = out;
                last += 1;
            }

            i += 1;
        }

        /* And fill in the final entries. */
        while last < (num << 8) {
            *(*table.offset((last & (0xffu32 >> shift)) as isize))
                .offset((last >> (8u32 - shift)) as isize) = 65535u16;
            last += 1;
        }
    }
}

/* Build a single 8-bit table. */
unsafe fn png_build_8bit_table(
    png_ptr: png_structrp,
    ptable: png_bytepp,
    gamma_val: png_fixed_point,
) {
    unsafe {
        let mut i: c_uint;
        let table: png_bytep = png_malloc(png_ptr, 256) as png_bytep;
        *ptable = table;

        if png_gamma_significant(gamma_val) != 0 {
            i = 0;
            while i < 256 {
                *table.offset(i as isize) = png_gamma_8bit_correct(i, gamma_val);
                i += 1;
            }
        } else {
            i = 0;
            while i < 256 {
                *table.offset(i as isize) = (i & 0xff) as png_byte;
                i += 1;
            }
        }
    }
}

/* Used from png_read_destroy and below to release the memory used by the gamma
 * tables.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_destroy_gamma_table(png_ptr: png_structrp) {
    unsafe {
        png_free(png_ptr, (*png_ptr).gamma_table as png_voidp);
        (*png_ptr).gamma_table = core::ptr::null_mut();

        if !(*png_ptr).gamma_16_table.is_null() {
            let mut i: c_int;
            let istop: c_int = 1 << (8 - (*png_ptr).gamma_shift);
            i = 0;
            while i < istop {
                png_free(png_ptr, *(*png_ptr).gamma_16_table.offset(i as isize) as png_voidp);
                i += 1;
            }
            png_free(png_ptr, (*png_ptr).gamma_16_table as png_voidp);
            (*png_ptr).gamma_16_table = core::ptr::null_mut();
        }

        png_free(png_ptr, (*png_ptr).gamma_from_1 as png_voidp);
        (*png_ptr).gamma_from_1 = core::ptr::null_mut();
        png_free(png_ptr, (*png_ptr).gamma_to_1 as png_voidp);
        (*png_ptr).gamma_to_1 = core::ptr::null_mut();

        if !(*png_ptr).gamma_16_from_1.is_null() {
            let mut i: c_int;
            let istop: c_int = 1 << (8 - (*png_ptr).gamma_shift);
            i = 0;
            while i < istop {
                png_free(png_ptr, *(*png_ptr).gamma_16_from_1.offset(i as isize) as png_voidp);
                i += 1;
            }
            png_free(png_ptr, (*png_ptr).gamma_16_from_1 as png_voidp);
            (*png_ptr).gamma_16_from_1 = core::ptr::null_mut();
        }
        if !(*png_ptr).gamma_16_to_1.is_null() {
            let mut i: c_int;
            let istop: c_int = 1 << (8 - (*png_ptr).gamma_shift);
            i = 0;
            while i < istop {
                png_free(png_ptr, *(*png_ptr).gamma_16_to_1.offset(i as isize) as png_voidp);
                i += 1;
            }
            png_free(png_ptr, (*png_ptr).gamma_16_to_1 as png_voidp);
            (*png_ptr).gamma_16_to_1 = core::ptr::null_mut();
        }
    }
}

/* We build the 8- or 16-bit gamma tables here. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_build_gamma_table(png_ptr: png_structrp, bit_depth: c_int) {
    unsafe {
        let file_gamma: png_fixed_point;
        let screen_gamma: png_fixed_point;
        let correction: png_fixed_point;
        let file_to_linear: png_fixed_point;
        let linear_to_screen: png_fixed_point;

        /* Remove any existing table. */
        if !(*png_ptr).gamma_table.is_null() || !(*png_ptr).gamma_16_table.is_null() {
            png_warning(png_ptr, c"gamma table being rebuilt".as_ptr());
            png_destroy_gamma_table(png_ptr);
        }

        file_gamma = (*png_ptr).file_gamma;
        screen_gamma = (*png_ptr).screen_gamma;
        file_to_linear = png_reciprocal(file_gamma);

        if screen_gamma > 0 {
            linear_to_screen = png_reciprocal(screen_gamma);
            correction = png_reciprocal2(screen_gamma, file_gamma);
        } else
        /* screen gamma unknown */
        {
            linear_to_screen = file_gamma;
            correction = PNG_FP_1;
        }

        if bit_depth <= 8 {
            png_build_8bit_table(png_ptr, &raw mut (*png_ptr).gamma_table, correction);

            if ((*png_ptr).transformations & (PNG_COMPOSE | PNG_RGB_TO_GRAY)) != 0 {
                png_build_8bit_table(png_ptr, &raw mut (*png_ptr).gamma_to_1, file_to_linear);

                png_build_8bit_table(png_ptr, &raw mut (*png_ptr).gamma_from_1, linear_to_screen);
            }
        } else {
            let mut shift: png_byte;
            let sig_bit: png_byte;

            if ((*png_ptr).color_type & PNG_COLOR_MASK_COLOR as png_byte) != 0 {
                let mut sb: png_byte = (*png_ptr).sig_bit.red;

                if (*png_ptr).sig_bit.green > sb {
                    sb = (*png_ptr).sig_bit.green;
                }

                if (*png_ptr).sig_bit.blue > sb {
                    sb = (*png_ptr).sig_bit.blue;
                }
                sig_bit = sb;
            } else {
                sig_bit = (*png_ptr).sig_bit.gray;
            }

            if sig_bit > 0 && (sig_bit as c_uint) < 16u32 {
                /* shift == insignificant bits */
                shift = ((16u32 - sig_bit as c_uint) & 0xff) as png_byte;
            } else {
                shift = 0; /* keep all 16 bits */
            }

            if ((*png_ptr).transformations & (PNG_16_TO_8 | PNG_SCALE_16_TO_8)) != 0 {
                if (shift as c_uint) < (16u32 - PNG_MAX_GAMMA_8 as c_uint) {
                    shift = (16 - PNG_MAX_GAMMA_8) as png_byte;
                }
            }

            if shift > 8 {
                shift = 8; /* Guarantees at least one table! */
            }

            (*png_ptr).gamma_shift = shift as c_int;

            if ((*png_ptr).transformations & (PNG_16_TO_8 | PNG_SCALE_16_TO_8)) != 0 {
                png_build_16to8_table(
                    png_ptr,
                    &raw mut (*png_ptr).gamma_16_table,
                    shift as c_uint,
                    png_reciprocal(correction),
                );
            } else {
                png_build_16bit_table(
                    png_ptr,
                    &raw mut (*png_ptr).gamma_16_table,
                    shift as c_uint,
                    correction,
                );
            }

            if ((*png_ptr).transformations & (PNG_COMPOSE | PNG_RGB_TO_GRAY)) != 0 {
                png_build_16bit_table(
                    png_ptr,
                    &raw mut (*png_ptr).gamma_16_to_1,
                    shift as c_uint,
                    file_to_linear,
                );

                png_build_16bit_table(
                    png_ptr,
                    &raw mut (*png_ptr).gamma_16_from_1,
                    shift as c_uint,
                    linear_to_screen,
                );
            }
        }
    }
}

/* HARDWARE OR SOFTWARE OPTION SUPPORT */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_option(
    png_ptr: png_structrp,
    option: c_int,
    onoff: c_int,
) -> c_int {
    unsafe {
        if !png_ptr.is_null() && option >= 0 && option < PNG_OPTION_NEXT && (option & 1) == 0 {
            let mask: png_uint_32 = 3u32 << option;
            let setting: png_uint_32 = (2u32 + (onoff != 0) as u32) << option;
            let current: png_uint_32 = (*png_ptr).options;

            (*png_ptr).options = (current & !mask) | setting;

            return ((current & mask) as c_int) >> option;
        }

        PNG_OPTION_INVALID
    }
}

/* The convert-to-sRGB table is only currently required for read. */
#[unsafe(no_mangle)]
pub static png_sRGB_table: [png_uint_16; 256] = [
    0, 20, 40, 60, 80, 99, 119, 139, 159, 179, 199, 219, 241, 264, 288, 313, 340, 367, 396, 427,
    458, 491, 526, 562, 599, 637, 677, 718, 761, 805, 851, 898, 947, 997, 1048, 1101, 1156, 1212,
    1270, 1330, 1391, 1453, 1517, 1583, 1651, 1720, 1790, 1863, 1937, 2013, 2090, 2170, 2250, 2333,
    2418, 2504, 2592, 2681, 2773, 2866, 2961, 3058, 3157, 3258, 3360, 3464, 3570, 3678, 3788, 3900,
    4014, 4129, 4247, 4366, 4488, 4611, 4736, 4864, 4993, 5124, 5257, 5392, 5530, 5669, 5810, 5953,
    6099, 6246, 6395, 6547, 6700, 6856, 7014, 7174, 7335, 7500, 7666, 7834, 8004, 8177, 8352, 8528,
    8708, 8889, 9072, 9258, 9445, 9635, 9828, 10022, 10219, 10417, 10619, 10822, 11028, 11235,
    11446, 11658, 11873, 12090, 12309, 12530, 12754, 12980, 13209, 13440, 13673, 13909, 14146,
    14387, 14629, 14874, 15122, 15371, 15623, 15878, 16135, 16394, 16656, 16920, 17187, 17456,
    17727, 18001, 18277, 18556, 18837, 19121, 19407, 19696, 19987, 20281, 20577, 20876, 21177,
    21481, 21787, 22096, 22407, 22721, 23038, 23357, 23678, 24002, 24329, 24658, 24990, 25325,
    25662, 26001, 26344, 26688, 27036, 27386, 27739, 28094, 28452, 28813, 29176, 29542, 29911,
    30282, 30656, 31033, 31412, 31794, 32179, 32567, 32957, 33350, 33745, 34143, 34544, 34948,
    35355, 35764, 36176, 36591, 37008, 37429, 37852, 38278, 38706, 39138, 39572, 40009, 40449,
    40891, 41337, 41785, 42236, 42690, 43147, 43606, 44069, 44534, 45002, 45473, 45947, 46423,
    46903, 47385, 47871, 48359, 48850, 49344, 49841, 50341, 50844, 51349, 51858, 52369, 52884,
    53401, 53921, 54445, 54971, 55500, 56032, 56567, 57105, 57646, 58190, 58737, 59287, 59840,
    60396, 60955, 61517, 62082, 62650, 63221, 63795, 64372, 64952, 65535,
];

/* The base/delta tables are required for both read and write. */
#[unsafe(no_mangle)]
pub static png_sRGB_base: [png_uint_16; 512] = [
    128, 1782, 3383, 4644, 5675, 6564, 7357, 8074, 8732, 9346, 9921, 10463, 10977, 11466, 11935,
    12384, 12816, 13233, 13634, 14024, 14402, 14769, 15125, 15473, 15812, 16142, 16466, 16781,
    17090, 17393, 17690, 17981, 18266, 18546, 18822, 19093, 19359, 19621, 19879, 20133, 20383,
    20630, 20873, 21113, 21349, 21583, 21813, 22041, 22265, 22487, 22707, 22923, 23138, 23350,
    23559, 23767, 23972, 24175, 24376, 24575, 24772, 24967, 25160, 25352, 25542, 25730, 25916,
    26101, 26284, 26465, 26645, 26823, 27000, 27176, 27350, 27523, 27695, 27865, 28034, 28201,
    28368, 28533, 28697, 28860, 29021, 29182, 29341, 29500, 29657, 29813, 29969, 30123, 30276,
    30429, 30580, 30730, 30880, 31028, 31176, 31323, 31469, 31614, 31758, 31902, 32045, 32186,
    32327, 32468, 32607, 32746, 32884, 33021, 33158, 33294, 33429, 33564, 33697, 33831, 33963,
    34095, 34226, 34357, 34486, 34616, 34744, 34873, 35000, 35127, 35253, 35379, 35504, 35629,
    35753, 35876, 35999, 36122, 36244, 36365, 36486, 36606, 36726, 36845, 36964, 37083, 37201,
    37318, 37435, 37551, 37668, 37783, 37898, 38013, 38127, 38241, 38354, 38467, 38580, 38692,
    38803, 38915, 39026, 39136, 39246, 39356, 39465, 39574, 39682, 39790, 39898, 40005, 40112,
    40219, 40325, 40431, 40537, 40642, 40747, 40851, 40955, 41059, 41163, 41266, 41369, 41471,
    41573, 41675, 41777, 41878, 41979, 42079, 42179, 42279, 42379, 42478, 42577, 42676, 42775,
    42873, 42971, 43068, 43165, 43262, 43359, 43456, 43552, 43648, 43743, 43839, 43934, 44028,
    44123, 44217, 44311, 44405, 44499, 44592, 44685, 44778, 44870, 44962, 45054, 45146, 45238,
    45329, 45420, 45511, 45601, 45692, 45782, 45872, 45961, 46051, 46140, 46229, 46318, 46406,
    46494, 46583, 46670, 46758, 46846, 46933, 47020, 47107, 47193, 47280, 47366, 47452, 47538,
    47623, 47709, 47794, 47879, 47964, 48048, 48133, 48217, 48301, 48385, 48468, 48552, 48635,
    48718, 48801, 48884, 48966, 49048, 49131, 49213, 49294, 49376, 49458, 49539, 49620, 49701,
    49782, 49862, 49943, 50023, 50103, 50183, 50263, 50342, 50422, 50501, 50580, 50659, 50738,
    50816, 50895, 50973, 51051, 51129, 51207, 51285, 51362, 51439, 51517, 51594, 51671, 51747,
    51824, 51900, 51977, 52053, 52129, 52205, 52280, 52356, 52432, 52507, 52582, 52657, 52732,
    52807, 52881, 52956, 53030, 53104, 53178, 53252, 53326, 53400, 53473, 53546, 53620, 53693,
    53766, 53839, 53911, 53984, 54056, 54129, 54201, 54273, 54345, 54417, 54489, 54560, 54632,
    54703, 54774, 54845, 54916, 54987, 55058, 55129, 55199, 55269, 55340, 55410, 55480, 55550,
    55620, 55689, 55759, 55828, 55898, 55967, 56036, 56105, 56174, 56243, 56311, 56380, 56448,
    56517, 56585, 56653, 56721, 56789, 56857, 56924, 56992, 57059, 57127, 57194, 57261, 57328,
    57395, 57462, 57529, 57595, 57662, 57728, 57795, 57861, 57927, 57993, 58059, 58125, 58191,
    58256, 58322, 58387, 58453, 58518, 58583, 58648, 58713, 58778, 58843, 58908, 58972, 59037,
    59101, 59165, 59230, 59294, 59358, 59422, 59486, 59549, 59613, 59677, 59740, 59804, 59867,
    59930, 59993, 60056, 60119, 60182, 60245, 60308, 60370, 60433, 60495, 60558, 60620, 60682,
    60744, 60806, 60868, 60930, 60992, 61054, 61115, 61177, 61238, 61300, 61361, 61422, 61483,
    61544, 61605, 61666, 61727, 61788, 61848, 61909, 61969, 62030, 62090, 62150, 62211, 62271,
    62331, 62391, 62450, 62510, 62570, 62630, 62689, 62749, 62808, 62867, 62927, 62986, 63045,
    63104, 63163, 63222, 63281, 63340, 63398, 63457, 63515, 63574, 63632, 63691, 63749, 63807,
    63865, 63923, 63981, 64039, 64097, 64155, 64212, 64270, 64328, 64385, 64443, 64500, 64557,
    64614, 64672, 64729, 64786, 64843, 64900, 64956, 65013, 65070, 65126, 65183, 65239, 65296,
    65352, 65409, 65465,
];

#[unsafe(no_mangle)]
pub static png_sRGB_delta: [png_byte; 512] = [
    207, 201, 158, 129, 113, 100, 90, 82, 77, 72, 68, 64, 61, 59, 56, 54,
    52, 50, 49, 47, 46, 45, 43, 42, 41, 40, 39, 39, 38, 37, 36, 36,
    35, 34, 34, 33, 33, 32, 32, 31, 31, 30, 30, 30, 29, 29, 28, 28,
    28, 27, 27, 27, 27, 26, 26, 26, 25, 25, 25, 25, 24, 24, 24, 24,
    23, 23, 23, 23, 23, 22, 22, 22, 22, 22, 22, 21, 21, 21, 21, 21,
    21, 20, 20, 20, 20, 20, 20, 20, 20, 19, 19, 19, 19, 19, 19, 19,
    19, 18, 18, 18, 18, 18, 18, 18, 18, 18, 18, 17, 17, 17, 17, 17,
    17, 17, 17, 17, 17, 17, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
    16, 16, 16, 16, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14, 14,
    14, 14, 14, 14, 14, 14, 14, 13, 13, 13, 13, 13, 13, 13, 13, 13,
    13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 12, 12,
    12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12,
    12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 11, 11, 11, 11,
    11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
    11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11,
    11, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
    10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
    10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
    10, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
];

/* SIMPLIFIED READ/WRITE SUPPORT */
unsafe fn png_image_free_function(argument: png_voidp) -> c_int {
    unsafe {
        let image: png_imagep = argument as png_imagep;
        let cp: png_controlp = (*image).opaque;
        let c: png_control;

        /* Double check that we have a png_ptr. */
        if (*cp).png_ptr.is_null() {
            return 0;
        }

        /* First free any data held in the control structure. */
        if (*cp).owned_file() != 0 {
            let fp: *mut FILE = (*(*cp).png_ptr).io_ptr as *mut FILE;
            (*cp).set_owned_file(0);

            /* Ignore errors here. */
            if !fp.is_null() {
                (*(*cp).png_ptr).io_ptr = core::ptr::null_mut();
                fclose(fp);
            }
        }

        /* Copy the control structure so that the original, allocated, version can be
         * safely freed.
         */
        c = core::ptr::read(cp);
        let c_ptr: *const png_control = &raw const c;
        (*image).opaque = c_ptr as png_controlp;
        png_free(c.png_ptr, cp as png_voidp);

        /* Then the structures, calling the correct API. */
        if c.for_write() != 0 {
            let mut png_ptr = c.png_ptr;
            let mut info_ptr = c.info_ptr;
            png_destroy_write_struct(&raw mut png_ptr, &raw mut info_ptr);
        } else {
            let mut png_ptr = c.png_ptr;
            let mut info_ptr = c.info_ptr;
            png_destroy_read_struct(&raw mut png_ptr, &raw mut info_ptr, core::ptr::null_mut());
        }

        /* Success. */
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_image_free(image: png_imagep) {
    unsafe {
        /* Safely call the real function, but only if doing so is safe at this point. */
        if !image.is_null()
            && !(*image).opaque.is_null()
            && (*(*image).opaque).error_buf.is_null()
        {
            png_image_free_function(image as png_voidp);
            (*image).opaque = core::ptr::null_mut();
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_image_error(
    image: png_imagep,
    error_message: png_const_charp,
) -> c_int {
    unsafe {
        /* Utility to log an error. */
        png_safecat(
            (*image).message.as_mut_ptr(),
            core::mem::size_of_val(&(*image).message),
            0,
            error_message,
        );
        (*image).warning_or_error |= PNG_IMAGE_ERROR;
        png_image_free(image);
        0
    }
}
