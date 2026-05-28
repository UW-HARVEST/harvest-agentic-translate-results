use std::ffi::c_float;
use std::io::{self, Write};

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for &b in p {
        // C printf("%02x", ...) — lowercase hex, zero-padded width 2
        let _ = write!(out, "{:02x}", b);
    }
    let _ = writeln!(out);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_float) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}
