use std::ffi::c_int;
use std::io::{self, Write};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(b"Hello World!\n");
    0
}
