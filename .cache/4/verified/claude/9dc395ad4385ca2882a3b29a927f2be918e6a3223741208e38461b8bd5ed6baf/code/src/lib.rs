//! cdylib surface of the translation.
//!
//! The C source, when compiled as a shared library, exports exactly two
//! symbols (`print_hex` is `static`, so it stays internal):
//!
//! ```text
//! T driver
//! T main
//! ```
//!
//! This crate re-exports the same two symbols with C ABI linkage and the same
//! signatures so that a caller loading either `.so` with `dlopen`/`dlsym`
//! observes identical behaviour.
//!
//! Note that no process-wide state is touched here — in particular the
//! `SIGPIPE` disposition is left alone, exactly as the C shared library leaves
//! it. That adjustment belongs to the *program* (`src/main.rs`), which has to
//! reproduce a C program's startup state.

use std::os::raw::c_int;

pub mod imp;

/// `void driver(int floors)`
#[no_mangle]
pub extern "C" fn driver(floors: c_int) {
    imp::driver(floors);
}

/// `int main()`
///
/// The C shared library exports its `main` just like any other non-static
/// function, so the translation exports it too. Calling it reads one `%d`
/// conversion from libc's `stdin` and prints the struct bytes, exactly as the C
/// `main` does — including sharing the stream state with the calling host.
#[no_mangle]
pub extern "C" fn main() -> c_int {
    imp::run_main()
}
