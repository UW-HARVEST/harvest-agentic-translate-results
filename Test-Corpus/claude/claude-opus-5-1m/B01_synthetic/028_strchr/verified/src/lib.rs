// cdylib surface of the translation: the `extern "C"` / `#[no_mangle]` exports
// that mirror, one-for-one, the symbols exported by the C shared library built
// from c_src/src/main.c (`foo`, `driver`, `main`).
//
// The actual logic lives in core_impl.rs, which is shared verbatim with the
// `driver` executable (src/main.rs).

#[path = "core_impl.rs"]
mod core_impl;

use std::os::raw::{c_char, c_int};

/// `int foo(const char *in, char c)`
///
/// # Safety
/// `in` must be a valid NUL-terminated string, exactly as required by the C
/// original (which calls `strchr` on it).
#[no_mangle]
pub unsafe extern "C" fn foo(input: *const c_char, c: c_char) -> c_int {
    core_impl::foo_impl(input, c)
}

/// `void driver(const char *in)`
///
/// # Safety
/// `in` must be a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn driver(input: *const c_char) {
    core_impl::driver_impl(input)
}

/// `int main()`
///
/// Exported so the Rust shared object presents the same symbol set as the C
/// shared object. Reads up to 1000 bytes from stdin and runs `driver` on them.
#[no_mangle]
pub extern "C" fn main() -> c_int {
    core_impl::main_impl()
}
