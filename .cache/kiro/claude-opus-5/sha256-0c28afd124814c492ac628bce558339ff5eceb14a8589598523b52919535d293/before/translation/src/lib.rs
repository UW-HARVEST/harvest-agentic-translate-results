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

// ---------------------------------------------------------------------------
// C runtime interop
//
// Output is emitted through the C library's `printf` (rather than Rust's
// `println!`) so that this library shares the caller's stdio stream and
// buffering exactly as the original C did.  `setlocale` is likewise the real
// libc call, because the original `driver()` mutates global locale state and a
// caller may observe that.
// ---------------------------------------------------------------------------
extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `LC_ALL` on glibc / Linux.
const LC_ALL: c_int = 6;

// ---------------------------------------------------------------------------
// glibc `<ctype.h>` character-class bits, from `_ISbit(n)`:
//
//     _ISbit(n) = n < 8 ? (1 << n) << 8 : (1 << n) >> 8
//
// This matters for byte-identical output: glibc's `isalpha()` and friends
// return the masked table entry, *not* a normalized 0/1.  So `isalpha('a')`
// yields 1024, and the C program prints "alphabetic: 1024".
// ---------------------------------------------------------------------------
const IS_UPPER: c_int = 0x0100; // 256
const IS_LOWER: c_int = 0x0200; // 512
const IS_ALPHA: c_int = 0x0400; // 1024
const IS_DIGIT: c_int = 0x0800; // 2048
const IS_XDIGIT: c_int = 0x1000; // 4096
const IS_SPACE: c_int = 0x2000; // 8192
const IS_PRINT: c_int = 0x4000; // 16384
const IS_GRAPH: c_int = 0x8000; // 32768
const IS_BLANK: c_int = 0x0001; // 1
const IS_CNTRL: c_int = 0x0002; // 2
const IS_PUNCT: c_int = 0x0004; // 4
const IS_ALNUM: c_int = 0x0008; // 8

/// The class bits glibc's "C" locale table holds for a character.
///
/// glibc indexes its table with the (possibly negative) `int` value of the
/// argument; the table's negative half (-128..=-1, i.e. what a *signed* `char`
/// holding a byte >= 0x80 becomes) is all zeroes in the "C" locale, so every
/// class query for such a byte answers 0.
fn ctype_class(c: c_int) -> c_int {
    if !(0..=127).contains(&c) {
        return 0;
    }
    let b = c as u8;
    let mut m = 0;

    let upper = b.is_ascii_uppercase();
    let lower = b.is_ascii_lowercase();
    let digit = b.is_ascii_digit();
    let alpha = upper || lower;
    // Printable: SPACE through '~'.  Graphical is the same minus SPACE.
    let print = (0x20..=0x7e).contains(&b);
    let graph = (0x21..=0x7e).contains(&b);

    if upper {
        m |= IS_UPPER;
    }
    if lower {
        m |= IS_LOWER;
    }
    if alpha {
        m |= IS_ALPHA;
    }
    if digit {
        m |= IS_DIGIT;
    }
    if b.is_ascii_hexdigit() {
        m |= IS_XDIGIT;
    }
    // Whitespace in the "C" locale: HT, LF, VT, FF, CR and SPACE.
    if matches!(b, b'\t' | b'\n' | 0x0b | 0x0c | b'\r' | b' ') {
        m |= IS_SPACE;
    }
    if print {
        m |= IS_PRINT;
    }
    if graph {
        m |= IS_GRAPH;
    }
    // Blank in the "C" locale: HT and SPACE.
    if matches!(b, b'\t' | b' ') {
        m |= IS_BLANK;
    }
    if b < 0x20 || b == 0x7f {
        m |= IS_CNTRL;
    }
    // Punctuation: graphical but neither alphabetic nor numeric.
    if graph && !alpha && !digit {
        m |= IS_PUNCT;
    }
    if alpha || digit {
        m |= IS_ALNUM;
    }
    m
}

fn isalnum(c: c_int) -> c_int {
    ctype_class(c) & IS_ALNUM
}
fn isalpha(c: c_int) -> c_int {
    ctype_class(c) & IS_ALPHA
}
fn islower(c: c_int) -> c_int {
    ctype_class(c) & IS_LOWER
}
fn isupper(c: c_int) -> c_int {
    ctype_class(c) & IS_UPPER
}
fn isdigit(c: c_int) -> c_int {
    ctype_class(c) & IS_DIGIT
}
fn isxdigit(c: c_int) -> c_int {
    ctype_class(c) & IS_XDIGIT
}
fn iscntrl(c: c_int) -> c_int {
    ctype_class(c) & IS_CNTRL
}
fn isgraph(c: c_int) -> c_int {
    ctype_class(c) & IS_GRAPH
}
fn isspace(c: c_int) -> c_int {
    ctype_class(c) & IS_SPACE
}
fn isblank(c: c_int) -> c_int {
    ctype_class(c) & IS_BLANK
}
fn isprint(c: c_int) -> c_int {
    ctype_class(c) & IS_PRINT
}
fn ispunct(c: c_int) -> c_int {
    ctype_class(c) & IS_PUNCT
}

/// "C" locale `tolower`: only 'A'..='Z' map; everything else (including the
/// negative indices produced by high bytes in a signed `char`) is identity.
fn tolower(c: c_int) -> c_int {
    if (b'A' as c_int..=b'Z' as c_int).contains(&c) {
        c + 32
    } else {
        c
    }
}

/// "C" locale `toupper`; identity outside 'a'..='z'.
fn toupper(c: c_int) -> c_int {
    if (b'a' as c_int..=b'z' as c_int).contains(&c) {
        c - 32
    } else {
        c
    }
}

/// Translation of `void driver(char c)`.
///
/// `c` is a C `char`, which is *signed* on the x86-64 / AArch64 Linux ABIs, so
/// it is modelled as `i8` and widens to a negative `c_int` for bytes >= 0x80 —
/// matching the original's integer promotion at each `is*`/`to*` call site.
#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    let c: c_int = c as c_int;

    unsafe {
        setlocale(LC_ALL, c"C".as_ptr());

        printf(c"alphanumeric: %d\n".as_ptr(), isalnum(c));
        printf(c"alphabetic: %d\n".as_ptr(), isalpha(c));
        printf(c"lowercase: %d\n".as_ptr(), islower(c));
        printf(c"uppercase: %d\n".as_ptr(), isupper(c));
        printf(c"digit: %d\n".as_ptr(), isdigit(c));
        printf(c"hexadecimal: %d\n".as_ptr(), isxdigit(c));
        printf(c"control: %d\n".as_ptr(), iscntrl(c));
        printf(c"graphical: %d\n".as_ptr(), isgraph(c));
        printf(c"space: %d\n".as_ptr(), isspace(c));
        printf(c"blank: %d\n".as_ptr(), isblank(c));
        printf(c"printing: %d\n".as_ptr(), isprint(c));
        printf(c"punctuation: %d\n".as_ptr(), ispunct(c));
        printf(c"to lower: %c\n".as_ptr(), tolower(c));
        printf(c"to upper: %c\n".as_ptr(), toupper(c));
    }
}
