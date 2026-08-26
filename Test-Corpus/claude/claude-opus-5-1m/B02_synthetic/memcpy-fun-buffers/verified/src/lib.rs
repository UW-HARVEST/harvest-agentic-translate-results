// Copyright 2025 MIT Lincoln Laboratory
// (see src/lib_impl.rs for the full license text)
//
// Shared-library face of the translation.  All of the translated code lives in
// `lib_impl`; this file only adds the C `main` entry point so that the produced
// `libdriver.so` exports exactly the same symbol set as a `libdriver_c.so`
// built from `c_src/src/main.c`.
//
// `main` deliberately lives here and not in `lib_impl` so that the `driver`
// executable (which pulls `lib_impl` in via `#[path]`) can keep using a normal
// Rust `fn main` without a duplicate-symbol clash.

mod lib_impl;

pub use lib_impl::*;

#[cfg(not(test))]
use core::ffi::{c_char, c_int};

/// `int main(int argc, char *argv[])`
///
/// Suppressed under `cfg(test)`: libtest generates its own entry point for the
/// library's unit-test harness, which would clash with this symbol.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    lib_impl::c_main(argc, argv)
}
