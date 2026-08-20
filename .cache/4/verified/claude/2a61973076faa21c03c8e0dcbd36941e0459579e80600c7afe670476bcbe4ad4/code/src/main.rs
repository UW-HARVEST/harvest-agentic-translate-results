// Entry point shim for the `driver` executable.
//
// The whole translation (including `main` itself) lives in src/lib.rs so that a
// single implementation can be built both as a `cdylib` -- exporting exactly the
// same symbols as the C shared object -- and as the program's `rlib`.
//
// `#![no_main]` suppresses rustc's own C `main`, so the process entry point is
// the library's `#[no_mangle] pub unsafe extern "C" fn main(argc, argv)`, i.e.
// the direct translation of c_src/src/main.c.  This also means the process exit
// status and the libc stdio flush-at-exit behaviour match the C program exactly.
#![no_main]

use core::ffi::{c_char, c_int};

/// Force the object file that defines the `main` symbol to be linked in.
#[used]
static ENTRY: unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int = driver::main;
