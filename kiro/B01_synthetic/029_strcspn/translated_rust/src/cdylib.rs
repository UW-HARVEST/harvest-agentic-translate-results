// This file is compiled as a cdylib to produce the shared library.
// It re-exports everything from lib.rs and adds the `main` symbol.

pub use driver::*;

#[no_mangle]
pub extern "C" fn main() -> std::os::raw::c_int {
    driver::run_main()
}
