//! C-ABI surface of the translated `container_of` program.
//!
//! Built as a `cdylib`, this crate produces `libdriver.so`, which exports the
//! exact same global symbols as the C shared library built from
//! `c_src/src/container_of.c`:
//!
//! | symbol                 | C declaration                                |
//! |------------------------|----------------------------------------------|
//! | `find_container_of_a`  | `struct test *find_container_of_a(int *i);`   |
//! | `find_container_of_b`  | `struct test *find_container_of_b(int *i);`   |
//! | `main`                 | `int main(int argc, char **argv);`            |
//!
//! The implementation lives in [`container_of`]; this file is only the
//! `#[no_mangle] extern "C"` export layer.

pub mod container_of;

use core::ffi::{c_char, c_int};

pub use container_of::Test;

/// `struct test* find_container_of_a(int *i)`
///
/// # Safety
///
/// Mirrors the C function, which performs no validation whatsoever: the
/// argument is treated purely as an address and no memory is dereferenced.
#[no_mangle]
pub unsafe extern "C" fn find_container_of_a(i: *mut c_int) -> *mut Test {
    container_of::find_container_of_a(i)
}

/// `struct test* find_container_of_b(int *i)`
///
/// # Safety
///
/// Mirrors the C function, which performs no validation whatsoever: the
/// argument is treated purely as an address and no memory is dereferenced.
#[no_mangle]
pub unsafe extern "C" fn find_container_of_b(i: *mut c_int) -> *mut Test {
    container_of::find_container_of_b(i)
}

/// `int main(int argc, char** argv)`
///
/// Exported under its C name so that the Rust shared object has the same public
/// symbol set as the C one, and so that the whole program body can be driven
/// through the FFI boundary by the differential tests.
///
/// # Safety
///
/// `argv` must be a NULL-terminated array of NUL-terminated strings, as a C
/// runtime provides. Just like the C original, `argc` is ignored and `argv[1]`
/// and `argv[2]` are read unconditionally.
#[no_mangle]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    container_of::c_main(argc, argv)
}
