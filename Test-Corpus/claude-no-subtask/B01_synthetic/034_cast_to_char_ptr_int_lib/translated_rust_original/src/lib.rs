use std::ffi::c_int;
use std::io::{self, Write};

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    for &b in p {
        let _ = write!(handle, "{:02x}", b);
    }
    let _ = writeln!(handle);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}
