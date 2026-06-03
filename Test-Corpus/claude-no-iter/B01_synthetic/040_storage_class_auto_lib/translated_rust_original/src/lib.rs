use std::ffi::c_int;
use std::io::{self, Write};

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y: c_int = x.wrapping_mul(2);
    y = y.wrapping_add(300);
    // Match C printf("%d\n", y) byte-for-byte.
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = write!(handle, "{}\n", y);
    let _ = handle.flush();
}
