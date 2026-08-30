// Rust translation of c_src/src/driver.c
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

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    /// Use the platform's C `printf` so that output bytes, formatting and stdio
    /// buffering behaviour are identical to the original C library.
    #[link_name = "printf"]
    safe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// Equivalent of `strchr(s, c)`: returns a pointer to the first byte in the
/// NUL-terminated string `s` equal to `c`, or null if there is none. As in C,
/// the terminating NUL is part of the searched string, so `c == 0` matches it.
unsafe fn strchr(s: *const c_char, c: c_char) -> *const c_char {
    let mut p = s;
    loop {
        let b = unsafe { *p };
        if b == c {
            return p;
        }
        if b == 0 {
            return std::ptr::null();
        }
        p = unsafe { p.add(1) };
    }
}

/// int foo(const char *in, char c)
///
/// Counts occurrences of `c` in `in`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foo(r#in: *const c_char, c: c_char) -> c_int {
    let mut res: c_int = 0;
    // for (const char *s = in; s = strchr(s, c); s++) { res++; }
    let mut s = r#in;
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

/// void driver(const char *in)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(r#in: *const c_char) {
    c_printf(c"A: %d\n".as_ptr(), unsafe { foo(r#in, b'A' as c_char) });
    c_printf(c"x: %d\n".as_ptr(), unsafe { foo(r#in, b'x' as c_char) });
}
