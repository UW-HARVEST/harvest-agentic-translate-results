//! Rust translation of the C library in `c_src/`.
//!
//! The C library (`c_src/src/lib.c`, public header `c_src/include/lib.h`) has a
//! single public entry point:
//!
//! ```c
//! char *searchAndReplace(const char *orig, const char *search, const char *value);
//! ```
//!
//! There are no namespace/renaming macros in the public header, so the final
//! linker symbol is literally `searchAndReplace`.
//!
//! The translation below is deliberately literal: the same variables, the same
//! order of operations, the same allocation sizes, and the same (buggy) edge
//! cases as the original C. In particular:
//!
//! * The returned buffer is allocated with the *C* allocator (`malloc`,
//!   `realloc`, `strdup`) so that callers can release it with `free()`, exactly
//!   as with the C library.
//! * When `orig` contains no occurrence of `search`, the result is `strdup(orig)`.
//! * On an allocation failure the function returns `NULL`, leaking whatever had
//!   already been allocated - just like the C.
//! * The trailing `if ((from < orig_len) && from > 0)` guard is reproduced
//!   verbatim, including the `from > 0` term (which suppresses the tail copy for
//!   a zero-length `search` matching at index 0).
//! * `strncpy`'s "stop at NUL, then pad the remainder with NUL" behaviour is
//!   reproduced faithfully, as is `strstr`'s "empty needle matches at the start
//!   of the haystack" behaviour (which makes the C loop spin forever for an
//!   empty `search`; that is preserved rather than "fixed").

use core::ffi::{c_char, CStr};
use core::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_char;
    fn realloc(ptr: *mut c_char, size: usize) -> *mut c_char;
    fn strdup(s: *const c_char) -> *mut c_char;
}

/// `strlen(3)`.
#[inline]
unsafe fn c_strlen(s: *const c_char) -> usize {
    unsafe { CStr::from_ptr(s) }.to_bytes().len()
}

/// `strstr(3)`.
///
/// Mirrors the C contract, including the special case of an empty needle, for
/// which the haystack pointer itself is returned.
unsafe fn c_strstr(haystack: *const c_char, needle: *const c_char) -> *const c_char {
    let needle_bytes = unsafe { CStr::from_ptr(needle) }.to_bytes();
    if needle_bytes.is_empty() {
        return haystack;
    }

    let haystack_bytes = unsafe { CStr::from_ptr(haystack) }.to_bytes();
    if needle_bytes.len() > haystack_bytes.len() {
        return ptr::null();
    }

    let last = haystack_bytes.len() - needle_bytes.len();
    for i in 0..=last {
        if &haystack_bytes[i..i + needle_bytes.len()] == needle_bytes {
            return unsafe { haystack.add(i) };
        }
    }

    ptr::null()
}

/// `strncpy(3)`: copy at most `n` bytes from `src`, stopping after the
/// terminating NUL, then pad `dst` with NUL bytes up to exactly `n` bytes.
unsafe fn c_strncpy(dst: *mut c_char, src: *const c_char, n: usize) {
    let mut i = 0usize;
    while i < n {
        let ch = unsafe { *src.add(i) };
        unsafe { *dst.add(i) = ch };
        if ch == 0 {
            break;
        }
        i += 1;
    }
    while i < n {
        unsafe { *dst.add(i) = 0 };
        i += 1;
    }
}

/// Replace every occurrence of `search` in `orig` with `value`.
///
/// Returns a newly `malloc`'d NUL-terminated string (to be released with
/// `free`), or `NULL` if an allocation failed.
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn searchAndReplace(
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    let mut p: *const c_char;
    let orig_len: usize = unsafe { c_strlen(orig) };
    let search_len: usize = unsafe { c_strlen(search) };
    let value_len: usize = unsafe { c_strlen(value) };

    let mut inx_start: usize;
    let mut tmp: *mut c_char = ptr::null_mut();
    let mut tmp_offset: usize = 0;
    let mut total_bytes_allocated: usize = 1;
    let mut from: usize;

    /* Check for any match */
    p = unsafe { c_strstr(orig, search) };
    if p.is_null() {
        tmp = unsafe { strdup(orig) };
        return tmp;
    }

    inx_start = unsafe { p.offset_from(orig) } as usize;
    from = inx_start + search_len;

    /* Copy content before first match, if any */
    if inx_start > 0 {
        total_bytes_allocated = inx_start + 1;
        tmp = unsafe { malloc(core::mem::size_of::<c_char>() * total_bytes_allocated) };
        if tmp.is_null() {
            return ptr::null_mut();
        }
        unsafe { c_strncpy(tmp, orig, inx_start) };
        tmp_offset = inx_start;
    }

    while !p.is_null() {
        /* Copy replacement */
        total_bytes_allocated += value_len;
        tmp = unsafe { realloc(tmp, total_bytes_allocated) };
        if tmp.is_null() {
            return ptr::null_mut();
        }

        unsafe {
            c_strncpy(
                tmp.add(tmp_offset),
                value,
                total_bytes_allocated - tmp_offset,
            )
        };
        tmp_offset += value_len;

        /* Search for further occurrences */
        p = unsafe { c_strstr(orig.add(inx_start + search_len), search) };
        if !p.is_null() {
            let inx_start2 = unsafe { p.offset_from(orig) } as usize;

            /* Copy content between matches, if any */
            if inx_start2 > from {
                let gap = inx_start2 - from;
                total_bytes_allocated += gap;
                tmp = unsafe { realloc(tmp, total_bytes_allocated) };
                if tmp.is_null() {
                    return ptr::null_mut();
                }
                unsafe { c_strncpy(tmp.add(tmp_offset), orig.add(from), gap) };
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
        tmp = unsafe { realloc(tmp, total_bytes_allocated) };
        if tmp.is_null() {
            return ptr::null_mut();
        }
        unsafe { c_strncpy(tmp.add(tmp_offset), orig.add(from), orig_len - from) };
    }

    unsafe { *tmp.add(total_bytes_allocated - 1) = 0 };

    tmp
}
