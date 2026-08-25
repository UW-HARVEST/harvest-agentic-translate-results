use std::ffi::{c_char, c_void};
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn strdup(value: *const c_char) -> *mut c_char;
    fn strlen(value: *const c_char) -> usize;
    fn strncpy(dest: *mut c_char, src: *const c_char, count: usize) -> *mut c_char;
    fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn searchAndReplace(
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    unsafe {
        let orig_len = strlen(orig);
        let search_len = strlen(search);
        let value_len = strlen(value);

        let mut tmp: *mut c_char = ptr::null_mut();
        let mut tmp_offset = 0usize;
        let mut total_bytes_allocated = 1usize;

        let mut p = strstr(orig, search);
        if p.is_null() {
            return strdup(orig);
        }

        let mut inx_start = p.offset_from(orig) as usize;
        let mut from = inx_start.wrapping_add(search_len);

        if inx_start > 0 {
            total_bytes_allocated = inx_start.wrapping_add(1);
            tmp = malloc(total_bytes_allocated).cast();
            if tmp.is_null() {
                return ptr::null_mut();
            }
            strncpy(tmp, orig, inx_start);
            tmp_offset = inx_start;
        }

        while !p.is_null() {
            total_bytes_allocated = total_bytes_allocated.wrapping_add(value_len);
            tmp = realloc(tmp.cast(), total_bytes_allocated).cast();
            if tmp.is_null() {
                return ptr::null_mut();
            }

            strncpy(
                tmp.add(tmp_offset),
                value,
                total_bytes_allocated.wrapping_sub(tmp_offset),
            );
            tmp_offset = tmp_offset.wrapping_add(value_len);

            p = strstr(orig.add(inx_start).add(search_len), search);
            if !p.is_null() {
                let inx_start2 = p.offset_from(orig) as usize;

                if inx_start2 > from {
                    let gap = inx_start2.wrapping_sub(from);
                    total_bytes_allocated = total_bytes_allocated.wrapping_add(gap);
                    tmp = realloc(tmp.cast(), total_bytes_allocated).cast();
                    if tmp.is_null() {
                        return ptr::null_mut();
                    }
                    strncpy(tmp.add(tmp_offset), orig.add(from), gap);
                    tmp_offset = tmp_offset.wrapping_add(gap);
                }

                inx_start = inx_start2;
            }

            from = inx_start.wrapping_add(search_len);
        }

        if from < orig_len && from > 0 {
            total_bytes_allocated = total_bytes_allocated.wrapping_add(orig_len.wrapping_sub(from));
            tmp = realloc(tmp.cast(), total_bytes_allocated).cast();
            if tmp.is_null() {
                return ptr::null_mut();
            }
            strncpy(
                tmp.add(tmp_offset),
                orig.add(from),
                orig_len.wrapping_sub(from),
            );
        }

        *tmp.add(total_bytes_allocated.wrapping_sub(1)) = 0;
        tmp
    }
}
