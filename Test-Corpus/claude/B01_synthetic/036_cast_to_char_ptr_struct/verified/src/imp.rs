// Translation of c_src/src/main.c to Rust.
//
// Original copyright notice from the C source:
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

//! Shared implementation, compiled into both the `driver` binary and the
//! `libdriver` cdylib. The cdylib re-exports `driver` and `main` with C ABI
//! linkage so that an external caller sees exactly the same symbols the C
//! shared library exports.
//!
//! # Why the I/O goes through libc
//!
//! The C code performs its I/O with `scanf` and `printf`, i.e. on the libc
//! `stdin`/`stdout` `FILE` objects, and those objects carry observable state
//! that no reimplementation on top of `std::io` can reproduce:
//!
//! * `stdin`'s buffer and position are **shared** with anything else in the
//!   process that uses C stdio — including the host that `dlopen`s this library
//!   and calls the exported `main`;
//! * glibc's `exit` runs `_IO_cleanup`, which **seeks the descriptor back** to
//!   the stream's logical position, so a program that reads `"42 rest"` leaves
//!   `" rest"` on the descriptor for the next reader (`{ driver; cat; } < f`).
//!   A `BufReader` swallows its whole 8 KiB read-ahead instead;
//! * `stdout` is fully buffered when redirected, so the 33 bytes are written at
//!   exit, in order with respect to a host's own `printf`s, and are lost if the
//!   process leaves through `_exit`. Rust's `stdout` is a separate,
//!   line-buffered stream;
//! * `scanf`'s pushback (`ungetc` of the terminating or mismatching character)
//!   and the **sticky** end-of-file indicator live in that same `FILE`.
//!
//! Calling the same libc functions the C calls reproduces all of it exactly,
//! rather than approximately.

use std::os::raw::{c_char, c_double, c_int, c_uchar};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn scanf(format: *const c_char, ...) -> c_int;
}

/// ```c
/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
/// ```
///
/// `repr(C)` gives this the same layout the C compiler gives `house_t`, so the
/// object representation `print_hex` walks is the C one by construction rather
/// than by assumption (no hard-coded size, offsets or byte order).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct House {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: c_double,
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
/// `static` in C, so it is deliberately **not** exported (see SYMBOLS.md).
/// Each byte is printed with its own `printf` call, exactly like the C: that
/// keeps the buffering and stdout-locking granularity identical.
///
/// # Safety
///
/// `p` must point to at least `len` readable bytes, and `len` must not be
/// negative — the same contract the C function has.
unsafe fn print_hex(p: *const c_uchar, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // The `unsigned char` is promoted to `int` by the default argument
        // promotions, and `%02x` prints it as an unsigned value.
        printf(
            b"%02x\0".as_ptr() as *const c_char,
            *p.offset(i as isize) as c_int,
        );
        i += 1;
    }
    printf(b"\n\0".as_ptr() as *const c_char);
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
pub fn driver(floors: c_int) {
    // `house_t house = {0};` zeroes the whole object, padding included.
    let mut house: House = unsafe { std::mem::zeroed() };
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.;
    unsafe {
        print_hex(
            &house as *const House as *const c_uchar,
            std::mem::size_of::<House>() as c_int,
        );
    }
}

/// ```c
/// int main() {
///     int x = 0;
///     scanf("%d", &x);
///     driver(x);
///     return 0;
/// }
/// ```
///
/// The `scanf` return value is discarded by the C code, so a matching or input
/// failure simply leaves `x` at its initial value of `0`.
pub fn run_main() -> c_int {
    let mut x: c_int = 0;
    unsafe {
        scanf(b"%d\0".as_ptr() as *const c_char, &mut x as *mut c_int);
    }
    driver(x);
    0
}
