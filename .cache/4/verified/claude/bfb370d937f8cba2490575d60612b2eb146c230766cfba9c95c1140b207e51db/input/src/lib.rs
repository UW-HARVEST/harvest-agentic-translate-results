//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (exactly matching `nm -D` of the C shared library):
//!   * `searchAndReplace`
//!
//! The returned buffers are allocated with the C allocator (`malloc`/`realloc`)
//! so that callers may `free()` them, exactly like the original C code.
//!
//! The translation is behaviour-preserving down to the byte level, including the
//! quirks of the original implementation: the `strncpy` zero padding that
//! implicitly NUL-terminates the intermediate buffers, the redundant `from > 0`
//! guard on the trailing copy, the leaks of the previous buffer when `realloc`
//! fails, and the endless loop that an empty `search` string provokes.

use std::ffi::{c_char, c_void};
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
}

/// `strlen(3)`
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    unsafe {
        while *s.add(n) != 0 {
            n += 1;
        }
    }
    n
}

/// `strstr(3)` — returns the first occurrence of `needle` in `haystack`;
/// an empty needle yields `haystack` itself (glibc behaviour).
unsafe fn c_strstr(haystack: *const c_char, needle: *const c_char) -> *const c_char {
    unsafe {
        let h = std::slice::from_raw_parts(haystack as *const u8, c_strlen(haystack));
        let n = std::slice::from_raw_parts(needle as *const u8, c_strlen(needle));

        if n.is_empty() {
            return haystack;
        }
        if n.len() > h.len() {
            return ptr::null();
        }
        for i in 0..=(h.len() - n.len()) {
            if &h[i..i + n.len()] == n {
                return haystack.add(i);
            }
        }
        ptr::null()
    }
}

/// `strncpy(3)` — copies at most `n` bytes and zero-pads the remainder of the
/// `n` bytes when `src` is shorter.  The padding is load bearing here: it is
/// what NUL-terminates the intermediate buffers in the algorithm below.
unsafe fn c_strncpy(dst: *mut c_char, src: *const c_char, n: usize) {
    unsafe {
        let mut i = 0usize;
        while i < n {
            let b = *src.add(i);
            *dst.add(i) = b;
            if b == 0 {
                break;
            }
            i += 1;
        }
        while i < n {
            *dst.add(i) = 0;
            i += 1;
        }
    }
}

/// `strdup(3)` using the C allocator.
unsafe fn c_strdup(s: *const c_char) -> *mut c_char {
    unsafe {
        let len = c_strlen(s);
        let p = malloc(len + 1) as *mut c_char;
        if p.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(s, p, len + 1);
        p
    }
}

#[allow(non_snake_case)]
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

        let mut inx_start: usize;
        let mut tmp: *mut c_char = ptr::null_mut();
        let mut tmp_offset: usize = 0;
        let mut total_bytes_allocated: usize = 1;
        let mut from: usize;

        /* Check for any match */
        let mut p = c_strstr(orig, search);
        if p.is_null() {
            return c_strdup(orig);
        }

        inx_start = p.offset_from(orig) as usize;
        from = inx_start + search_len;

        /* Copy content before first match, if any */
        if inx_start > 0 {
            total_bytes_allocated = inx_start + 1;
            tmp = malloc(size_of::<c_char>() * total_bytes_allocated) as *mut c_char;
            if tmp.is_null() {
                return ptr::null_mut();
            }
            c_strncpy(tmp, orig, inx_start);
            tmp_offset = inx_start;
        }

        while !p.is_null() {
            /* Copy replacement */
            total_bytes_allocated += value_len;
            tmp = realloc(tmp as *mut c_void, total_bytes_allocated) as *mut c_char;
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
                let inx_start2 = p.offset_from(orig) as usize;

                /* Copy content between matches, if any */
                if inx_start2 > from {
                    let gap = inx_start2 - from;
                    total_bytes_allocated += gap;
                    tmp = realloc(tmp as *mut c_void, total_bytes_allocated) as *mut c_char;
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
            tmp = realloc(tmp as *mut c_void, total_bytes_allocated) as *mut c_char;
            if tmp.is_null() {
                return ptr::null_mut();
            }
            c_strncpy(tmp.add(tmp_offset), orig.add(from), orig_len - from);
        }

        *tmp.add(total_bytes_allocated - 1) = 0;

        tmp
    }
}
