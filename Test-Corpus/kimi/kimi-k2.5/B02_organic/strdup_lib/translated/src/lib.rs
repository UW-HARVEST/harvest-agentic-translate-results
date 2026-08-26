use std::ffi::{CStr, c_char};
use std::os::raw::c_char as raw_c_char;
use std::alloc::{alloc, Layout};
use std::ptr::copy_nonoverlapping;

#[unsafe(no_mangle)]
pub extern "C" fn custom_strdup(str: *const raw_c_char) -> *mut raw_c_char {
    if str.is_null() {
        return std::ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(str) };
    let bytes = c_str.to_bytes_with_nul();
    let len = bytes.len();

    let layout = match Layout::array::<u8>(len) {
        Ok(l) => l,
        Err(_) => return std::ptr::null_mut(),
    };

    let new_ptr = unsafe { alloc(layout) };
    if new_ptr.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        copy_nonoverlapping(bytes.as_ptr(), new_ptr, len);
    }

    new_ptr as *mut raw_c_char
}