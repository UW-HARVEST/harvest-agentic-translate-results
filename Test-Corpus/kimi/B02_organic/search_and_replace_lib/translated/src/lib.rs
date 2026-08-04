use std::ffi::{CStr, CString, c_char};
use std::os::raw::c_char as raw_c_char;

#[unsafe(no_mangle)]
pub extern "C" fn searchAndReplace(orig: *const raw_c_char, search: *const raw_c_char, value: *const raw_c_char) -> *mut raw_c_char {
    let orig_str = unsafe { CStr::from_ptr(orig) }.to_str().unwrap_or("");
    let search_str = unsafe { CStr::from_ptr(search) }.to_str().unwrap_or("");
    let value_str = unsafe { CStr::from_ptr(value) }.to_str().unwrap_or("");

    if search_str.is_empty() {
        let cstring = CString::new(orig_str).unwrap_or_default();
        return cstring.into_raw();
    }

    let mut result = String::new();
    let mut start = 0;

    while let Some(pos) = orig_str[start..].find(search_str) {
        let absolute_pos = start + pos;
        result.push_str(&orig_str[start..absolute_pos]);
        result.push_str(value_str);
        start = absolute_pos + search_str.len();
    }

    result.push_str(&orig_str[start..]);

    match CString::new(result) {
        Ok(cstring) => cstring.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
