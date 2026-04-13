use std::os::raw::{c_int, c_char};
use std::ffi::CString;

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let quot = x / y;
    let rem = x % y;
    let msg = format!("quotient: {}, remainder: {}\n", quot, rem);
    let c_msg = CString::new(msg).unwrap();
    unsafe {
        libc::printf(c_msg.as_ptr() as *const c_char);
    }
}
