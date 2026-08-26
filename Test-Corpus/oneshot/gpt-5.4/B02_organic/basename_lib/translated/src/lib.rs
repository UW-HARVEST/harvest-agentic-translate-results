use std::ffi::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    if path.is_null() {
        return path;
    }

    let mut current = path;
    let mut p = path;

    loop {
        let ch = unsafe { *p };
        if ch == 0 {
            break;
        }
        if ch == b'/' as c_char || ch == b'\\' as c_char {
            current = unsafe { p.add(1) };
        }
        p = unsafe { p.add(1) };
    }

    current
}
