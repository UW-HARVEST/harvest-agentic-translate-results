//! Rust translation of the C `driver` package (`hashmap.c`, `tree.c`, `main.c`).
//!
//! The crate is built as a `cdylib` so that it exports exactly the same C ABI
//! symbols as the original shared library.

#[macro_use]
pub mod cio;
pub mod driver;
pub mod hashmap;
pub mod tree;

use core::ffi::c_int;

/// `int main(void)` from `c_src/src/main.c`.
///
/// Exported so the shared object's symbol table matches the C one.  Just like
/// the C function it returns 0 and leaves flushing of `stdout` to `exit()`.
#[no_mangle]
pub unsafe extern "C" fn main() -> c_int {
    driver::run_main()
}
