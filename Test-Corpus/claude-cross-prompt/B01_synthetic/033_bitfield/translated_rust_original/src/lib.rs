// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT
//
// Rust translation of c_src/src/main.c.
// The C struct uses bitfields:
//   typedef struct {
//       unsigned int x : 2;
//       unsigned int y : 3;
//       bool b : 1;
//       int z;
//   } foo_t;
//
// On the targeted ABI (System V x86-64 / typical little-endian) these bitfields
// pack into the low byte as: bit0..1 = x, bit2..4 = y, bit5 = b, bit6..7 = pad.
// Total sizeof(foo_t) = 8, offsetof(z) = 4.

use std::ffi::{c_int, c_uint};

unsafe extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
    fn scanf(fmt: *const u8, ...) -> c_int;
}

#[repr(C)]
pub struct foo_t {
    /// Holds the packed `x : 2 | y : 3 | b : 1 | pad : 2` bitfield byte.
    bits: u8,
    _pad: [u8; 3],
    z: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_foo(foo: *const foo_t) {
    let foo_ref = unsafe { &*foo };
    let x: c_uint = (foo_ref.bits & 0x3) as c_uint;
    let y: c_uint = ((foo_ref.bits >> 2) & 0x7) as c_uint;
    let b: c_int = ((foo_ref.bits >> 5) & 0x1) as c_int;
    let z: c_int = foo_ref.z;
    unsafe {
        printf(b"%u %u %d %d\n\0".as_ptr(), x, y, b, z);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_uint, y: c_uint, b: bool, z: c_int) {
    // Truncate to bitfield widths just like the C compiler would when assigning
    // through the bitfield members.
    let bits: u8 = ((x & 0x3) as u8)
        | (((y & 0x7) as u8) << 2)
        | (((b as u8) & 0x1) << 5);
    let foo = foo_t {
        bits,
        _pad: [0; 3],
        z,
    };
    unsafe { print_foo(&foo) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main() -> c_int {
    let mut x: c_uint = 0;
    let mut y: c_uint = 0;
    let mut b: c_int = 0;
    let mut z: c_int = 0;
    unsafe {
        scanf(b"%u\0".as_ptr(), &mut x as *mut c_uint);
        scanf(b"%u\0".as_ptr(), &mut y as *mut c_uint);
        scanf(b"%d\0".as_ptr(), &mut b as *mut c_int);
        scanf(b"%d\0".as_ptr(), &mut z as *mut c_int);
        driver(x, y, b != 0, z);
    }
    0
}
