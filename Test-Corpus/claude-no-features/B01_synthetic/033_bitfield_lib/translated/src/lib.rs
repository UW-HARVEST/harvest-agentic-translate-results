// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT
//
// Rust translation of c_src/src/driver.c.
//
// The original C uses bit-field members:
//   typedef struct {
//       unsigned int x : 2;
//       unsigned int y : 3;
//       bool         b : 1;
//       int          z;
//   } foo_t;
//
// Storing an unsigned int value into an N-bit unsigned bit-field truncates
// to the low N bits. Storing a bool into a 1-bit bool bit-field leaves it
// 0 or 1. We mimic the truncation explicitly and call libc printf so the
// produced bytes are byte-identical to the C library's output.

use core::ffi::{c_char, c_int, c_uint};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    // Replicate the bit-field truncation that the C compiler performs at
    // assignment time.
    let fx: c_uint = x & 0x3; // 2 bits
    let fy: c_uint = y & 0x7; // 3 bits
    let fb: c_int = if b { 1 } else { 0 }; // 1-bit bool, printed via %d
    let fz: c_int = z;

    let fmt = b"%u %u %d %d\n\0".as_ptr() as *const c_char;
    unsafe {
        printf(fmt, fx, fy, fb, fz);
    }
}
