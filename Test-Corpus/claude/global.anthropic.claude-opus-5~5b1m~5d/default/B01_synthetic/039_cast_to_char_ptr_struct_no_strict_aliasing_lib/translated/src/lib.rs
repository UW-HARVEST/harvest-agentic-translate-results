// Rust translation of c_src/src/driver.c
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::ffi::{c_char, c_int, c_uchar};

extern "C" {
    // Use the C runtime's printf so that output ordering / buffering matches the
    // original C library exactly when mixed with other libc output.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Mirror of the C `house_t`:
///
/// ```c
/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
/// ```
///
/// On the SysV/x86-64 (and AArch64) ABI this is 16 bytes: `int` at offset 0,
/// `int` at offset 4, `double` at offset 8 (no trailing padding).
#[repr(C)]
#[derive(Copy, Clone)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

impl HouseT {
    /// Equivalent to the C `house_t house = {0};` zero-initialization.
    const fn zeroed() -> Self {
        HouseT {
            floors: 0,
            bedrooms: 0,
            bathrooms: 0.0,
        }
    }
}

/// `static void print_hex(unsigned char *p, int len)`
///
/// Kept internal (the C function is `static`, so it is not part of the ABI).
unsafe fn print_hex(p: *const c_uchar, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        printf(
            b"%02x\0".as_ptr() as *const c_char,
            c_int::from(*p.offset(i as isize)),
        );
        i += 1;
    }
    printf(b"\n\0".as_ptr() as *const c_char);
}

/// `void driver(int floors)`
#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    let mut house = HouseT::zeroed();
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;

    // char raw[sizeof(house)]; memcpy(raw, &house, sizeof(house));
    let mut raw = [0u8; core::mem::size_of::<HouseT>()];
    unsafe {
        core::ptr::copy_nonoverlapping(
            &house as *const HouseT as *const u8,
            raw.as_mut_ptr(),
            core::mem::size_of::<HouseT>(),
        );

        print_hex(raw.as_ptr() as *const c_uchar, raw.len() as c_int);
    }
}
