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

//! Faithful reproduction of the glibc `<ctype.h>` macros.
//!
//! glibc's `<ctype.h>` implements the classification interfaces as
//!
//! ```c
//! #define __isctype(c, type) \
//!   ((*__ctype_b_loc ())[(int) (c)] & (unsigned short int) type)
//! #define isalnum(c) __isctype((c), _ISalnum)
//! ```
//!
//! Two consequences are load-bearing for byte-identical output and are
//! preserved here verbatim:
//!
//! 1. The result is the **masked table bits**, not a normalised `0`/`1`.  So
//!    `isalpha('a')` yields `1024` and `isgraph('a')` yields `32768`, which is
//!    what the reference library prints.
//! 2. The table index is `(int) c`.  Because `char` is *signed* on this target,
//!    a `char` above `0x7F` sign-extends to a negative index.  glibc's tables
//!    intentionally cover `-128 ..= 255`, so the lookup stays in bounds and
//!    reproduces the C behaviour rather than trapping or wrapping.
//!
//! The conversion interfaces behave the same way:
//!
//! ```c
//! #define tolower(c) __tobody (c, tolower, *__ctype_tolower_loc (), (c))
//! ```
//!
//! and for a `char` argument (`sizeof (c) == 1`) `__tobody` reduces to the
//! plain table lookup `(*__ctype_tolower_loc ())[(int) (c)]`.  The
//! out-of-line `tolower`/`toupper` functions that an unoptimised build calls
//! instead are defined as `c >= -128 && c < 256 ? table[c] : c`, which for any
//! `char` input is the identical table lookup — verified by diffing the output
//! of an optimised and an unoptimised build of the reference library.

use crate::ffi::{__ctype_b_loc, __ctype_tolower_loc, __ctype_toupper_loc};
use core::ffi::{c_char, c_int};

/// glibc's `_ISbit(bit)`:
///
/// ```c
/// # define _ISbit(bit) ((bit) < 8 ? ((1 << (bit)) << 8) : ((1 << (bit)) >> 8))
/// ```
const fn is_bit(bit: u32) -> u16 {
    if bit < 8 {
        (1u16 << bit) << 8
    } else {
        (1u16 << bit) >> 8
    }
}

/// The `enum` of character-class bits from glibc's `<ctype.h>`.
pub const IS_UPPER: u16 = is_bit(0); // 0x0100 =   256
pub const IS_LOWER: u16 = is_bit(1); // 0x0200 =   512
pub const IS_ALPHA: u16 = is_bit(2); // 0x0400 =  1024
pub const IS_DIGIT: u16 = is_bit(3); // 0x0800 =  2048
pub const IS_XDIGIT: u16 = is_bit(4); // 0x1000 =  4096
pub const IS_SPACE: u16 = is_bit(5); // 0x2000 =  8192
pub const IS_PRINT: u16 = is_bit(6); // 0x4000 = 16384
pub const IS_GRAPH: u16 = is_bit(7); // 0x8000 = 32768
pub const IS_BLANK: u16 = is_bit(8); // 0x0001 =     1
pub const IS_CNTRL: u16 = is_bit(9); // 0x0002 =     2
pub const IS_PUNCT: u16 = is_bit(10); // 0x0004 =     4
pub const IS_ALNUM: u16 = is_bit(11); // 0x0008 =     8

/// Reads the character-class entry for `c` out of the current locale's table.
///
/// Mirrors `(*__ctype_b_loc ())[(int) (c)]`.
fn ctype_class_bits(c: c_char) -> u16 {
    // `(int) c` — sign-extends, exactly as the C cast does.
    let index = c as c_int as isize;

    // SAFETY: `__ctype_b_loc` returns a pointer to a non-null table pointer for
    // the calling thread's locale, and that table is defined for the index
    // range `-128 ..= 255`.  `index` comes from a `c_char`, so it is confined
    // to `-128 ..= 127` and is therefore always in bounds.
    unsafe { *(*__ctype_b_loc()).offset(index) }
}

/// Mirrors glibc's `__isctype(c, mask)`.
///
/// The C expression's operands are both promoted to `int`, and an
/// `unsigned short` promotes to a non-negative `int`, so the result is the
/// zero-extended mask value.
fn isctype(c: c_char, mask: u16) -> c_int {
    c_int::from(ctype_class_bits(c) & mask)
}

/// Reads `c`'s entry from one of the case-conversion tables.
///
/// Mirrors `(*__ctype_tolower_loc ())[(int) (c)]` /
/// `(*__ctype_toupper_loc ())[(int) (c)]`.
fn convert_case(table: *mut *const i32, c: c_char) -> c_int {
    let index = c as c_int as isize;

    // SAFETY: as in `ctype_class_bits` — the conversion tables returned by
    // glibc are likewise defined over `-128 ..= 255`, and `index` is derived
    // from a `c_char` so it lies within `-128 ..= 127`.
    unsafe { *(*table).offset(index) }
}

pub fn isalnum(c: c_char) -> c_int {
    isctype(c, IS_ALNUM)
}

pub fn isalpha(c: c_char) -> c_int {
    isctype(c, IS_ALPHA)
}

pub fn islower(c: c_char) -> c_int {
    isctype(c, IS_LOWER)
}

pub fn isupper(c: c_char) -> c_int {
    isctype(c, IS_UPPER)
}

pub fn isdigit(c: c_char) -> c_int {
    isctype(c, IS_DIGIT)
}

pub fn isxdigit(c: c_char) -> c_int {
    isctype(c, IS_XDIGIT)
}

pub fn iscntrl(c: c_char) -> c_int {
    isctype(c, IS_CNTRL)
}

pub fn isgraph(c: c_char) -> c_int {
    isctype(c, IS_GRAPH)
}

pub fn isspace(c: c_char) -> c_int {
    isctype(c, IS_SPACE)
}

pub fn isblank(c: c_char) -> c_int {
    isctype(c, IS_BLANK)
}

pub fn isprint(c: c_char) -> c_int {
    isctype(c, IS_PRINT)
}

pub fn ispunct(c: c_char) -> c_int {
    isctype(c, IS_PUNCT)
}

pub fn tolower(c: c_char) -> c_int {
    // SAFETY: `__ctype_tolower_loc` is a plain glibc accessor returning the
    // thread's current conversion table pointer.
    convert_case(unsafe { __ctype_tolower_loc() }, c)
}

pub fn toupper(c: c_char) -> c_int {
    // SAFETY: see `tolower`.
    convert_case(unsafe { __ctype_toupper_loc() }, c)
}
