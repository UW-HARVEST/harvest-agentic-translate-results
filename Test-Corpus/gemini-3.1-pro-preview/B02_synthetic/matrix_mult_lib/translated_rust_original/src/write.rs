use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::fs::File;
use std::io::Write;

#[unsafe(no_mangle)]
pub extern "C" fn write_to_file(filename: *const c_char, contents: *const c_char) -> c_int {
    if filename.is_null() || contents.is_null() {
        return 22;
    }
    let fname = unsafe { CStr::from_ptr(filename) }.to_string_lossy();
    let cont = unsafe { CStr::from_ptr(contents) }.to_bytes();

    match File::create(fname.as_ref()) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(cont) {
                return e.raw_os_error().unwrap_or(5);
            }
            0
        }
        Err(e) => e.raw_os_error().unwrap_or(5),
    }
}
