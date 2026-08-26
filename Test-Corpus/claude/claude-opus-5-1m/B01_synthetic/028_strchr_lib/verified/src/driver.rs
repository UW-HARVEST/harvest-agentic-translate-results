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

    /// C `strchr` from libc -- the *same* function `c_src/src/driver.c` calls
    /// (the C `.so` imports `strchr@GLIBC_2.2.5`).
    ///
    /// Calling libc rather than re-implementing the scan matters for exactness,
    /// because `foo` hands `strchr` pointers that C's rules do not permit:
    /// `in == NULL`, and (when `c == '\0'`) pointers walking past the end of the
    /// object. Those inputs must fault exactly the way the C library faults
    /// (`SIGSEGV`). A hand-written Rust loop dereferencing the same pointers
    /// instead trips rustc's debug-assertion "null pointer dereference" check,
    /// which panics; the panic then crosses an `extern "C"` boundary and turns
    /// into `SIGABRT`, so the Rust `.so` died from signal 6 where the C `.so`
    /// died from signal 11. Delegating to libc removes that divergence and also
    /// reproduces glibc's exact page-crossing behaviour byte for byte.
    ///
    /// Note the prototype: C declares the second parameter as `int`, and
    /// `foo`'s `char c` is sign-extended by the usual integer promotions before
    /// the call, which is reproduced by the `c as c_int` at the call site.
    fn strchr(s: *const c_char, c: c_int) -> *const c_char;
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
        // `c` is promoted to `int` (sign-extending, as `char` is signed here)
        // exactly as the C call does.
        s = strchr(s, c as c_int);
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
