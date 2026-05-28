// Translated from c_src/src/driver.c

use std::ffi::c_int;
use std::io::{self, Write};

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut buf = String::with_capacity(p.len() * 2 + 1);
    for &b in p {
        // Match printf("%02x", ...) formatting
        buf.push_str(&format!("{:02x}", b));
    }
    buf.push('\n');
    let _ = out.write_all(buf.as_bytes());
    let _ = out.flush();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}
