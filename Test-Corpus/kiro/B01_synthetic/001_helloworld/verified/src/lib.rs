#![no_main]

use std::io::Write;

#[no_mangle]
pub extern "C" fn main() -> i32 {
    let _ = std::io::stdout().write_all(b"Hello World!\n");
    let _ = std::io::stdout().flush();
    0
}
