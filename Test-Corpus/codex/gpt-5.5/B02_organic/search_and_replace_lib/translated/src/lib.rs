use std::ffi::c_char;
use std::ptr;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn searchAndReplace(
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    let orig_len = unsafe { libc::strlen(orig) };
    let search_len = unsafe { libc::strlen(search) };
    let value_len = unsafe { libc::strlen(value) };

    let mut tmp: *mut c_char = ptr::null_mut();
    let mut tmp_offset: usize = 0;
    let mut total_bytes_allocated: usize = 1;

    let mut p = unsafe { libc::strstr(orig, search) };
    if p.is_null() {
        tmp = unsafe { libc::strdup(orig) };
        return tmp;
    }

    let mut inx_start = unsafe { p.offset_from(orig) as usize };
    let mut from = inx_start + search_len;

    if inx_start > 0 {
        total_bytes_allocated = inx_start + 1;
        tmp = unsafe { libc::malloc(total_bytes_allocated) as *mut c_char };
        if tmp.is_null() {
            return ptr::null_mut();
        }
        unsafe {
            libc::strncpy(tmp, orig, inx_start);
        }
        tmp_offset = inx_start;
    }

    while !p.is_null() {
        total_bytes_allocated += value_len;
        tmp = unsafe { libc::realloc(tmp.cast(), total_bytes_allocated) as *mut c_char };
        if tmp.is_null() {
            return ptr::null_mut();
        }

        unsafe {
            libc::strncpy(tmp.add(tmp_offset), value, total_bytes_allocated - tmp_offset);
        }
        tmp_offset += value_len;

        p = unsafe { libc::strstr(orig.add(inx_start + search_len), search) };
        if !p.is_null() {
            let inx_start2 = unsafe { p.offset_from(orig) as usize };

            if inx_start2 > from {
                let gap = inx_start2 - from;
                total_bytes_allocated += gap;
                tmp = unsafe { libc::realloc(tmp.cast(), total_bytes_allocated) as *mut c_char };
                if tmp.is_null() {
                    return ptr::null_mut();
                }
                unsafe {
                    libc::strncpy(tmp.add(tmp_offset), orig.add(from), gap);
                }
                tmp_offset += gap;
            }

            inx_start = inx_start2;
        }

        from = inx_start + search_len;
    }

    if (from < orig_len) && from > 0 {
        total_bytes_allocated += orig_len - from;
        tmp = unsafe { libc::realloc(tmp.cast(), total_bytes_allocated) as *mut c_char };
        if tmp.is_null() {
            return ptr::null_mut();
        }
        unsafe {
            libc::strncpy(tmp.add(tmp_offset), orig.add(from), orig_len - from);
        }
    }

    unsafe {
        *tmp.add(total_bytes_allocated - 1) = 0;
    }

    tmp
}
