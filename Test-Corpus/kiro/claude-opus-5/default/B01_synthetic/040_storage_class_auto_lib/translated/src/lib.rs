// Rust translation of c_src/src/driver.c
//
// Original copyright 2025 MIT Lincoln Laboratory (MIT-style license); see
// c_src/src/driver.c for the full notice.
//
// The C source is:
//
//     void driver(int x) {
//         auto int y = 2*x;
//         y += 300;
//         printf("%d\n", y);
//     }
//
// The `auto` storage-class specifier has no effect on behavior, so the
// translation is a plain local. Arithmetic uses wrapping ops so that the
// 32-bit truncation a C compiler produces for `2*x` / `y += 300` is
// reproduced exactly rather than panicking on overflow.
//
// Output goes through the C library's `printf` on the process-wide `stdout`
// stream, matching the original byte-for-byte (including buffering and
// flush-at-exit semantics, which matter when the caller mixes its own C
// stdio writes with calls into this library).

use std::ffi::c_int;

unsafe extern "C" {
    fn printf(fmt: *const std::ffi::c_char, ...) -> c_int;
}

/// Linker symbol: `driver` (no namespace/renaming macros in include/driver.h).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_int) {
    let y: c_int = x.wrapping_mul(2);
    let y: c_int = y.wrapping_add(300);
    unsafe {
        printf(c"%d\n".as_ptr(), y);
    }
}
