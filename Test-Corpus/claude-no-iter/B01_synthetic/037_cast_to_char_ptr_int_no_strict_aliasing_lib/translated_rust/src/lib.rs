// Copyright 2025 MIT Lincoln Laboratory
// Rust translation of driver.c

use std::ffi::c_int;
use std::io::Write;

fn print_hex(p: &[u8]) {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    for byte in p {
        let _ = write!(handle, "{:02x}", byte);
    }
    let _ = writeln!(handle);
    let _ = handle.flush();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    // C code: char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    // sizeof(int) on the target ABI; c_int is i32 on all standard platforms.
    let raw: [u8; std::mem::size_of::<c_int>()] = x.to_ne_bytes();
    print_hex(&raw);
}
