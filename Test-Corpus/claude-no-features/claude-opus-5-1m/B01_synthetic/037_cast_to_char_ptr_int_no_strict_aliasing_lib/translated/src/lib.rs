use std::ffi::c_int;
use std::io::{self, Write};

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let mut s = String::with_capacity(p.len() * 2 + 1);
    for &b in p {
        s.push_str(&format!("{:02x}", b));
    }
    s.push('\n');
    let _ = handle.write_all(s.as_bytes());
    let _ = handle.flush();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let raw: [u8; std::mem::size_of::<c_int>()] = x.to_ne_bytes();
    print_hex(&raw);
}
