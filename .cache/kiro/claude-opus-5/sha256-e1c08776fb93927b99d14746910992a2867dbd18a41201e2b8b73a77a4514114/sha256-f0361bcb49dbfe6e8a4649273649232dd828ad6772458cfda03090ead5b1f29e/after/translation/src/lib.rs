//! Rust translation of `c_src/src/lib.c`.
//!
//! The C library exports exactly one public symbol, `searchAndReplace`
//! (declared in `c_src/include/lib.h`, no namespace/renaming macros are
//! involved). The translation below reproduces the original control flow,
//! allocation strategy and quirks (including `strncpy`'s NUL-padding
//! behaviour and the non-terminating behaviour for an empty `search`
//! string) byte for byte.
//!
//! The returned buffer is allocated with the platform `malloc`/`realloc`
//! (and `strdup`), so callers may release it with `free()` exactly as with
//! the C implementation.

use std::ffi::{c_char, c_void};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
}

/// Equivalent of `strstr(hay + start, needle)`, returning the offset of the
/// match relative to the beginning of `hay`.
///
/// `hay` holds the bytes of a NUL-terminated string (without the NUL) and
/// `start <= hay.len()`. As with `strstr`, an empty `needle` matches
/// immediately at `start`.
fn c_strstr(hay: &[u8], start: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(start);
    }
    if needle.len() > hay.len() {
        return None;
    }
    let last = hay.len() - needle.len();
    let mut i = start;
    while i <= last {
        if &hay[i..i + needle.len()] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Equivalent of `strncpy(dst, src, n)` where `src` holds the bytes of the
/// source NUL-terminated string (without the NUL): at most `n` bytes are
/// copied and, if the source is shorter than `n`, the remainder of the `n`
/// destination bytes is filled with NULs.
///
/// # Safety
/// `dst` must be valid for `n` writes.
unsafe fn c_strncpy(dst: *mut u8, src: &[u8], n: usize) {
    let copied = if src.len() < n { src.len() } else { n };
    std::ptr::copy_nonoverlapping(src.as_ptr(), dst, copied);
    if copied < n {
        std::ptr::write_bytes(dst.add(copied), 0u8, n - copied);
    }
}

/// See `searchAndReplace` in `c_src/src/lib.c`.
///
/// # Safety
/// `orig`, `search` and `value` must be valid NUL-terminated C strings, as
/// required by the original C implementation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn searchAndReplace(
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    let p: Option<usize>;
    let orig_len: usize = strlen(orig);
    let search_len: usize = strlen(search);
    let value_len: usize = strlen(value);

    let mut inx_start: usize;
    let mut tmp: *mut c_char = std::ptr::null_mut();
    let mut tmp_offset: usize = 0;
    let mut total_bytes_allocated: usize = 1;
    let mut from: usize;

    let orig_b: &[u8] = std::slice::from_raw_parts(orig as *const u8, orig_len);
    let search_b: &[u8] = std::slice::from_raw_parts(search as *const u8, search_len);
    let value_b: &[u8] = std::slice::from_raw_parts(value as *const u8, value_len);

    /* Check for any match */
    p = c_strstr(orig_b, 0, search_b);
    if p.is_none() {
        tmp = strdup(orig);
        return tmp;
    }

    inx_start = p.unwrap();
    from = inx_start + search_len;

    /* Copy content before first match, if any */
    if inx_start > 0 {
        total_bytes_allocated = inx_start + 1;
        tmp = malloc(std::mem::size_of::<c_char>() * total_bytes_allocated) as *mut c_char;
        if tmp.is_null() {
            return std::ptr::null_mut();
        }
        c_strncpy(tmp as *mut u8, orig_b, inx_start);
        tmp_offset = inx_start;
    }

    let mut p = p;
    while p.is_some() {
        /* Copy replacement */
        total_bytes_allocated += value_len;
        tmp = realloc(tmp as *mut c_void, total_bytes_allocated) as *mut c_char;
        if tmp.is_null() {
            return std::ptr::null_mut();
        }

        c_strncpy(
            (tmp as *mut u8).add(tmp_offset),
            value_b,
            total_bytes_allocated - tmp_offset,
        );
        tmp_offset += value_len;

        /* Search for further occurrences */
        p = c_strstr(orig_b, inx_start + search_len, search_b);
        if let Some(inx_start2) = p {
            /* Copy content between matches, if any */
            if inx_start2 > from {
                let gap = inx_start2 - from;
                total_bytes_allocated += gap;
                tmp = realloc(tmp as *mut c_void, total_bytes_allocated) as *mut c_char;
                if tmp.is_null() {
                    return std::ptr::null_mut();
                }
                c_strncpy((tmp as *mut u8).add(tmp_offset), &orig_b[from..], gap);
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
            return std::ptr::null_mut();
        }
        c_strncpy(
            (tmp as *mut u8).add(tmp_offset),
            &orig_b[from..],
            orig_len - from,
        );
    }

    *(tmp as *mut u8).add(total_bytes_allocated - 1) = 0u8;

    tmp
}
