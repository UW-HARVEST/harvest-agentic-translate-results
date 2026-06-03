// Translated from C sources in c_src/
// Original Copyright 2025 MIT Lincoln Laboratory

pub mod matrix;
pub mod write;
pub mod driver;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

/// C-compatible export of the driver function. Mirrors the signature of
/// `int driver(int, int, const char*, int, int, const char*)` from driver.c.
///
/// # Safety
/// The caller must ensure that `matrix_a` and `matrix_b` point to valid,
/// NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn driver(
    width_a: c_int,
    height_a: c_int,
    matrix_a: *const c_char,
    width_b: c_int,
    height_b: c_int,
    matrix_b: *const c_char,
) -> c_int {
    if matrix_a.is_null() || matrix_b.is_null() {
        return driver::EXIT_FAILURE;
    }
    let a = match CStr::from_ptr(matrix_a).to_str() {
        Ok(s) => s,
        Err(_) => return driver::EXIT_FAILURE,
    };
    let b = match CStr::from_ptr(matrix_b).to_str() {
        Ok(s) => s,
        Err(_) => return driver::EXIT_FAILURE,
    };
    driver::driver(width_a, height_a, a, width_b, height_b, b)
}
