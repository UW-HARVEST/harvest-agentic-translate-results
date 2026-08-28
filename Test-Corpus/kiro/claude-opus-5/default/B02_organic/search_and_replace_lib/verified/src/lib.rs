//! Rust translation of `c_src/src/lib.c`.
//!
//! The behaviour of the original C is reproduced exactly, including its
//! quirks (e.g. an empty `search` string makes the C loop forever, and the
//! returned buffer is allocated with the C allocator so that callers may
//! release it with `free()`).

use std::ffi::{c_char, c_void};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
}

/// `strlen()`
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    unsafe {
        while *s.add(n) != 0 {
            n += 1;
        }
    }
    n
}

/// Borrow a NUL-terminated C string as a byte slice (NUL excluded).
unsafe fn c_bytes<'a>(s: *const c_char, len: usize) -> &'a [u8] {
    unsafe { std::slice::from_raw_parts(s as *const u8, len) }
}

/// `strstr()` semantics, returning the offset of the match inside `hay`.
/// An empty needle matches at offset 0, exactly like glibc's `strstr`.
fn c_strstr(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}

/// `strncpy(dst, src, n)` where `src` is the remainder of a C string.
/// Copies at most `n` bytes and zero-pads the destination up to `n` bytes.
unsafe fn c_strncpy(dst: *mut u8, src: &[u8], n: usize) {
    let copied = if src.len() < n { src.len() } else { n };
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst, copied);
        if copied < n {
            std::ptr::write_bytes(dst.add(copied), 0, n - copied);
        }
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

        let orig_s = c_bytes(orig, orig_len);
        let search_s = c_bytes(search, search_len);
        let value_s = c_bytes(value, value_len);

        let mut inx_start: usize;
        let mut tmp: *mut u8 = std::ptr::null_mut();
        let mut tmp_offset: usize = 0;
        let mut total_bytes_allocated: usize = 1;
        let mut from: usize;

        /* Check for any match */
        let mut p = c_strstr(orig_s, search_s);
        if p.is_none() {
            /* strdup(orig) */
            let dup = malloc(orig_len + 1) as *mut u8;
            if dup.is_null() {
                return std::ptr::null_mut();
            }
            std::ptr::copy_nonoverlapping(orig as *const u8, dup, orig_len);
            *dup.add(orig_len) = 0;
            return dup as *mut c_char;
        }

        inx_start = p.unwrap();
        from = inx_start + search_len;

        /* Copy content before first match, if any */
        if inx_start > 0 {
            total_bytes_allocated = inx_start + 1;
            tmp = malloc(total_bytes_allocated) as *mut u8;
            if tmp.is_null() {
                return std::ptr::null_mut();
            }
            c_strncpy(tmp, orig_s, inx_start);
            tmp_offset = inx_start;
        }

        while p.is_some() {
            /* Copy replacement */
            total_bytes_allocated += value_len;
            tmp = realloc(tmp as *mut c_void, total_bytes_allocated) as *mut u8;
            if tmp.is_null() {
                return std::ptr::null_mut();
            }

            c_strncpy(
                tmp.add(tmp_offset),
                value_s,
                total_bytes_allocated - tmp_offset,
            );
            tmp_offset += value_len;

            /* Search for further occurrences */
            let scan_from = inx_start + search_len;
            p = c_strstr(&orig_s[scan_from..], search_s).map(|off| scan_from + off);
            if let Some(inx_start2) = p {
                /* Copy content between matches, if any */
                if inx_start2 > from {
                    let gap = inx_start2 - from;
                    total_bytes_allocated += gap;
                    tmp = realloc(tmp as *mut c_void, total_bytes_allocated) as *mut u8;
                    if tmp.is_null() {
                        return std::ptr::null_mut();
                    }
                    c_strncpy(tmp.add(tmp_offset), &orig_s[from..], gap);
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
            tmp = realloc(tmp as *mut c_void, total_bytes_allocated) as *mut u8;
            if tmp.is_null() {
                return std::ptr::null_mut();
            }
            c_strncpy(tmp.add(tmp_offset), &orig_s[from..], orig_len - from);
        }

        *tmp.add(total_bytes_allocated - 1) = 0;

        tmp as *mut c_char
    }
}
