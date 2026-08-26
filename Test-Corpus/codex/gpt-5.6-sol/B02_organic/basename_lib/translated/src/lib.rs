use std::ffi::c_char;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tool_basename(path: *mut c_char) -> *mut c_char {
    let mut cursor = path;
    let mut basename = path;

    unsafe {
        while *cursor != 0 {
            if *cursor == b'/' as c_char || *cursor == b'\\' as c_char {
                basename = cursor.add(1);
            }
            cursor = cursor.add(1);
        }
    }

    basename
}
