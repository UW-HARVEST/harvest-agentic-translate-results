// Rust translation of c_src/src/sieve.c
//
// Original copyright 2025 MIT Lincoln Laboratory (MIT license); see c_src for
// the full notice.
//
// Behaviour is reproduced exactly, including the fact that the function is
// named `sieve` but does not actually implement a sieve: it counts upwards
// from `val`, printing each value, and stops once a value ends in 9 (base 10).

// The crate/library is named `Sieve` to match the `libSieve` artifact produced
// by the original CMake build.
#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    // Use the platform's `printf` so that formatting *and* stdio buffering
    // semantics are identical to the C original.
    #[link_name = "printf"]
    safe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// Count from a starting point, stopping when the count ends in 9 (base 10).
///
/// C signature: `void sieve(int val)`
#[unsafe(no_mangle)]
pub extern "C" fn sieve(val: c_int) {
    // `%d\n` as a NUL-terminated C string literal.
    const FMT: &[u8; 4] = b"%d\n\0";

    let mut val = val;
    loop {
        c_printf(FMT.as_ptr() as *const c_char, val);

        // C's `%` truncates toward zero, and so does Rust's, so negative
        // inputs behave identically (e.g. -19 % 10 == -9, which is not 9).
        if val % 10 == 9 {
            break;
        }

        // `val++` in C; wrapping matches the usual compiled behaviour of the
        // original's signed overflow (unreachable in practice, since counting
        // upwards always hits a value ending in 9 first).
        val = val.wrapping_add(1);
    }
}
