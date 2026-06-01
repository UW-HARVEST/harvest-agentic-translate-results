use libc::{c_void, malloc, realloc, size_t, strdup, strlen, strncpy, strstr};
use std::ffi::c_char;
use std::ptr;

/// Translation of `searchAndReplace` from c_src/src/lib.c.
///
/// This intentionally mirrors the C implementation's exact byte-level
/// behaviour, including its use of `strncpy` (which does not always write a
/// terminating NUL) and its quirks around `realloc` failure paths. The
/// allocator routines from libc are used so the caller can `free()` the
/// returned pointer using the C runtime's allocator, exactly as if it had been
/// returned by the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn searchAndReplace(
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    let orig_len: size_t = strlen(orig);
    let search_len: size_t = strlen(search);
    let value_len: size_t = strlen(value);

    let mut tmp: *mut c_char = ptr::null_mut();
    let mut tmp_offset: size_t = 0;
    let mut total_bytes_allocated: size_t = 1;

    /* Check for any match */
    let mut p = strstr(orig, search);
    if p.is_null() {
        return strdup(orig);
    }

    let mut inx_start: size_t = (p as usize).wrapping_sub(orig as usize) as size_t;
    let mut from: size_t = inx_start + search_len;

    /* Copy content before first match, if any */
    if inx_start > 0 {
        total_bytes_allocated = inx_start + 1;
        tmp = malloc(total_bytes_allocated) as *mut c_char;
        if tmp.is_null() {
            return ptr::null_mut();
        }
        strncpy(tmp, orig, inx_start);
        tmp_offset = inx_start;
    }

    while !p.is_null() {
        /* Copy replacement */
        total_bytes_allocated += value_len;
        tmp = realloc(tmp as *mut c_void, total_bytes_allocated) as *mut c_char;
        if tmp.is_null() {
            return ptr::null_mut();
        }

        strncpy(
            tmp.add(tmp_offset),
            value,
            total_bytes_allocated - tmp_offset,
        );
        tmp_offset += value_len;

        /* Search for further occurrences */
        p = strstr(orig.add(inx_start + search_len), search);
        if !p.is_null() {
            let inx_start2: size_t = (p as usize).wrapping_sub(orig as usize) as size_t;

            /* Copy content between matches, if any */
            if inx_start2 > from {
                let gap = inx_start2 - from;
                total_bytes_allocated += gap;
                tmp = realloc(tmp as *mut c_void, total_bytes_allocated) as *mut c_char;
                if tmp.is_null() {
                    return ptr::null_mut();
                }
                strncpy(tmp.add(tmp_offset), orig.add(from), gap);
                tmp_offset += gap;
            }

            inx_start = inx_start2;
        }

        /* Set position for copying content after last match */
        from = inx_start + search_len;
    }

    /* Copy content after last match, if any */
    if (from < orig_len) && from > 0 {
        total_bytes_allocated += orig_len - from;
        tmp = realloc(tmp as *mut c_void, total_bytes_allocated) as *mut c_char;
        if tmp.is_null() {
            return ptr::null_mut();
        }
        strncpy(tmp.add(tmp_offset), orig.add(from), orig_len - from);
    }

    *tmp.add(total_bytes_allocated - 1) = 0;

    tmp
}
