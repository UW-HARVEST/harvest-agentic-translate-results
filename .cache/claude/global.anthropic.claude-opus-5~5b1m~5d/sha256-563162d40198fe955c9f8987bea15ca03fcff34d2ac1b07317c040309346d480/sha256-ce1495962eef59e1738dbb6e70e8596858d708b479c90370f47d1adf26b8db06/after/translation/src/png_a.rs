//! png.c lines 1-1066: general purpose libpng functions (signature handling,
//! zlib allocators, CRC, png_struct/png_info creation and destruction, RFC1123
//! time conversion, version strings and zlib error messages).
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

/* Tells libpng that we have already handled the first "num_bytes" bytes
 * of the PNG file signature.  If the PNG data is embedded into another
 * stream we can set num_bytes = 8 so that libpng will not attempt to read
 * or write any of the magic bytes before it starts on the IHDR.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_sig_bytes(png_ptr: png_structrp, num_bytes: c_int) {
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

/* Checks whether the supplied bytes match the PNG signature.  We allow
 * checking less than the full 8-byte signature so that those apps that
 * already read the first few bytes of a file to determine the file type
 * can simply check the remaining bytes for extra assurance.  Returns
 * an integer less than, equal to, or greater than zero if sig is found,
 * respectively, to be less than, to match, or be greater than the correct
 * PNG signature (this is the same behavior as strcmp, memcmp, etc).
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_sig_cmp(
    sig: png_const_bytep,
    start: usize,
    num_to_check_in: usize,
) -> c_int {
    static png_signature: [png_byte; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    let mut num_to_check = num_to_check_in;

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
        sig.add(start) as *const u8,
        png_signature.as_ptr().add(start) as *const u8,
        num_to_check,
    )
}

/* Function to allocate memory for zlib */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_zalloc(
    png_ptr: crate::zlib::voidpf,
    items: uInt,
    size: uInt,
) -> crate::zlib::voidpf {
    let mut num_bytes: png_alloc_size_t = size as png_alloc_size_t;

    if png_ptr.is_null() {
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
            >= (!(0 as png_alloc_size_t)) / (size as png_alloc_size_t)
    {
        png_warning(
            png_ptr as png_const_structrp,
            c"Potential overflow in png_zalloc()".as_ptr(),
        );
        return core::ptr::null_mut();
    }

    num_bytes = num_bytes.wrapping_mul(items as png_alloc_size_t);
    png_malloc_warn(png_ptr as png_const_structrp, num_bytes)
}

/* Function to free memory for zlib */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_zfree(png_ptr: crate::zlib::voidpf, ptr: crate::zlib::voidpf) {
    png_free(png_ptr as png_const_structrp, ptr);
}

/* Reset the CRC variable to 32 bits of 1's.  Care must be taken
 * in case CRC is > 32 bits to leave the top bits 0.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_reset_crc(png_ptr: png_structrp) {
    /* The cast is safe because the crc is a 32-bit value. */
    (*png_ptr).crc = crc32(0, core::ptr::null(), 0) as png_uint_32;
}

