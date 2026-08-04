use libc::{c_char, c_void, size_t};
use std::ptr;

/// Compute the length of a null-terminated C string.
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut len: usize = 0;
    while *s.add(len) != 0 {
        len += 1;
    }
    len
}

/// Locate `needle` (null-terminated) in `haystack` (null-terminated).
/// Returns a pointer into `haystack` to the first match, or null if not found.
unsafe fn c_strstr(haystack: *const c_char, needle: *const c_char) -> *const c_char {
    let needle_len = c_strlen(needle);
    if needle_len == 0 {
        return haystack;
    }

    let mut h = haystack;
    loop {
        // Try to match needle at position h.
        let mut i: usize = 0;
        while i < needle_len {
            let hc = *h.add(i);
            if hc == 0 {
                return ptr::null();
            }
            if hc != *needle.add(i) {
                break;
            }
            i += 1;
        }
        if i == needle_len {
            return h;
        }
        if *h == 0 {
            return ptr::null();
        }
        h = h.add(1);
    }
}

/// Duplicate a C string using libc::malloc, mirroring `strdup`.
unsafe fn c_strdup(s: *const c_char) -> *mut c_char {
    let len = c_strlen(s);
    let total = len + 1;
    let p = libc::malloc(total) as *mut c_char;
    if p.is_null() {
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(s, p, total);
    p
}

/// Mirrors `strncpy(dst, src, n)`. Copies at most `n` bytes from `src` to
/// `dst`, stopping early if a NUL byte is encountered. If `src` is shorter
/// than `n`, the remaining bytes in `dst` are zero-filled.
unsafe fn c_strncpy(dst: *mut c_char, src: *const c_char, n: size_t) {
    let n = n as usize;
    let mut i: usize = 0;
    let mut hit_null = false;
    while i < n {
        if !hit_null {
            let c = *src.add(i);
            *dst.add(i) = c;
            if c == 0 {
                hit_null = true;
            }
        } else {
            *dst.add(i) = 0;
        }
        i += 1;
    }
}

/// Replace every occurrence of `search` in `orig` with `value`.
///
/// The returned pointer is allocated with `libc::malloc`/`libc::realloc`
/// and must be released with `libc::free`. Returns NULL on allocation
/// failure.
///
/// # Safety
///
/// `orig`, `search`, and `value` must all be valid pointers to
/// null-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn searchAndReplace(
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    let orig_len: size_t = c_strlen(orig);
    let search_len: size_t = c_strlen(search);
    let value_len: size_t = c_strlen(value);

    let mut tmp: *mut c_char = ptr::null_mut();
    let mut tmp_offset: size_t = 0;
    let mut total_bytes_allocated: size_t = 1;

    /* Check for any match */
    let mut p: *const c_char = c_strstr(orig, search);
    if p.is_null() {
        return c_strdup(orig);
    }

    let mut inx_start: size_t = (p as usize) - (orig as usize);
    let mut from: size_t = inx_start + search_len;

    /* Copy content before first match, if any */
    if inx_start > 0 {
        total_bytes_allocated = inx_start + 1;
        tmp = libc::malloc(total_bytes_allocated) as *mut c_char;
        if tmp.is_null() {
            return ptr::null_mut();
        }
        c_strncpy(tmp, orig, inx_start);
        tmp_offset = inx_start;
    }

    while !p.is_null() {
        /* Copy replacement */
        total_bytes_allocated += value_len;
        tmp = libc::realloc(tmp as *mut c_void, total_bytes_allocated) as *mut c_char;
        if tmp.is_null() {
            return ptr::null_mut();
        }

        c_strncpy(
            tmp.add(tmp_offset),
            value,
            total_bytes_allocated - tmp_offset,
        );
        tmp_offset += value_len;

        /* Search for further occurrences */
        p = c_strstr(orig.add(inx_start + search_len), search);
        if !p.is_null() {
            let inx_start2: size_t = (p as usize) - (orig as usize);

            /* Copy content between matches, if any */
            if inx_start2 > from {
                let gap: size_t = inx_start2 - from;
                total_bytes_allocated += gap;
                tmp = libc::realloc(tmp as *mut c_void, total_bytes_allocated) as *mut c_char;
                if tmp.is_null() {
                    return ptr::null_mut();
                }
                c_strncpy(tmp.add(tmp_offset), orig.add(from), gap);
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
        tmp = libc::realloc(tmp as *mut c_void, total_bytes_allocated) as *mut c_char;
        if tmp.is_null() {
            return ptr::null_mut();
        }
        c_strncpy(tmp.add(tmp_offset), orig.add(from), orig_len - from);
    }

    *tmp.add(total_bytes_allocated - 1) = 0;

    tmp
}
