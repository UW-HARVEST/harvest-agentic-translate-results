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

//! Minimal, hand-written bindings to the libc entry points that the original
//! C translation unit referenced.
//!
//! `driver.c` includes `<ctype.h>`, `<locale.h>` and `<stdio.h>` and uses
//! `printf`, `setlocale` and the `<ctype.h>` classification/conversion
//! interfaces.  Under glibc the `<ctype.h>` interfaces expand to macros that
//! index the per-locale lookup tables returned by `__ctype_b_loc`,
//! `__ctype_tolower_loc` and `__ctype_toupper_loc`; that is exactly what the
//! reference shared object references in its dynamic symbol table:
//!
//! ```text
//! U __ctype_b_loc@GLIBC_2.3
//! U __ctype_tolower_loc@GLIBC_2.3
//! U __ctype_toupper_loc@GLIBC_2.3
//! U printf@GLIBC_2.2.5
//! U setlocale@GLIBC_2.2.5
//! ```
//!
//! Binding to the very same tables (rather than reimplementing the ASCII
//! classification logic in Rust) is what guarantees byte-identical output: the
//! glibc macros return the *raw masked table bits* (e.g. `1024` for
//! `isalpha`), not a normalised `0`/`1`, and the tables also cover the
//! negative indices produced by sign-extending a negative `char`.

use core::ffi::{c_char, c_int};

/// `locale.h`: category argument for `setlocale`. glibc defines `LC_ALL` as 6.
pub const LC_ALL: c_int = 6;

unsafe extern "C" {
    /// `int printf(const char *restrict format, ...)`
    ///
    /// Reusing libc's `printf` (instead of Rust's `std::io::stdout`) keeps the
    /// formatting, the stdout `FILE` object and its buffering behaviour bit
    /// for bit identical to the C library, including how the output
    /// interleaves with any `printf` performed by the calling program.
    pub fn printf(format: *const c_char, ...) -> c_int;

    /// `char *setlocale(int category, const char *locale)`
    pub fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;

    /// glibc: pointer to the current locale's character-class table.
    ///
    /// The returned table is valid for the index range `-128 ..= 255`, which is
    /// why sign-extended negative `char` values are legal lookups.
    pub fn __ctype_b_loc() -> *mut *const u16;

    /// glibc: pointer to the current locale's `tolower` conversion table.
    pub fn __ctype_tolower_loc() -> *mut *const i32;

    /// glibc: pointer to the current locale's `toupper` conversion table.
    pub fn __ctype_toupper_loc() -> *mut *const i32;
}