/* Calculate the CRC over a section of data.  We can only pass as
 * much data to this routine as the largest single buffer size.  We
 * also check that this data will actually be used before going to the
 * trouble of calculating it.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_calculate_crc(
    png_ptr: png_structrp,
    ptr_in: png_const_bytep,
    length_in: usize,
) {
    let mut ptr = ptr_in;
    let mut length = length_in;
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

            crc = crc32(crc, ptr, safe_length);

            /* The following should never issue compiler warnings; if they do the
             * target system has characteristics that will probably violate other
             * assumptions within the libpng code.
             */
            ptr = ptr.add(safe_length as usize);
            length = length.wrapping_sub(safe_length as usize);

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
pub unsafe extern "C-unwind" fn png_user_version_check(
    png_ptr: png_structrp,
    user_png_ver: png_const_charp,
) -> c_int {
    /* Libpng versions 1.0.0 and later are binary compatible if the version
     * string matches through the second '.'; we must recompile any
     * applications that use any older library version.
     */

    if !user_png_ver.is_null() {
        let mut i: c_int = -1;
        let mut found_dots: c_int = 0;

        loop {
            i += 1;
            if *user_png_ver.add(i as usize)
                != *PNG_LIBPNG_VER_STRING.as_ptr().add(i as usize) as c_char
            {
                (*png_ptr).flags |= PNG_FLAG_LIBRARY_MISMATCH;
            }
            if *user_png_ver.add(i as usize) == b'.' as c_char {
                found_dots += 1;
            }

            if !(found_dots < 2
                && *user_png_ver.add(i as usize) != 0
                && *PNG_LIBPNG_VER_STRING.as_ptr().add(i as usize) != 0)
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
            c"Application built with libpng-".as_ptr(),
        );
        pos = png_safecat(m.as_mut_ptr(), 128, pos, user_png_ver);
        pos = png_safecat(m.as_mut_ptr(), 128, pos, c" but running with ".as_ptr());
        pos = png_safecat(
            m.as_mut_ptr(),
            128,
            pos,
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        );

        png_warning(png_ptr, m.as_ptr());

        return 0;
    }

    /* Success return. */
    1
}

/* Generic function to create a png_struct for either read or write - this
 * contains the common initialization.
 */
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
    let mut create_struct: png_struct;
    let mut create_jmp_buf: jmp_buf = jmp_buf([0; 25]);

    /* This temporary stack-allocated structure is used to provide a place to
     * build enough context to allow the user provided memory allocator (if any)
     * to be called.
     */
    create_struct = png_struct::default();

    let cs: *mut png_struct = &mut create_struct;
    let cjb: *mut jmp_buf = &mut create_jmp_buf;

    (*cs).user_width_max = PNG_USER_WIDTH_MAX;
    (*cs).user_height_max = PNG_USER_HEIGHT_MAX;

    (*cs).user_chunk_cache_max = PNG_USER_CHUNK_CACHE_MAX;

    /* PNG_USER_CHUNK_MALLOC_MAX > 0: default to the compile-time limit */
    (*cs).user_chunk_malloc_max = PNG_USER_CHUNK_MALLOC_MAX;

    /* The following two API calls simply set fields in png_struct, so it is safe
     * to do them now even though error handling is not yet set up.
     */
    png_set_mem_fn(cs, mem_ptr, malloc_fn, free_fn);

    /* (*error_fn) can return control to the caller after the error_ptr is set,
     * this will result in a memory leak unless the error_fn does something
     * extremely sophisticated.  The design lacks merit but is implicit in the
     * API.
     */
    png_set_error_fn(cs, error_ptr, error_fn, warn_fn);

    /* if (!setjmp(create_jmp_buf)) - the Rust translation uses an unwind in
     * place of setjmp/longjmp.
     */
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        /* Temporarily fake out the longjmp information until we have
         * successfully completed this function.  This only works if we have
         * setjmp() support compiled in, but it is safe - this stuff should
         * never happen.
         */
        (*cs).jmp_buf_ptr = cjb;
        (*cs).jmp_buf_size = 0; /*stack allocation*/
        (*cs).longjmp_fn = Some(png_internal_longjmp);

        /* Call the general version checker (shared with read and write code): */
        if png_user_version_check(cs, user_png_ver) != 0 {
            let png_ptr: png_structrp =
                png_malloc_warn(cs, core::mem::size_of::<png_struct>()) as png_structrp;

            if !png_ptr.is_null() {
                /* png_ptr->zstream holds a back-pointer to the png_struct, so
                 * this can only be done now:
                 */
                (*cs).zstream.zalloc = Some(png_zalloc);
                (*cs).zstream.zfree = Some(png_zfree);
                (*cs).zstream.opaque = png_ptr as crate::zlib::voidpf;

                /* Eliminate the local error handling: */
                (*cs).jmp_buf_ptr = core::ptr::null_mut();
                (*cs).jmp_buf_size = 0;
                (*cs).longjmp_fn = None;

                core::ptr::copy_nonoverlapping(cs as *const png_struct, png_ptr, 1);

                /* This is the successful return point */
                return png_ptr;
            }
        }

        core::ptr::null_mut()
    }));

    /* A longjmp because of a bug in the application storage allocator or a
     * simple failure to allocate the png_struct.
     */
    match caught {
        Ok(png_ptr) => png_ptr,
        Err(_) => core::ptr::null_mut(),
    }
}

