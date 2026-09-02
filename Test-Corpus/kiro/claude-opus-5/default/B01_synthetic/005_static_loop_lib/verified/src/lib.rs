// Rust translation of c_src/src/staticloop.c
//
// Original C library: StaticLoop (Copyright 2025 MIT Lincoln Laboratory, MIT license).
// This file reproduces the exact public ABI and observable behavior of the C
// shared library, including the function-local `static` accumulator in
// `static_sum` and the stdout formatting/buffering performed by libc `printf`.

// The crate/library name matches the C target name (`libStaticLoop.so`).
#![allow(non_snake_case)]

use std::ffi::c_int;

unsafe extern "C" {
    /// libc `printf`. Called directly so that stdout formatting *and* buffering
    /// (and therefore interleaving with any other C stdio output in the process)
    /// are byte-for-byte identical to the original C library.
    fn printf(fmt: *const std::ffi::c_char, ...) -> c_int;
}

/// Translation of the function-local `static int sum = 0;` inside `static_sum`.
///
/// C semantics: a single process-wide instance with static storage duration,
/// zero-initialized, mutated without synchronization. Reproduced faithfully
/// (i.e. it is likewise not thread-safe) rather than "fixed".
static mut SUM: c_int = 0;

/// ```c
/// int
/// static_sum(int update) {
///   static int sum = 0;
///   sum += update;
///   return sum;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_sum(update: c_int) -> c_int {
    // `wrapping_add` matches the two's-complement wraparound emitted by the
    // reference C compilers for `sum += update` (signed overflow is UB in C,
    // but the generated code wraps).
    let sum = unsafe { SUM.wrapping_add(update) };
    unsafe { SUM = sum };
    sum
}

/// Maintain a running total using a static variable.
///
/// ```c
/// void
/// driver(int stride) {
///   for (int i = 0; i < 10; i++) {
///     printf("%d\n", static_sum(i * stride));
///   }
///   return;
/// }
/// ```
///
/// Note: the public header declares this as `void driver(int update);` while the
/// definition names the parameter `stride`. Only the name differs, so the ABI is
/// unaffected.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(stride: c_int) {
    for i in 0..10 as c_int {
        // `wrapping_mul` mirrors the C `i * stride` multiplication.
        let value = unsafe { static_sum(i.wrapping_mul(stride)) };
        unsafe { printf(c"%d\n".as_ptr(), value) };
    }
}
