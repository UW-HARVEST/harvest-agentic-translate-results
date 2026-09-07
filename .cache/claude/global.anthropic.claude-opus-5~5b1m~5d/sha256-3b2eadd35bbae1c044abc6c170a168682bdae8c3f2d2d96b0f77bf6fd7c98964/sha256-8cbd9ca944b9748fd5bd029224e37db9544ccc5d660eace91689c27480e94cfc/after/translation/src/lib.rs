// Rust translation of the C library in c_src/.
//
// Original copyright notice from the C sources:
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

#![allow(non_snake_case)]

use std::ffi::c_char;
use std::ffi::c_int;

// The C sources are written with ISO 646 alternative spellings and digraphs:
//   `%:` == `#`, `<%` == `{`, `%>` == `}`,
//   `bitor` == `|`, `compl` == `~`  (from <iso646.h>).
//
// So `src/driver.c` is equivalent to:
//
//     void driver(int x, int y) {
//         int result = x | ~y;
//         printf("%d", result);
//         puts("");
//     }
//
// Output is emitted through the C standard library's stdio so that the observable
// byte stream (and its buffering/interleaving with any other C output) is identical.

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn puts(s: *const c_char) -> c_int;
}

/// `void driver(int x, int y);` — see include/driver.h
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    // `x bitor compl y` => x | ~y  (two's-complement bitwise ops, no overflow possible)
    let result: c_int = x | !y;

    unsafe {
        printf(b"%d\0".as_ptr() as *const c_char, result);
        puts(b"\0".as_ptr() as *const c_char);
    }
}