/* Allocate the memory for an info_struct for the application. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_create_info_struct(png_ptr: png_const_structrp) -> png_infop {
    let info_ptr: png_inforp;

    if png_ptr.is_null() {
        return core::ptr::null_mut();
    }

    /* Use the internal API that does not (or at least should not) error out, so
     * that this call always returns ok.  The application typically sets up the
     * error handling *after* creating the info_struct because this is the way it
     * has always been done in 'example.c'.
     */
    info_ptr = png_malloc_base(png_ptr, core::mem::size_of::<png_info>()) as png_inforp;

    if !info_ptr.is_null() {
        memset(info_ptr as *mut u8, 0, core::mem::size_of::<png_info>());
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
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_destroy_info_struct(
    png_ptr: png_const_structrp,
    info_ptr_ptr: png_infopp,
) {
    let mut info_ptr: png_inforp = core::ptr::null_mut();

    if png_ptr.is_null() {
        return;
    }

    if !info_ptr_ptr.is_null() {
        info_ptr = *info_ptr_ptr;
    }

    if !info_ptr.is_null() {
        /* Do this first in case of an error below; if the app implements its own
         * memory management this can lead to png_free calling png_error, which
         * will abort this routine and return control to the app error handler.
         * An infinite loop may result if it then tries to free the same info
         * ptr.
         */
        *info_ptr_ptr = core::ptr::null_mut();

        png_free_data(png_ptr, info_ptr, PNG_FREE_ALL, -1);
        memset(info_ptr as *mut u8, 0, core::mem::size_of::<png_info>());
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
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_info_init_3(
    ptr_ptr: png_infopp,
    png_info_struct_size: usize,
) {
    let mut info_ptr: png_inforp = *ptr_ptr;

    if info_ptr.is_null() {
        return;
    }

    if core::mem::size_of::<png_info>() > png_info_struct_size {
        *ptr_ptr = core::ptr::null_mut();
        /* The following line is why this API should not be used: */
        crate::cabi::free(info_ptr as *mut c_void);
        info_ptr =
            png_malloc_base(core::ptr::null(), core::mem::size_of::<png_info>()) as png_inforp;
        if info_ptr.is_null() {
            return;
        }
        *ptr_ptr = info_ptr;
    }

    /* Set everything to 0 */
    memset(info_ptr as *mut u8, 0, core::mem::size_of::<png_info>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_data_freer(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    freer: c_int,
    mask: png_uint_32,
) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_free_data(
    png_ptr: png_const_structrp,
    info_ptr: png_inforp,
    mask_in: png_uint_32,
    num: c_int,
) {
    let mut mask = mask_in;

    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }

    /* Free text item num or (if num == -1) all text items */
    if !(*info_ptr).text.is_null() && ((mask & PNG_FREE_TEXT) & (*info_ptr).free_me) != 0 {
        if num != -1 {
            png_free(png_ptr, (*(*info_ptr).text.add(num as usize)).key as png_voidp);
            (*(*info_ptr).text.add(num as usize)).key = core::ptr::null_mut();
        } else {
            let mut i: c_int;

            i = 0;
            while i < (*info_ptr).num_text {
                png_free(png_ptr, (*(*info_ptr).text.add(i as usize)).key as png_voidp);
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
            let mut i: c_int;

            i = 0;
            while i < (*info_ptr).pcal_nparams as c_int {
                png_free(png_ptr, *(*info_ptr).pcal_params.add(i as usize) as png_voidp);
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
                (*(*info_ptr).splt_palettes.add(num as usize)).name as png_voidp,
            );
            png_free(
                png_ptr,
                (*(*info_ptr).splt_palettes.add(num as usize)).entries as png_voidp,
            );
            (*(*info_ptr).splt_palettes.add(num as usize)).name = core::ptr::null_mut();
            (*(*info_ptr).splt_palettes.add(num as usize)).entries = core::ptr::null_mut();
        } else {
            let mut i: c_int;

            i = 0;
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

    if !(*info_ptr).unknown_chunks.is_null()
        && ((mask & PNG_FREE_UNKN) & (*info_ptr).free_me) != 0
    {
        if num != -1 {
            png_free(
                png_ptr,
                (*(*info_ptr).unknown_chunks.add(num as usize)).data as png_voidp,
            );
            (*(*info_ptr).unknown_chunks.add(num as usize)).data = core::ptr::null_mut();
        } else {
            let mut i: c_int;

            i = 0;
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
            let mut row: png_uint_32;
            row = 0;
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

/* This function returns a pointer to the io_ptr associated with the user
 * functions.  The application should free any memory associated with this
 * pointer before png_write_destroy() or png_read_destroy() are called.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_io_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr.is_null() {
        return core::ptr::null_mut();
    }

    (*png_ptr).io_ptr
}

/* Initialize the default input/output functions for the PNG file.  If you
 * use your own read or write routines, you can call either png_set_read_fn()
 * or png_set_write_fn() instead of png_init_io().  If you have defined
 * PNG_NO_STDIO or otherwise disabled PNG_STDIO_SUPPORTED, you must use a
 * function of your own because "FILE *" isn't necessarily available.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_init_io(png_ptr: png_structrp, fp: *mut c_void) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).io_ptr = fp as png_voidp;
}

/* PNG signed integers are saved in 32-bit 2's complement format.  ANSI C-90
 * defines a cast of a signed integer to an unsigned integer either to preserve
 * the value, if it is positive, or to calculate:
 *
 *     (UNSIGNED_MAX+1) + integer
 *
 * Where UNSIGNED_MAX is the appropriate maximum unsigned value, so when the
 * negative integral value is added the result will be an unsigned value
 * corresponding to the 2's complement representation.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_save_int_32(buf: png_bytep, i: png_int_32) {
    png_save_uint_32(buf, i as png_uint_32);
}

/* Convert the supplied time into an RFC 1123 string suitable for use in
 * a "Creation Time" or other text-based time string.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_convert_to_rfc1123_buffer(
    out: *mut c_char,
    ptime: png_const_timep,
) -> c_int {
    static short_months: [[u8; 4]; 12] = [
        *b"Jan\0", *b"Feb\0", *b"Mar\0", *b"Apr\0", *b"May\0", *b"Jun\0", *b"Jul\0", *b"Aug\0",
        *b"Sep\0", *b"Oct\0", *b"Nov\0", *b"Dec\0",
    ];

    if out.is_null() {
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

        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_u, (unsigned)ptime->day); */
        pos = png_safecat(
            out,
            29,
            pos,
            png_format_number(
                number_buf.as_ptr(),
                number_buf.as_mut_ptr().add(5),
                PNG_NUMBER_FORMAT_u,
                (*ptime).day as c_uint as png_alloc_size_t,
            ),
        );
        /* APPEND(' '); */
        if pos < 28 {
            *out.add(pos) = b' ' as c_char;
            pos += 1;
        }
        /* APPEND_STRING(short_months[(ptime->month - 1)]); */
        pos = png_safecat(
            out,
            29,
            pos,
            short_months[((*ptime).month as c_int - 1) as usize].as_ptr() as *const c_char,
        );
        /* APPEND(' '); */
        if pos < 28 {
            *out.add(pos) = b' ' as c_char;
            pos += 1;
        }
        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_u, ptime->year); */
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
        /* APPEND(' '); */
        if pos < 28 {
            *out.add(pos) = b' ' as c_char;
            pos += 1;
        }
        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_02u, (unsigned)ptime->hour); */
        pos = png_safecat(
            out,
            29,
            pos,
            png_format_number(
                number_buf.as_ptr(),
                number_buf.as_mut_ptr().add(5),
                PNG_NUMBER_FORMAT_02u,
                (*ptime).hour as c_uint as png_alloc_size_t,
            ),
        );
        /* APPEND(':'); */
        if pos < 28 {
            *out.add(pos) = b':' as c_char;
            pos += 1;
        }
        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_02u, (unsigned)ptime->minute); */
        pos = png_safecat(
            out,
            29,
            pos,
            png_format_number(
                number_buf.as_ptr(),
                number_buf.as_mut_ptr().add(5),
                PNG_NUMBER_FORMAT_02u,
                (*ptime).minute as c_uint as png_alloc_size_t,
            ),
        );
        /* APPEND(':'); */
        if pos < 28 {
            *out.add(pos) = b':' as c_char;
            pos += 1;
        }
        /* APPEND_NUMBER(PNG_NUMBER_FORMAT_02u, (unsigned)ptime->second); */
        pos = png_safecat(
            out,
            29,
            pos,
            png_format_number(
                number_buf.as_ptr(),
                number_buf.as_mut_ptr().add(5),
                PNG_NUMBER_FORMAT_02u,
                (*ptime).second as c_uint as png_alloc_size_t,
            ),
        );
        /* APPEND_STRING(" +0000"); - This reliably terminates the buffer */
        pos = png_safecat(out, 29, pos, c" +0000".as_ptr());
    }

    1
}

/* To do: remove the following from libpng-1.7 */
/* Original API that uses a private buffer in png_struct.
 * Deprecated because it causes png_struct to carry a spurious temporary
 * buffer (png_struct::time_buffer), better to have the caller pass this in.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_convert_to_rfc1123(
    png_ptr: png_structrp,
    ptime: png_const_timep,
) -> png_const_charp {
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_copyright(png_ptr: png_const_structrp) -> png_const_charp {
    c"\nlibpng version 1.6.59.git\nCopyright (c) 2018-2026 Cosmin Truta\nCopyright (c) 1998-2002,2004,2006-2018 Glenn Randers-Pehrson\nCopyright (c) 1996-1997 Andreas Dilger\nCopyright (c) 1995-1996 Guy Eric Schalnat, Group 42, Inc.\n".as_ptr()
}

/* The following return the library version as a short string in the
 * format 1.0.0 through 99.99.99zz.  To get the version of *.h files
 * used with your application, print out PNG_LIBPNG_VER_STRING, which
 * is defined in png.h.
 * Note: now there is no difference between png_get_libpng_ver() and
 * png_get_header_ver().  Due to the version_nn_nn_nn typedef guard,
 * it is guaranteed that png.c uses the correct version of png.h.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_libpng_ver(
    png_ptr: png_const_structrp,
) -> png_const_charp {
    /* Version of *.c files used when building libpng */
    png_get_header_ver(png_ptr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_header_ver(png_ptr: png_const_structrp) -> png_const_charp {
    /* Version of *.h files used when building libpng */
    PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_header_version(
    png_ptr: png_const_structrp,
) -> png_const_charp {
    /* Returns longer string containing both version and date */
    c" libpng version 1.6.59.git\n\n".as_ptr()
}

/* NOTE: this routine is not used internally! */
/* Build a grayscale palette.  Palette is assumed to be 1 << bit_depth
 * large of png_color.  This lets grayscale images be treated as
 * paletted.  Most useful for gamma correction and simplification
 * of code.  This API is not used internally.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_build_grayscale_palette(
    bit_depth: c_int,
    palette: png_colorp,
) {
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
        (*palette.add(i as usize)).red = (v & 0xff) as png_byte;
        (*palette.add(i as usize)).green = (v & 0xff) as png_byte;
        (*palette.add(i as usize)).blue = (v & 0xff) as png_byte;
        i += 1;
        v = v.wrapping_add(color_inc);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_handle_as_unknown(
    png_ptr: png_const_structrp,
    chunk_name: png_const_bytep,
) -> c_int {
    /* Check chunk_name and return "keep" value if it's on the list, else 0 */
    let mut p: png_const_bytep;
    let p_end: png_const_bytep;

    if png_ptr.is_null() || chunk_name.is_null() || (*png_ptr).num_chunk_list == 0 {
        return PNG_HANDLE_CHUNK_AS_DEFAULT;
    }

    p_end = (*png_ptr).chunk_list;
    p = p_end.add(((*png_ptr).num_chunk_list * 5) as usize); /* beyond end */

    /* The code is the fifth byte after each four byte string.  Historically this
     * code was always searched from the end of the list, this is no longer
     * necessary because the 'set' routine handles duplicate entries correctly.
     */
    loop
    /* num_chunk_list > 0, so at least one */
    {
        p = p.sub(5);

        if memcmp(chunk_name as *const u8, p as *const u8, 4) == 0 {
            return *p.add(4) as c_int;
        }

        if !(p > p_end) {
            break;
        }
    }

    /* This means that known chunks should be processed and unknown chunks should
     * be handled according to the value of png_ptr->unknown_default; this can be
     * confusing because, as a result, there are two levels of defaulting for
     * unknown chunks.
     */
    PNG_HANDLE_CHUNK_AS_DEFAULT
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_chunk_unknown_handling(
    png_ptr: png_const_structrp,
    chunk_name: png_uint_32,
) -> c_int {
    let mut chunk_string: [png_byte; 5] = [0; 5];

    PNG_CSTRING_FROM_CHUNK(chunk_string.as_mut_ptr(), chunk_name);
    png_handle_as_unknown(png_ptr, chunk_string.as_ptr())
}

/* This function, added to libpng-1.0.6g, is untested. */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_reset_zstream(png_ptr: png_structrp) -> c_int {
    if png_ptr.is_null() {
        return Z_STREAM_ERROR;
    }

    /* WARNING: this resets the window bits to the maximum! */
    inflateReset(&mut (*png_ptr).zstream)
}

/* This function was added to libpng-1.0.7 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_access_version_number() -> png_uint_32 {
    /* Version of *.c files used when building libpng */
    PNG_LIBPNG_VER as png_uint_32
}

/* Ensure that png_ptr->zstream.msg holds some appropriate error message string.
 * If it doesn't 'ret' is used to set it to something appropriate, even in cases
 * like Z_OK or Z_STREAM_END where the error code is apparently a success code.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_zstream_error(png_ptr: png_structrp, ret: c_int) {
    /* Translate 'ret' into an appropriate error string, priority is given to the
     * one in zstream if set.  This always returns a string, even in cases like
     * Z_OK or Z_STREAM_END where the error code is a success code.
     */
    if (*png_ptr).zstream.msg.is_null() {
        match ret {
            Z_STREAM_END => {
                /* Normal exit */
                (*png_ptr).zstream.msg = c"unexpected end of LZ stream".as_ptr();
            }

            Z_NEED_DICT => {
                /* This means the deflate stream did not have a dictionary; this
                 * indicates a bogus PNG.
                 */
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
                /* End of input or output; not a problem if the caller is doing
                 * incremental read or write.
                 */
                (*png_ptr).zstream.msg = c"truncated".as_ptr();
            }

            Z_VERSION_ERROR => {
                (*png_ptr).zstream.msg = c"unsupported zlib version".as_ptr();
            }

            PNG_UNEXPECTED_ZLIB_RETURN => {
                /* Compile errors here mean that zlib now uses the value co-opted in
                 * pngpriv.h for PNG_UNEXPECTED_ZLIB_RETURN; update the switch above
                 * and change pngpriv.h.  Note that this message is "... return",
                 * whereas the default/Z_OK one is "... return code".
                 */
                (*png_ptr).zstream.msg = c"unexpected zlib return".as_ptr();
            }

            /* default: and case Z_OK: */
            _ => {
                (*png_ptr).zstream.msg = c"unexpected zlib return code".as_ptr();
            }
        }
    }
}
