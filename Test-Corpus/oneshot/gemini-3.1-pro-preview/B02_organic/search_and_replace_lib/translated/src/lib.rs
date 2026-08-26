use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|window| window == needle)
}

#[unsafe(no_mangle)]
pub extern "C" fn searchAndReplace(
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    if orig.is_null() || search.is_null() || value.is_null() {
        return ptr::null_mut();
    }

    let orig_bytes = unsafe { CStr::from_ptr(orig) }.to_bytes();
    let search_bytes = unsafe { CStr::from_ptr(search) }.to_bytes();
    let value_bytes = unsafe { CStr::from_ptr(value) }.to_bytes();

    if search_bytes.is_empty() {
        let len = orig_bytes.len() + 1;
        let ptr = unsafe { malloc(len) } as *mut c_char;
        if !ptr.is_null() {
            unsafe {
                ptr::copy_nonoverlapping(orig, ptr, len);
            }
        }
        return ptr;
    }

    let mut result = Vec::new();
    let mut current = orig_bytes;

    while let Some(pos) = find_subsequence(current, search_bytes) {
        result.extend_from_slice(&current[..pos]);
        result.extend_from_slice(value_bytes);
        current = &current[pos + search_bytes.len()..];
    }
    result.extend_from_slice(current);
    result.push(0);

    let ptr = unsafe { malloc(result.len()) } as *mut c_char;
    if !ptr.is_null() {
        unsafe {
            ptr::copy_nonoverlapping(result.as_ptr() as *const c_char, ptr, result.len());
        }
    }
    ptr
}
