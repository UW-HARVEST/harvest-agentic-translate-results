//! Translation of `c_src/libsodium/sodium/version.c`.

use core::ffi::{c_char, c_int};

const SODIUM_VERSION_STRING: &[u8] = b"1.0.23\0";
const SODIUM_LIBRARY_VERSION_MAJOR: c_int = 30;
const SODIUM_LIBRARY_VERSION_MINOR: c_int = 0;

#[no_mangle]
pub unsafe extern "C" fn sodium_version_string() -> *const c_char {
    SODIUM_VERSION_STRING.as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn sodium_library_version_major() -> c_int {
    SODIUM_LIBRARY_VERSION_MAJOR
}

#[no_mangle]
pub unsafe extern "C" fn sodium_library_version_minor() -> c_int {
    SODIUM_LIBRARY_VERSION_MINOR
}

#[no_mangle]
pub unsafe extern "C" fn sodium_library_minimal() -> c_int {
    // `SODIUM_LIBRARY_MINIMAL` is not defined in this build.
    0
}
