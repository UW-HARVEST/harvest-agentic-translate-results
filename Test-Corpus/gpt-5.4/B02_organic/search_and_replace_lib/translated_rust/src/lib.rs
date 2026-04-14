use libc::malloc;
use std::ffi::{CStr, CString, c_char};
use std::ptr;

#[unsafe(no_mangle)]
pub extern "C" fn searchAndReplace(
    orig: *const c_char,
    search: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    if orig.is_null() || search.is_null() || value.is_null() {
        return ptr::null_mut();
    }

    let orig_str = unsafe { CStr::from_ptr(orig) };
    let search_str = unsafe { CStr::from_ptr(search) };
    let value_str = unsafe { CStr::from_ptr(value) };

    let orig_bytes = orig_str.to_bytes();
    let search_bytes = search_str.to_bytes();
    let value_bytes = value_str.to_bytes();

    if search_bytes.is_empty() {
        let duplicated = match CString::new(orig_bytes) {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };
        return duplicated.into_raw();
    }

    let mut result = Vec::with_capacity(orig_bytes.len().saturating_add(1));
    let mut i = 0;
    let mut found = false;

    while i <= orig_bytes.len().saturating_sub(search_bytes.len()) {
        if &orig_bytes[i..i + search_bytes.len()] == search_bytes {
            result.extend_from_slice(value_bytes);
            i += search_bytes.len();
            found = true;
        } else {
            result.push(orig_bytes[i]);
            i += 1;
        }
    }

    if i < orig_bytes.len() {
        result.extend_from_slice(&orig_bytes[i..]);
    }

    if !found {
        let duplicated = match CString::new(orig_bytes) {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        };
        return duplicated.into_raw();
    }

    result.push(0);

    let len = result.len();
    let ptr_out = unsafe { malloc(len) as *mut c_char };
    if ptr_out.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        ptr::copy_nonoverlapping(result.as_ptr() as *const c_char, ptr_out, len);
    }

    ptr_out
}