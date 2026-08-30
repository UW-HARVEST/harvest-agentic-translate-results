// Rust translation of the C library in c_src/.
//
// Original copyright 2025 MIT Lincoln Laboratory (MIT license); see c_src for
// the full notice. This file reproduces the behavior and the public ABI of
// that library exactly.
//
// Public surface (from c_src/include/driver.h):
//     void driver(int x);
//
// The C `.so` exports exactly one symbol: `driver`.

#![allow(non_snake_case)]

use std::ffi::c_int;

unsafe extern "C" {
    /// libc `printf`, used so that the emitted bytes and the stdout buffering
    /// behavior are byte-for-byte identical to the C implementation.
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const std::ffi::c_char, ...) -> c_int;
}

/// Translation of:
///
/// ```c
/// void driver(int x) {
///     auto int y = 2*x;
///     y += 300;
///     printf("%d\n", y);
/// }
/// ```
///
/// `auto` is merely a storage-class specifier on a block-scope variable in C,
/// so it has no effect on semantics. Signed arithmetic is performed with
/// wrapping semantics to match what the C compiler emits for `2*x` and
/// `y += 300` on overflow (two's-complement wraparound), rather than panicking.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_int) {
    let y: c_int = 2i32.wrapping_mul(x);
    let y: c_int = y.wrapping_add(300);
    unsafe {
        c_printf(b"%d\n\0".as_ptr() as *const std::ffi::c_char, y);
    }
}
