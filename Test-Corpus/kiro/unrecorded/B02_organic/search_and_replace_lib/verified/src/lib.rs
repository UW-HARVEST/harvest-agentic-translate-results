use std::ffi::c_char;
use std::ptr;

unsafe fn libc_malloc(size: usize) -> *mut c_char {
    unsafe { libc_realloc(ptr::null_mut(), size) }
}

unsafe fn libc_realloc(p: *mut c_char, size: usize) -> *mut c_char {
    extern "C" {
        fn realloc(ptr: *mut c_char, size: usize) -> *mut c_char;
    }
    unsafe { realloc(p, size) }
}

unsafe fn libc_strdup(s: *const c_char) -> *mut c_char {
    extern "C" {
        fn strdup(s: *const c_char) -> *mut c_char;
    }
    unsafe { strdup(s) }
}

unsafe fn libc_strlen(s: *const c_char) -> usize {
    extern "C" {
        fn strlen(s: *const c_char) -> usize;
    }
    unsafe { strlen(s) }
}

unsafe fn libc_strstr(haystack: *const c_char, needle: *const c_char) -> *const c_char {
    extern "C" {
        fn strstr(haystack: *const c_char, needle: *const c_char) -> *const c_char;
    }
    unsafe { strstr(haystack, needle) }
}

unsafe fn libc_strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char {
    extern "C" {
        fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    }
    unsafe { strncpy(dst, src, n) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn searchAndReplace(
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    unsafe {
        let orig_len = libc_strlen(orig);
        let search_len = libc_strlen(search);
        let value_len = libc_strlen(value);

        let mut p = libc_strstr(orig, search);
        if p.is_null() {
            return libc_strdup(orig);
        }

        let mut inx_start = p.offset_from(orig) as usize;
        let mut from = inx_start + search_len;

        let mut tmp: *mut c_char = ptr::null_mut();
        let mut tmp_offset: usize = 0;
        let mut total_bytes_allocated: usize = 1;

        if inx_start > 0 {
            total_bytes_allocated = inx_start + 1;
            tmp = libc_malloc(total_bytes_allocated);
            if tmp.is_null() {
                return ptr::null_mut();
            }
            libc_strncpy(tmp, orig, inx_start);
            tmp_offset = inx_start;
        }

        while !p.is_null() {
            total_bytes_allocated += value_len;
            tmp = libc_realloc(tmp, total_bytes_allocated);
            if tmp.is_null() {
                return ptr::null_mut();
            }
            libc_strncpy(tmp.add(tmp_offset), value, total_bytes_allocated - tmp_offset);
            tmp_offset += value_len;

            p = libc_strstr(orig.add(inx_start + search_len), search);
            if !p.is_null() {
                let inx_start2 = p.offset_from(orig) as usize;
                if inx_start2 > from {
                    let gap = inx_start2 - from;
                    total_bytes_allocated += gap;
                    tmp = libc_realloc(tmp, total_bytes_allocated);
                    if tmp.is_null() {
                        return ptr::null_mut();
                    }
                    libc_strncpy(tmp.add(tmp_offset), orig.add(from), gap);
                    tmp_offset += gap;
                }
                inx_start = inx_start2;
            }

            from = inx_start + search_len;
        }

        if from < orig_len && from > 0 {
            total_bytes_allocated += orig_len - from;
            tmp = libc_realloc(tmp, total_bytes_allocated);
            if tmp.is_null() {
                return ptr::null_mut();
            }
            libc_strncpy(tmp.add(tmp_offset), orig.add(from), orig_len - from);
        }

        *tmp.add(total_bytes_allocated - 1) = 0;

        tmp
    }
}
