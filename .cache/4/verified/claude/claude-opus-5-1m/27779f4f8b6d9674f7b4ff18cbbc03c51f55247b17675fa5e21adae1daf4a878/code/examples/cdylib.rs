//! The C-ABI shared object used by the differential tests.
//!
//! `c_src/src/main.c` exports `print_foo`, `driver` and `main`.  The first two
//! are exported directly by `src/lib.rs`; `main` cannot be, because a
//! `#[no_mangle] fn main` in the library would collide with the entry point
//! rustc generates for the `driver` executable target.  This `cdylib` target
//! therefore adds it — forwarding to exactly the function the executable calls
//! — and re-exports the other two from the library it links.

#[no_mangle]
pub extern "C" fn main() -> std::os::raw::c_int {
    driver::c_main()
}
