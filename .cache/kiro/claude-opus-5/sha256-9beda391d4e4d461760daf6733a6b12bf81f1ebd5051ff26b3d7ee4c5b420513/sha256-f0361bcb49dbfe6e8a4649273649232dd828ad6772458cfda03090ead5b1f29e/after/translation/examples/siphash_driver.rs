//! Driver used by the differential tests to capture `siphash()`'s stdout in
//! isolation.
//!
//! Usage: `siphash_driver <path-to-.so> <init>`
//!
//! It `dlopen`s the given shared object (either the C one or the Rust one),
//! looks up the exported `siphash` symbol, and calls it. Because this runs in
//! its own process, the captured stdout contains *only* what the library
//! printed — no test-harness output can interleave.
//!
//! This also serves as the "driver binary" whose stdout is compared between the
//! C and Rust implementations.

use libloading::{Library, Symbol};
use std::os::raw::c_int;

fn main() {
    let mut args = std::env::args().skip(1);
    let so = args.next().expect("usage: siphash_driver <so> <init>");
    let init: i32 = args
        .next()
        .expect("usage: siphash_driver <so> <init>")
        .parse()
        .expect("init must be an i32");

    unsafe {
        let lib = Library::new(&so).unwrap_or_else(|e| panic!("dlopen {so}: {e}"));
        let f: Symbol<unsafe extern "C" fn(c_int)> =
            lib.get(b"siphash\0").expect("missing exported symbol `siphash`");
        f(init as c_int);
    }
}
