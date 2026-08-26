use core::ffi::{c_char, c_void};
use core::ptr;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
    fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;
    fn strncpy(dest: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn strdup(s: *const c_char) -> *mut c_char;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn searchAndReplace(
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    let orig_len: usize = strlen(orig);
    let search_len: usize = strlen(search);
    let value_len: usize = strlen(value);

    let mut tmp: *mut c_char = ptr::null_mut();
    let mut tmp_offset: usize = 0;
    let mut total_bytes_allocated: usize = 1;

    /* Check for any match */
    let mut p: *mut c_char = strstr(orig, search);
    if p.is_null() {
        let dup: *mut c_char = strdup(orig);
        return dup;
    }

    let mut inx_start: usize = (p as usize).wrapping_sub(orig as usize);
    let mut from: usize = inx_start + search_len;

    /* Copy content before first match, if any */
    if inx_start > 0 {
        total_bytes_allocated = inx_start + 1;
        tmp = malloc(core::mem::size_of::<c_char>() * total_bytes_allocated) as *mut c_char;
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
            let inx_start2: usize = (p as usize).wrapping_sub(orig as usize);

            /* Copy content between matches, if any */
            if inx_start2 > from {
                let gap: usize = inx_start2 - from;
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
