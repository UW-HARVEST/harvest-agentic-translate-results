use std::ffi::c_int;
use std::io::{self, Write};

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let mut y: c_int = 2i32.wrapping_mul(x);
    y = y.wrapping_add(300);
    // Match printf("%d\n", y) byte-for-byte on stdout.
    let s = format!("{}\n", y);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(s.as_bytes());
    let _ = handle.flush();
}
