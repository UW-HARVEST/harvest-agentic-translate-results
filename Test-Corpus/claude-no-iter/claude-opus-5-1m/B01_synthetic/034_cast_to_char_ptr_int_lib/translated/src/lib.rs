// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_int;

extern "C" {
    fn printf(format: *const u8, ...) -> c_int;
}

fn print_hex(p: *const u8, len: c_int) {
    unsafe {
        for i in 0..len {
            printf(b"%02x\0".as_ptr(), *p.offset(i as isize) as c_int);
        }
        printf(b"\n\0".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let p: *const u8 = &x as *const c_int as *const u8;
    print_hex(p, std::mem::size_of::<c_int>() as c_int);
}
