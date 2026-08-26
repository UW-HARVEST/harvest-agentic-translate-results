// Copyright 2025 MIT Lincoln Laboratory
// (see src/lib_impl.rs for the full license text)
//
// Executable face of the translation.  The translated code is included by path
// rather than through the library target so that the library can export a
// C-ABI `main` symbol (matching the C shared object) without clashing with the
// Rust `fn main` below.

#[path = "lib_impl.rs"]
mod lib_impl;

use core::ffi::{c_char, c_int};
use core::ptr::null_mut;

fn main() {
    // Reconstruct C's `argc`/`argv`.  The translated `main` ignores both, but
    // they are passed through so the two entry points stay identical.
    let args: Vec<std::ffi::CString> = std::env::args_os()
        .map(|a| {
            use std::os::unix::ffi::OsStrExt;
            std::ffi::CString::new(a.as_os_str().as_bytes()).unwrap_or_default()
        })
        .collect();
    let mut argv: Vec<*mut c_char> = args
        .iter()
        .map(|a| a.as_ptr() as *mut c_char)
        .collect();
    argv.push(null_mut());
    let argc = args.len() as c_int;

    let rc = unsafe { lib_impl::c_main(argc, argv.as_mut_ptr()) };
    std::process::exit(rc);
}
