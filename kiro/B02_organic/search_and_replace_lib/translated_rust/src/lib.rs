use std::ffi::{c_char, c_void};

unsafe extern "C" fn strlen(s: *const c_char) -> usize {
    let mut len = 0usize;
    unsafe {
        while *s.add(len) != 0 {
            len += 1;
        }
    }
    len
}

unsafe fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char {
    let needle_len = unsafe { strlen(needle) };
    if needle_len == 0 {
        return haystack as *mut c_char;
    }
    let haystack_len = unsafe { strlen(haystack) };
    if needle_len > haystack_len {
        return std::ptr::null_mut();
    }
    for i in 0..=(haystack_len - needle_len) {
        let mut found = true;
        for j in 0..needle_len {
            if unsafe { *haystack.add(i + j) != *needle.add(j) } {
                found = false;
                break;
            }
        }
        if found {
            return unsafe { haystack.add(i) as *mut c_char };
        }
    }
    std::ptr::null_mut()
}

unsafe fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char {
    for i in 0..n {
        let ch = unsafe { *src.add(i) };
        unsafe { *dst.add(i) = ch };
        if ch == 0 {
            // strncpy zero-fills remainder
            for j in (i + 1)..n {
                unsafe { *dst.add(j) = 0 };
            }
            return dst;
        }
    }
    dst
}

unsafe fn strdup(s: *const c_char) -> *mut c_char {
    let len = unsafe { strlen(s) };
    let p = unsafe { libc_malloc(len + 1) } as *mut c_char;
    if p.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { std::ptr::copy_nonoverlapping(s, p, len + 1) };
    p
}

unsafe fn libc_malloc(size: usize) -> *mut c_void {
    // Use the system allocator via libc layout
    if size == 0 {
        return std::ptr::null_mut();
    }
    let layout = std::alloc::Layout::from_size_align(size, 1).unwrap();
    let ptr = unsafe { std::alloc::alloc(layout) };
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    ptr as *mut c_void
}

unsafe fn libc_realloc(ptr: *mut c_void, old_size: usize, new_size: usize) -> *mut c_void {
    if new_size == 0 {
        return std::ptr::null_mut();
    }
    if ptr.is_null() {
        return unsafe { libc_malloc(new_size) };
    }
    let old_layout = std::alloc::Layout::from_size_align(old_size, 1).unwrap();
    let new_ptr = unsafe { std::alloc::realloc(ptr as *mut u8, old_layout, new_size) };
    if new_ptr.is_null() {
        return std::ptr::null_mut();
    }
    new_ptr as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn searchAndReplace(
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    let orig_len = unsafe { strlen(orig) };
    let search_len = unsafe { strlen(search) };
    let value_len = unsafe { strlen(value) };

    let mut inx_start: usize;
    let mut tmp: *mut c_char = std::ptr::null_mut();
    let mut tmp_offset: usize = 0;
    let mut total_bytes_allocated: usize = 1;
    let mut from: usize;

    // We need to track the previous allocation size for realloc
    let mut prev_alloc: usize = 0;

    /* Check for any match */
    let mut p = unsafe { strstr(orig, search) };
    if p.is_null() {
        let tmp = unsafe { strdup(orig) };
        return tmp;
    }

    inx_start = (p as usize) - (orig as usize);
    from = inx_start + search_len;

    /* Copy content before first match, if any */
    if inx_start > 0 {
        total_bytes_allocated = inx_start + 1;
        tmp = unsafe { libc_malloc(total_bytes_allocated) } as *mut c_char;
        prev_alloc = total_bytes_allocated;
        if tmp.is_null() {
            return std::ptr::null_mut();
        }
        unsafe { strncpy(tmp, orig, inx_start) };
        tmp_offset = inx_start;
    }

    while !p.is_null() {
        /* Copy replacement */
        let old_alloc = prev_alloc;
        total_bytes_allocated += value_len;
        tmp = unsafe { libc_realloc(tmp as *mut c_void, old_alloc, total_bytes_allocated) }
            as *mut c_char;
        prev_alloc = total_bytes_allocated;
        if tmp.is_null() {
            return std::ptr::null_mut();
        }

        unsafe {
            strncpy(
                tmp.add(tmp_offset),
                value,
                total_bytes_allocated - tmp_offset,
            )
        };
        tmp_offset += value_len;

        /* Search for further occurrences */
        p = unsafe { strstr(orig.add(inx_start + search_len), search) };
        if !p.is_null() {
            let inx_start2 = (p as usize) - (orig as usize);

            /* Copy content between matches, if any */
            if inx_start2 > from {
                let gap = inx_start2 - from;
                let old_alloc2 = prev_alloc;
                total_bytes_allocated += gap;
                tmp = unsafe {
                    libc_realloc(tmp as *mut c_void, old_alloc2, total_bytes_allocated)
                } as *mut c_char;
                prev_alloc = total_bytes_allocated;
                if tmp.is_null() {
                    return std::ptr::null_mut();
                }
                unsafe { strncpy(tmp.add(tmp_offset), orig.add(from), gap) };
                tmp_offset += gap;
            }

            inx_start = inx_start2;
        }

        /* Set position for copying content after last match */
        from = inx_start + search_len;
    }

    /* Copy content after last match, if any */
    if from < orig_len && from > 0 {
        let old_alloc3 = prev_alloc;
        total_bytes_allocated += orig_len - from;
        tmp = unsafe { libc_realloc(tmp as *mut c_void, old_alloc3, total_bytes_allocated) }
            as *mut c_char;
        if tmp.is_null() {
            return std::ptr::null_mut();
        }
        unsafe { strncpy(tmp.add(tmp_offset), orig.add(from), orig_len - from) };
    }

    unsafe { *tmp.add(total_bytes_allocated - 1) = 0 };

    tmp
}
