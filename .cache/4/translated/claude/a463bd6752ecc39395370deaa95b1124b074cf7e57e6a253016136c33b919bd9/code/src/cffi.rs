//! Raw C runtime bindings used by the translated stb_ds implementation.
//!
//! The original C code allocates through `realloc`/`free` (see the
//! `STBDS_REALLOC`/`STBDS_FREE` macros in `c_src/src/lib.c`), so the Rust
//! translation must use the very same allocator: blocks handed out by this
//! library may be released by callers with `free()` and vice versa.

use core::ffi::{c_char, c_int, c_uint, c_void};

unsafe extern "C" {
    pub fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    /// glibc/musl backend of the `assert()` macro.  `c_src/src/lib.c` is built
    /// without `NDEBUG`, so `STBDS_ASSERT` == `assert` is live and a failing
    /// check prints the canonical diagnostic and calls `abort()`.
    pub fn __assert_fail(
        assertion: *const c_char,
        file: *const c_char,
        line: c_uint,
        function: *const c_char,
    ) -> !;
}

/// `__FILE__` as the C compiler sees it for `c_src/src/lib.c`.
pub const ASSERT_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/src/lib.c\0");

/// Reproduce a failing `assert()` exactly like the C build would.
///
/// `expr` and `func` must be NUL terminated.
pub unsafe fn assert_fail(expr: &str, line: u32, func: &str) -> ! {
    __assert_fail(
        expr.as_ptr() as *const c_char,
        ASSERT_FILE.as_ptr() as *const c_char,
        line as c_uint,
        func.as_ptr() as *const c_char,
    )
}
