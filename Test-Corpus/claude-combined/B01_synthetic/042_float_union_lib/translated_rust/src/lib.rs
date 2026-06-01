// Copyright 2025 MIT Lincoln Laboratory
//
// Rust translation of driver.c — produces byte-identical output via libc printf.

use std::os::raw::{c_double, c_ulonglong};

#[link(name = "c")]
unsafe extern "C" {
    fn printf(fmt: *const u8, ...) -> i32;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(f: c_double) {
    // Reinterpret the double's bit pattern as a u64, mirroring the C union.
    let bits: u64 = f.to_bits();
    // Format string matches the original C: "%llx %a %.4f\n"
    let fmt = b"%llx %a %.4f\n\0";
    unsafe {
        printf(fmt.as_ptr(), bits as c_ulonglong, f, f);
    }
}
