// Rust translation of c_src/src/sieve.c (MIT Lincoln Laboratory, 2025).
//
// Public ABI mirrored from c_src/include/sieve.h:
//     void sieve(int start);
//
// Behavior is reproduced exactly as written in the C, including its quirks:
//   * The counter is printed *before* the terminating check, so the value
//     ending in 9 is printed too.
//   * The check is `val % 10 == 9` using C's truncating remainder, which is
//     negative for negative operands. Negative inputs therefore never match
//     and the loop keeps counting up through zero until it reaches 9.
//   * Output goes through C `printf` with the "%d\n" format so stdout
//     buffering, interleaving and byte layout match the C library exactly.

// The crate/library name is `Sieve` so the produced artifact is `libSieve.so`,
// matching the C build's output name.
#![allow(non_snake_case)]

use std::ffi::c_int;

unsafe extern "C" {
    /// C standard library `printf`. Used directly (rather than Rust's
    /// `println!`) so that the shared `stdout` FILE buffer, its flush
    /// semantics and the emitted bytes are identical to the C original.
    fn printf(fmt: *const std::ffi::c_char, ...) -> c_int;
}

/// Format string `"%d\n"` as a NUL-terminated byte string.
const FMT: &[u8; 4] = b"%d\n\0";

/// Count from a starting point, stopping when the count ends in 9 (base 10).
///
/// Direct translation of the C `sieve` function. The parameter is named `val`
/// here to match the C definition (the header declares it as `start`).
#[unsafe(no_mangle)]
pub extern "C" fn sieve(val: c_int) {
    let mut val = val;
    loop {
        // printf("%d\n", val);
        unsafe {
            printf(FMT.as_ptr() as *const std::ffi::c_char, val);
        }

        // if (val % 10 == 9) break;
        // Rust's `%` on integers truncates toward zero exactly like C's,
        // so negative values yield negative remainders here as well.
        if val % 10 == 9 {
            break;
        }

        // val++;
        // Signed overflow is undefined in C; the emitted code wraps in
        // practice, so wrapping arithmetic reproduces the observable
        // behavior without panicking in debug builds.
        val = val.wrapping_add(1);
    }
}
