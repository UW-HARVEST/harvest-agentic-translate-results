//! Shared-object view of the translation.
//!
//! `c_src/CMakeLists.txt` builds `src/main.c` as an executable, so the only
//! symbol the C translation unit defines is `main`.  Compiling that same C file
//! as a shared object (`gcc -shared -fPIC src/main.c`) therefore exports
//! exactly one non-libc dynamic symbol: `main`.
//!
//! This crate is built as a `cdylib` exporting a `main` with the same C ABI, so
//! the Rust `.so` presents the same dynamic symbol surface as the C `.so` and
//! the two can be loaded side by side and compared through the FFI boundary
//! (see `tests/differential_so.rs`).

#[path = "echo.rs"]
mod echo;

/// `int main()` from `c_src/src/main.c`.
///
/// The C function reads `stdin` and writes `stdout`, i.e. file descriptors 0
/// and 1 of the calling process, and always returns 0.  One difference is
/// unavoidable when hosting `main` inside a shared object: the C version
/// relies on the C runtime to flush `stdout` after `main` returns, so a caller
/// that dlopen()s the C `.so` has to call `fflush(NULL)` itself.  This version
/// performs that final flush before returning, so callers should flush the C
/// side to compare like with like.
#[no_mangle]
pub extern "C" fn main() -> std::ffi::c_int {
    echo::run() as std::ffi::c_int
}
