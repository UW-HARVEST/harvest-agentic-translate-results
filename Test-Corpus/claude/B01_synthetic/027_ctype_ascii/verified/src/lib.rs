//! Shared-library surface of the translation of `c_src/src/main.c`.
//!
//! The C translation unit defines exactly two external symbols, `driver` and
//! `main`; both are re-exported here with the C ABI and identical names so an
//! external caller (or `dlsym`) sees the same surface as the C shared object.

pub mod ctype;
mod tables;

use std::os::raw::c_char;

/// `void driver(char c)`
#[no_mangle]
pub extern "C" fn driver(c: c_char) {
    ctype::driver(c as i8);
}

/// `int main()`
///
/// Hidden from the `cargo test` unit-test harness, whose generated entry point
/// also provides a `main` symbol.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> std::os::raw::c_int {
    ctype::c_main() as std::os::raw::c_int
}
