use std::ffi::c_char;
use std::ptr;

/// Compute strlen of a NUL-terminated C string.
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut len: usize = 0;
    unsafe {
        while *s.add(len) != 0 {
            len += 1;
        }
    }
    len
}

/// Find the first occurrence of `needle` (length `needle_len`) within
/// `haystack` (length `haystack_len`). Returns a pointer to the start of
/// the match within haystack, or NULL.
///
/// Mirrors C `strstr` semantics, including the special case where an empty
/// needle returns haystack.
unsafe fn c_strstr(
    haystack: *const c_char,
    haystack_len: usize,
    needle: *const c_char,
    needle_len: usize,
) -> *const c_char {
    if needle_len == 0 {
        return haystack;
    }
    if needle_len > haystack_len {
        return ptr::null();
    }
    let max = haystack_len - needle_len;
    let mut i: usize = 0;
    while i <= max {
        let mut matched = true;
        let mut j: usize = 0;
        while j < needle_len {
            unsafe {
                if *haystack.add(i + j) != *needle.add(j) {
                    matched = false;
                    break;
                }
            }
            j += 1;
        }
        if matched {
            return unsafe { haystack.add(i) };
        }
        i += 1;
    }
    ptr::null()
}

/// Mirrors C `strncpy`: copies up to `n` bytes from `src` to `dst`. If
/// `src` (NUL-terminated) is shorter than `n`, the remainder of `dst` is
/// padded with NUL bytes. Does NOT NUL-terminate `dst` if `src` length
/// >= `n`.
unsafe fn c_strncpy(dst: *mut c_char, src: *const c_char, n: usize) {
    let mut i: usize = 0;
    let mut hit_null = false;
    while i < n {
        unsafe {
            if !hit_null {
                let ch = *src.add(i);
                *dst.add(i) = ch;
                if ch == 0 {
                    hit_null = true;
                }
            } else {
                *dst.add(i) = 0;
            }
        }
        i += 1;
    }
}

/// Mirrors C `strdup`: allocates with `malloc` and copies the
/// NUL-terminated string into the new buffer.
unsafe fn c_strdup(s: *const c_char) -> *mut c_char {
    unsafe {
        let len = c_strlen(s);
        let total = len + 1;
        let p = libc::malloc(total) as *mut c_char;
        if p.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(s, p, total);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn searchAndReplace(
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    unsafe {
        let orig_len = c_strlen(orig);
        let search_len = c_strlen(search);
        let value_len = c_strlen(value);

        let mut tmp: *mut c_char = ptr::null_mut();
        let mut tmp_offset: usize = 0;
        let mut total_bytes_allocated: usize = 1;

        /* Check for any match */
        let mut p = c_strstr(orig, orig_len, search, search_len);
        if p.is_null() {
            tmp = c_strdup(orig);
            return tmp;
        }

        let mut inx_start: usize = p.offset_from(orig) as usize;
        let mut from: usize = inx_start + search_len;

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
            tmp = libc::realloc(tmp as *mut libc::c_void, total_bytes_allocated) as *mut c_char;
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
            let search_from = orig.add(inx_start + search_len);
            let search_from_len = orig_len - (inx_start + search_len);
            p = c_strstr(search_from, search_from_len, search, search_len);
            if !p.is_null() {
                let inx_start2: usize = p.offset_from(orig) as usize;

                /* Copy content between matches, if any */
                if inx_start2 > from {
                    let gap = inx_start2 - from;
                    total_bytes_allocated += gap;
                    tmp =
                        libc::realloc(tmp as *mut libc::c_void, total_bytes_allocated) as *mut c_char;
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
            tmp = libc::realloc(tmp as *mut libc::c_void, total_bytes_allocated) as *mut c_char;
            if tmp.is_null() {
                return ptr::null_mut();
            }
            c_strncpy(tmp.add(tmp_offset), orig.add(from), orig_len - from);
        }

        *tmp.add(total_bytes_allocated - 1) = 0;

        tmp
    }
}
