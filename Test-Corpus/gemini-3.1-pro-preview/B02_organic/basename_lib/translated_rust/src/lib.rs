use std::ffi::CStr;
use std::os::raw::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    if path.is_null() {
        return path;
    }

    let c_str = unsafe { CStr::from_ptr(path) };
    let bytes = c_str.to_bytes();

    let s1 = bytes.iter().rposition(|&b| b == b'/');
    let s2 = bytes.iter().rposition(|&b| b == b'\\');

    let idx = match (s1, s2) {
        (Some(i1), Some(i2)) => std::cmp::max(i1, i2) + 1,
        (Some(i1), None) => i1 + 1,
        (None, Some(i2)) => i2 + 1,
        (None, None) => 0,
    };

    unsafe { path.add(idx) }
}
