use std::ffi::c_char;

use libc::{c_void, free, malloc, memcpy, realloc, size_t, strdup, strlen, strstr};

/// Mirror of C `strncpy`: copies up to `n` bytes from `src` to `dst`.
/// If a NUL byte from `src` is reached before `n` bytes are written, the
/// remainder of `dst` (up to `n`) is padded with NUL bytes. If `src` has
/// `n` or more non-NUL bytes, no NUL terminator is written.
unsafe fn strncpy_impl(dst: *mut u8, src: *const u8, n: usize) {
    let mut i: usize = 0;
    let mut hit_null = false;
    while i < n {
        if !hit_null {
            let c = unsafe { *src.add(i) };
            unsafe { *dst.add(i) = c };
            if c == 0 {
                hit_null = true;
            }
        } else {
            unsafe { *dst.add(i) = 0 };
        }
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn searchAndReplace(
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    let mut p: *const c_char;
    let orig_len: size_t = unsafe { strlen(orig) };
    let search_len: size_t = unsafe { strlen(search) };
    let value_len: size_t = unsafe { strlen(value) };

    let mut inx_start: size_t;
    let mut tmp: *mut c_char = std::ptr::null_mut();
    let mut tmp_offset: size_t = 0;
    let mut total_bytes_allocated: size_t = 1;
    let mut from: size_t;

    /* Check for any match */
    p = unsafe { strstr(orig, search) };
    if p.is_null() {
        let dup = unsafe { strdup(orig) };
        return dup;
    }

    inx_start = (p as usize) - (orig as usize);
    from = inx_start + search_len;

    /* Copy content before first match, if any */
    if inx_start > 0 {
        total_bytes_allocated = inx_start + 1;
        tmp = unsafe { malloc(total_bytes_allocated) } as *mut c_char;
        if tmp.is_null() {
            return std::ptr::null_mut();
        }
        unsafe {
            strncpy_impl(tmp as *mut u8, orig as *const u8, inx_start);
        }
        tmp_offset = inx_start;
    }

    while !p.is_null() {
        /* Copy replacement */
        total_bytes_allocated += value_len;
        tmp = unsafe { realloc(tmp as *mut c_void, total_bytes_allocated) } as *mut c_char;
        if tmp.is_null() {
            return std::ptr::null_mut();
        }

        unsafe {
            strncpy_impl(
                (tmp as *mut u8).add(tmp_offset),
                value as *const u8,
                total_bytes_allocated - tmp_offset,
            );
        }
        tmp_offset += value_len;

        /* Search for further occurrences */
        p = unsafe { strstr(orig.add(inx_start + search_len), search) };
        if !p.is_null() {
            let inx_start2: size_t = (p as usize) - (orig as usize);

            /* Copy content between matches, if any */
            if inx_start2 > from {
                let gap: size_t = inx_start2 - from;
                total_bytes_allocated += gap;
                tmp = unsafe { realloc(tmp as *mut c_void, total_bytes_allocated) } as *mut c_char;
                if tmp.is_null() {
                    return std::ptr::null_mut();
                }
                unsafe {
                    strncpy_impl(
                        (tmp as *mut u8).add(tmp_offset),
                        (orig as *const u8).add(from),
                        gap,
                    );
                }
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
        tmp = unsafe { realloc(tmp as *mut c_void, total_bytes_allocated) } as *mut c_char;
        if tmp.is_null() {
            return std::ptr::null_mut();
        }
        unsafe {
            strncpy_impl(
                (tmp as *mut u8).add(tmp_offset),
                (orig as *const u8).add(from),
                orig_len - from,
            );
        }
    }

    unsafe {
        *tmp.add(total_bytes_allocated - 1) = 0;
    }

    // Suppress unused warnings for items intentionally kept to mirror C structure.
    let _ = (memcpy, free);

    tmp
}
