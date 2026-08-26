use std::ffi::c_int;
use std::io::{self, Write};

#[no_mangle]
pub extern "C" fn main() -> c_int {
    let _ = io::stdout().write_all(b"Hello World!\n");
    0
}
