// Translation of c_src/src/driver.c (and its public header c_src/include/driver.h).
//
// Both `foo` and `driver` have external linkage in the C translation unit, so
// both appear in the dynamic symbol table of the C shared library and both are
// re-exported here. There are no namespace/renaming macros in the C headers, so
// the linker names are exactly `foo` and `driver`.

use core::ffi::{c_char, c_int};

extern "C" {
    /// C `printf` from libc: used (instead of Rust's own stdio) so that the
    /// emitted bytes *and* the stdout buffering/flushing behaviour are
    /// identical to the C library's.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Faithful re-implementation of C's `strchr`.
///
/// Returns a pointer to the first byte of `s` equal to `c`, or NULL if no such
/// byte occurs before (and including) the terminating NUL. Note that, exactly
/// like the C library function, searching for `c == 0` succeeds and yields a
/// pointer to the terminating NUL byte.
unsafe fn strchr(s: *const c_char, c: c_char) -> *const c_char {
    let mut p = s;
    loop {
        let ch = *p;
        if ch == c {
            return p;
        }
        if ch == 0 {
            return core::ptr::null();
        }
        // `wrapping_add` keeps the pointer walk free of Rust-level provenance
        // assumptions, so the (possibly out-of-bounds) scanning behaviour of
        // the original C code is preserved rather than optimized away.
        p = p.wrapping_add(1);
    }
}

/// C:
/// ```c
/// int foo(const char *in, char c) {
///     int res = 0;
///     for (const char *s = in; s = strchr(s, c); s++) {
///         res++;
///     }
///     return res;
/// }
/// ```
///
/// Counts the occurrences of `c` in the NUL-terminated string `in`. The quirks
/// of the original are preserved verbatim: `in == NULL` dereferences NULL, and
/// `c == '\0'` matches the terminator and then keeps scanning past the end of
/// the string (the C code's behaviour, bugs included).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foo(in_: *const c_char, c: c_char) -> c_int {
    let mut res: c_int = 0;
    let mut s: *const c_char = in_;
    loop {
        // `s = strchr(s, c)` -- loop condition of the C `for` statement.
        s = strchr(s, c);
        if s.is_null() {
            break;
        }
        // Loop body: `res++`.
        res = res.wrapping_add(1);
        // Iteration expression: `s++`.
        s = s.wrapping_add(1);
    }
    res
}

/// C:
/// ```c
/// void driver(const char *in) {
///     printf("A: %d\n", foo(in, 'A'));
///     printf("x: %d\n", foo(in, 'x'));
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_: *const c_char) {
    printf(
        b"A: %d\n\0".as_ptr() as *const c_char,
        foo(in_, b'A' as c_char),
    );
    printf(
        b"x: %d\n\0".as_ptr() as *const c_char,
        foo(in_, b'x' as c_char),
    );
}
