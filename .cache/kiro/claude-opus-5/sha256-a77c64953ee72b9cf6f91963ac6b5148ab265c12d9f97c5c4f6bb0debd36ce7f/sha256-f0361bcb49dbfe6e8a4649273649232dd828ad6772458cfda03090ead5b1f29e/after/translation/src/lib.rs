// Rust translation of c_src/src/driver.c
//
// Original C:
//     void driver(int x) {
//         auto int y = 2*x;
//         y += 300;
//         printf("%d\n", y);
//     }
//
// The `auto` storage-class specifier has no observable effect, so it is not
// modeled. Signed overflow in `2*x` / `y += 300` is undefined behavior in C but
// wraps two's-complement in practice on the reference build, so wrapping
// arithmetic is used here to reproduce the same bytes for every input.
//
// Output is emitted through libc's `printf` (rather than Rust's `println!`) so
// that stdout buffering, ordering and formatting are byte-identical to the C
// library.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// C: `void driver(int x)`
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    // auto int y = 2*x;
    let mut y: c_int = (2 as c_int).wrapping_mul(x);
    // y += 300;
    y = y.wrapping_add(300 as c_int);
    // printf("%d\n", y);
    unsafe {
        printf(b"%d\n\0".as_ptr() as *const c_char, y);
    }
}
