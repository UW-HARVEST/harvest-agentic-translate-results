// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT
//
// C-ABI shim for the differential tests.
//
// `c_src/src/main.c` is a single translation unit that defines exactly two
// external symbols: `driver` and `main`. Building that translation unit with
// `-shared` therefore yields a shared object exporting both. This file mirrors
// that surface for the Rust translation so the tests can `dlopen` the C `.so`
// and the Rust `.so` side by side and compare them through the FFI boundary,
// including the `#[no_mangle]` export wrappers themselves.
//
// The bodies delegate to the `driver` library crate, i.e. to exactly the same
// code that backs the `driver` executable.

use std::os::raw::c_int;

/// `void driver(int x, int y)`
#[no_mangle]
pub extern "C" fn driver(x: c_int, y: c_int) {
    driver::driver_impl(x, y);
}

/// `int main()`
///
/// Deliberately does *not* touch the `SIGPIPE` disposition: the C `main` does
/// not either. Restoring the C default is the job of the process entry point
/// (`src/main.rs`), which has to undo the Rust runtime's start-up code.
#[no_mangle]
pub extern "C" fn main() -> c_int {
    driver::c_main()
}
