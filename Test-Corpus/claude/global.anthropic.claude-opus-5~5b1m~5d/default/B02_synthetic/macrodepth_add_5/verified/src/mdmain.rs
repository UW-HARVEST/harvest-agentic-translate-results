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

//! Rust equivalent of `mdmain.c`.
//!
//! The C build (`c_src/CMakeLists.txt`) compiles `mdcore.c` *and* `mdmain.c`
//! into one artifact, so the entry point `main` is part of the exported symbol
//! surface of the shared object and is translated here with the same name and
//! signature (`int main(int argc, char **argv)`).

use core::ffi::{c_char, c_int};
use core::ptr;

use crate::cshim::{atoi, fprintf, printf, stderr};
use crate::mdcore::{helper_call, helper_ptr, use_generated, G_OP, G_OP_NAME};
use crate::mdmacros::{op_fn, run_loop, INIT_FOR, REPEAT};

/// ```c
/// int main(int argc, char **argv) {
///     if (argc < 3) {
///         fprintf(stderr, "usage: %s A B\n", argv[0]);
///         return 2;
///     }
///     int a = atoi(argv[1]);
///     int b = atoi(argv[2]);
///
///     int r_call = (OP_FN(OP))(a, b);
///     int acc = INIT_FOR(OP);
///     RUN_LOOP(OP, acc, REPEAT);
///
///     int x1 = helper_call(a, b);
///     int x2 = helper_ptr(a, b);
///     int x3 = use_generated(REPEAT);
///     int g  = G_OP(a, b);
///
///     printf("op=%s call=%d acc=%d g.call=%d\n", G_OP_NAME, r_call, acc, g);
///     printf("summary=%d\n", r_call + acc + x1 + x2 + x3 + g);
///     return 0;
/// }
/// ```
///
/// Unlike `helper_ptr`, `main` calls the operation *through the `G_OP` global*
/// and prints the *global* `G_OP_NAME`, so both reads observe whatever a caller
/// stored into those exported objects.
///
/// # Safety
///
/// `argv` must be a valid pointer to at least `max(argc, 1)` C-string pointers,
/// exactly as required by the C original (which dereferences `argv[0]` even when
/// `argc == 0`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    if argc < 3 {
        fprintf(
            ptr::read(&raw const stderr),
            b"usage: %s A B\n\0".as_ptr() as *const c_char,
            ptr::read(argv),
        );
        return 2;
    }
    let a = atoi(ptr::read(argv.offset(1)) as *const c_char);
    let b = atoi(ptr::read(argv.offset(2)) as *const c_char);

    let r_call = op_fn(a, b);
    let acc = run_loop(INIT_FOR);

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    let g = (ptr::read(&raw const G_OP))(a, b);

    printf(
        b"op=%s call=%d acc=%d g.call=%d\n\0".as_ptr() as *const c_char,
        ptr::read(&raw const G_OP_NAME),
        r_call,
        acc,
        g,
    );
    printf(
        b"summary=%d\n\0".as_ptr() as *const c_char,
        r_call
            .wrapping_add(acc)
            .wrapping_add(x1)
            .wrapping_add(x2)
            .wrapping_add(x3)
            .wrapping_add(g),
    );
    0
}
