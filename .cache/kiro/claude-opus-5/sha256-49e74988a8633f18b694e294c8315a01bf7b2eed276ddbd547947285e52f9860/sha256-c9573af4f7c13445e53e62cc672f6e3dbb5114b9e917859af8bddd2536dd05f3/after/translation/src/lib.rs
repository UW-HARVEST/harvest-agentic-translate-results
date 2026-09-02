// Rust translation of c_src/src/driver.c (+ c_src/include/driver.h).
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

use std::ffi::{c_char, c_double, c_int, c_uchar};

// `#include <stdio.h>`
//
// Output is emitted through the C library's `printf`/`putchar` so that it lands
// in the exact same `stdout` stream (and buffering discipline) as the original C
// implementation. Using Rust's `println!` would write through a separate,
// independently buffered handle and could reorder output relative to any C code
// sharing the process.
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn putchar(c: c_int) -> c_int;
}

/// ```c
/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

/// ```c
/// static void print_hex(unsigned char *p, int len) {
///     for (int i = 0; i < len; i++) {
///         printf("%02x", p[i]);
///     }
///     printf("\n");
/// }
/// ```
///
/// `static` in C, so it is not part of the exported ABI; kept private here.
fn print_hex(p: &[c_uchar], len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // `p[i]` is an `unsigned char`, promoted to `int` by the default
        // argument promotions before being consumed by `%02x`.
        unsafe {
            printf(c"%02x".as_ptr(), c_int::from(p[i as usize]));
        }
        i += 1;
    }
    // gcc lowers `printf("\n")` to `putchar('\n')`; byte-identical either way.
    unsafe {
        putchar(c_int::from(b'\n'));
    }
}

/// ```c
/// void driver(int floors) {
///     house_t house = {0};
///     house.floors = floors;
///     house.bedrooms = 3;
///     house.bathrooms = 2.;
///     print_hex((unsigned char *)&house, sizeof(house));
/// }
/// ```
///
/// Declared in `include/driver.h` as `void driver(int x);` with no namespace
/// macro, so the linker symbol is plain `driver`.
#[unsafe(no_mangle)]
pub extern "C" fn driver(floors: c_int) {
    // `house_t house = {0};` — every byte of the object, including any padding,
    // starts out zeroed.
    let mut house_bytes = [0u8; core::mem::size_of::<HouseT>()];

    // Reproduce the field stores through the `#[repr(C)]` struct so the exact
    // C layout (offsets and padding) is preserved on every target.
    //
    // SAFETY: `house_bytes` is `size_of::<HouseT>()` bytes of zeroed storage.
    // `HouseT` is a `#[repr(C)]` plain-old-data aggregate with no invalid bit
    // patterns, so an all-zero image is a valid value. The array is declared
    // `align(8)`-compatible by going through `write_unaligned`/`read_unaligned`,
    // so no alignment assumption is made about the byte buffer.
    unsafe {
        let p = house_bytes.as_mut_ptr().cast::<HouseT>();
        let mut house: HouseT = p.read_unaligned();
        house.floors = floors;
        house.bedrooms = 3;
        house.bathrooms = 2.;
        p.write_unaligned(house);
    }

    // `sizeof(house)` is a `size_t`, narrowed to the `int len` parameter.
    print_hex(&house_bytes, core::mem::size_of::<HouseT>() as c_int);
}
