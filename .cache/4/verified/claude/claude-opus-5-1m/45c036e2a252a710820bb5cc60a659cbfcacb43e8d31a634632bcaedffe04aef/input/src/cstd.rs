// Declarations of the C standard library routines used by the original
// translation unit. Calling straight through to libc (rather than re-writing
// these in Rust) keeps the observable behaviour byte-identical: `printf` shares
// the very same `stdout` FILE object -- and therefore the same buffering and
// flush-at-exit semantics -- that the C library relied on, and `malloc`'d
// pointers returned across the ABI stay free-able by the caller's `free`.

use core::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    #[allow(clashing_extern_declarations)]
    pub unsafe fn printf(fmt: *const c_char, ...) -> c_int;

    pub unsafe fn malloc(size: usize) -> *mut c_void;
    pub unsafe fn free(ptr: *mut c_void);

    pub unsafe fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    pub unsafe fn strlen(s: *const c_char) -> usize;
    pub unsafe fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
}
