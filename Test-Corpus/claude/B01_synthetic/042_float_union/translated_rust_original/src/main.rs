// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Produces byte-identical output to the C program.
//
// The original C program reads a double via scanf("%lf"), then prints:
//   "%llx %a %.4f\n" with the raw bits, the hex-float, and the decimal form.
//
// To guarantee byte-identical output (especially for the platform-specific
// "%a" hex-float form, the "%llx" formatting, and "%.4f" rounding behavior),
// we delegate scanf/printf to libc via a tiny FFI surface.

use std::os::raw::{c_char, c_double, c_int};

#[link(name = "c")]
unsafe extern "C" {
    fn scanf(fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

fn driver(f: f64) {
    // Equivalent to the C union { uint64_t x; double f; } trick.
    let x: u64 = f.to_bits();
    // The original C format string is exactly: "%llx %a %.4f\n"
    let fmt = b"%llx %a %.4f\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, x as u64, f as c_double, f as c_double);
    }
}

fn main() {
    let mut f: f64 = 0.0;
    let fmt = b"%lf\0".as_ptr() as *const c_char;
    unsafe {
        scanf(fmt, &mut f as *mut f64);
    }
    driver(f);
}
