//! Raw C runtime bindings used by the translation.
//!
//! The original C code (`c_src/src/lib.c`) allocates with `realloc`/`free`,
//! copies with `memmove`/`memcpy`/`memset`, compares with `memcmp`/`strcmp`
//! and prints with `printf`/`sprintf`.  Interoperating with the very same
//! functions keeps the observable behaviour (allocator, formatting, locale)
//! byte-for-byte identical, so we bind them directly instead of pulling in a
//! crates.io dependency.

use core::ffi::{c_char, c_int, c_uint, c_void};

unsafe extern "C" {
    pub fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);

    pub fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;

    pub fn strlen(s: *const c_char) -> usize;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;

    pub fn printf(fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;

    /// glibc/musl back-end of `assert()`; the C build has no `NDEBUG`, so a
    /// failing `STBDS_ASSERT` aborts through this symbol.
    #[link_name = "__assert_fail"]
    pub fn assert_fail(
        assertion: *const c_char,
        file: *const c_char,
        line: c_uint,
        function: *const c_char,
    ) -> !;
}

/// Reproduces a failing `STBDS_ASSERT` (i.e. `assert`) from `src/lib.c`.
#[cold]
#[inline(never)]
pub fn assert_failed(expr: &'static str, line: u32, func: &'static str) -> ! {
    // `expr`/`func` are built with a trailing NUL by the `stbds_assert!` macro.
    unsafe {
        assert_fail(
            expr.as_ptr() as *const c_char,
            c"src/lib.c".as_ptr(),
            line as c_uint,
            func.as_ptr() as *const c_char,
        )
    }
}

/// `STBDS_ASSERT(cond)` — evaluates `cond` and aborts exactly like the C
/// `assert` macro when it does not hold.
macro_rules! stbds_assert {
    ($cond:expr, $text:literal, $line:expr, $func:literal) => {
        if !$cond {
            $crate::c::assert_failed(concat!($text, "\0"), $line, concat!($func, "\0"));
        }
    };
}
