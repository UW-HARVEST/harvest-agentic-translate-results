// Rust translation of the MIT Lincoln Laboratory `driver` C library.
//
// Original copyright notice from c_src (reproduced for attribution):
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// The complete public ABI of the C shared library, as reported by
// `nm -D libdriver.so` (defined, global symbols), is:
//
//     T driver
//     T foo
//
// `foo` is not declared in include/driver.h, but it has external linkage in
// src/driver.c and is therefore part of the exported ABI; it is reproduced here.

use core::ffi::{c_char, c_int};

unsafe extern "C" {
    /// The C library formats its output with `printf` from libc. We reuse it
    /// verbatim so that the produced bytes *and* the stdio stream buffering
    /// behaviour are identical to the original library.
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// Faithful reimplementation of C `strchr`.
///
/// Semantics preserved from the C standard / glibc:
/// * the comparison is performed on `char`-sized values,
/// * the terminating NUL is part of the searched string, so `strchr(s, 0)`
///   returns a pointer to that terminator rather than NULL,
/// * a NULL / invalid `s` is dereferenced, exactly as the C code would.
///
/// # Safety
/// `s` must point to a NUL-terminated string (or, for bug-compatibility with
/// the original, to whatever the caller passed).
#[inline]
unsafe fn strchr(mut s: *const c_char, c: c_char) -> *const c_char {
    loop {
        let v = unsafe { *s };
        if v == c {
            return s;
        }
        if v == 0 {
            return core::ptr::null();
        }
        s = unsafe { s.add(1) };
    }
}

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
/// Note: the original increments `s` past every match, including the case where
/// `c == '\0'` and the match is the terminator (walking off the end of the
/// buffer). That behaviour is intentionally reproduced rather than "fixed".
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foo(in_: *const c_char, c: c_char) -> c_int {
    let mut res: c_int = 0;
    let mut s: *const c_char = in_;
    loop {
        s = unsafe { strchr(s, c) };
        if s.is_null() {
            break;
        }
        res = res.wrapping_add(1);
        s = unsafe { s.add(1) };
    }
    res
}

/// ```c
/// void driver(const char *in) {
///     printf("A: %d\n", foo(in, 'A'));
///     printf("x: %d\n", foo(in, 'x'));
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(in_: *const c_char) {
    unsafe {
        c_printf(c"A: %d\n".as_ptr(), foo(in_, b'A' as c_char));
        c_printf(c"x: %d\n".as_ptr(), foo(in_, b'x' as c_char));
    }
}
