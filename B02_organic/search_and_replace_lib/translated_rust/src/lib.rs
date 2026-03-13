use std::ffi::c_char;
use std::ptr;

/// Find `needle` in `haystack` starting from byte 0, returning the offset or None.
fn find_substr(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[unsafe(no_mangle)]
pub extern "C" fn searchAndReplace(
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    // Safety: caller must pass valid C strings, matching the C contract.
    unsafe {
        let orig_bytes = {
            let mut len = 0usize;
            while *orig.add(len) != 0 {
                len += 1;
            }
            std::slice::from_raw_parts(orig as *const u8, len)
        };
        let search_bytes = {
            let mut len = 0usize;
            while *search.add(len) != 0 {
                len += 1;
            }
            std::slice::from_raw_parts(search as *const u8, len)
        };
        let value_bytes = {
            let mut len = 0usize;
            while *value.add(len) != 0 {
                len += 1;
            }
            std::slice::from_raw_parts(value as *const u8, len)
        };

        let orig_len = orig_bytes.len();
        let search_len = search_bytes.len();
        let value_len = value_bytes.len();

        // Check for any match
        let first_match = match find_substr(orig_bytes, search_bytes) {
            None => {
                // strdup(orig)
                let dup = libc::malloc(orig_len + 1) as *mut u8;
                if dup.is_null() {
                    return ptr::null_mut();
                }
                ptr::copy_nonoverlapping(orig_bytes.as_ptr(), dup, orig_len);
                *dup.add(orig_len) = 0;
                return dup as *mut c_char;
            }
            Some(pos) => pos,
        };

        let mut inx_start = first_match;
        let mut from = inx_start + search_len;

        let mut tmp: *mut u8 = ptr::null_mut();
        let mut tmp_offset: usize = 0;
        let mut total_bytes_allocated: usize = 1;

        // Copy content before first match, if any
        if inx_start > 0 {
            total_bytes_allocated = inx_start + 1;
            tmp = libc::malloc(total_bytes_allocated) as *mut u8;
            if tmp.is_null() {
                return ptr::null_mut();
            }
            // strncpy(tmp, orig, inx_start)
            ptr::copy_nonoverlapping(orig_bytes.as_ptr(), tmp, inx_start);
            tmp_offset = inx_start;
        }

        // p != NULL for first iteration (we found a match above)
        let mut have_match = true;

        while have_match {
            // Copy replacement
            total_bytes_allocated += value_len;
            tmp = libc::realloc(tmp as *mut libc::c_void, total_bytes_allocated) as *mut u8;
            if tmp.is_null() {
                return ptr::null_mut();
            }

            // strncpy(tmp + tmp_offset, value, total_bytes_allocated - tmp_offset)
            let copy_len = std::cmp::min(value_len, total_bytes_allocated - tmp_offset);
            ptr::copy_nonoverlapping(value_bytes.as_ptr(), tmp.add(tmp_offset), copy_len);
            tmp_offset += value_len;

            // Search for further occurrences
            let search_from = inx_start + search_len;
            match find_substr(&orig_bytes[search_from..], search_bytes) {
                Some(rel_pos) => {
                    let inx_start2 = search_from + rel_pos;

                    // Copy content between matches, if any
                    if inx_start2 > from {
                        let gap = inx_start2 - from;
                        total_bytes_allocated += gap;
                        tmp = libc::realloc(tmp as *mut libc::c_void, total_bytes_allocated)
                            as *mut u8;
                        if tmp.is_null() {
                            return ptr::null_mut();
                        }
                        ptr::copy_nonoverlapping(
                            orig_bytes.as_ptr().add(from),
                            tmp.add(tmp_offset),
                            gap,
                        );
                        tmp_offset += gap;
                    }

                    inx_start = inx_start2;
                }
                None => {
                    have_match = false;
                }
            }

            from = inx_start + search_len;
        }

        // Copy content after last match, if any
        if from < orig_len && from > 0 {
            total_bytes_allocated += orig_len - from;
            tmp = libc::realloc(tmp as *mut libc::c_void, total_bytes_allocated) as *mut u8;
            if tmp.is_null() {
                return ptr::null_mut();
            }
            ptr::copy_nonoverlapping(
                orig_bytes.as_ptr().add(from),
                tmp.add(tmp_offset),
                orig_len - from,
            );
        }

        *tmp.add(total_bytes_allocated - 1) = 0;

        tmp as *mut c_char
    }
}
