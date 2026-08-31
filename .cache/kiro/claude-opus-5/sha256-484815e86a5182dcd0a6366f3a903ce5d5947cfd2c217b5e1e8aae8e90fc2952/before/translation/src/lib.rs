// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/driver.c`.
//!
//! The C source calls the `<ctype.h>` classification routines and prints their
//! results with `%d`. Under glibc those names are *macros*
//! (`#define isalpha(c) __isctype((c), _ISalpha)`), which expand to
//! `(*__ctype_b_loc())[(int)(c)] & _ISalpha`. The value printed is therefore the
//! masked class *bit*, not a normalized `1`. Those bit values are reproduced
//! here verbatim (see `class_bits`).
//!
//! `char` is signed on the platforms targeted by the original CMake project, so
//! `driver` receives values in `-128..=127`. glibc's ctype tables are indexed
//! from `-128`, and in the `"C"` locale the entries for negative indices carry
//! no class bits and map to themselves for case conversion. That behaviour is
//! reproduced as well.

use std::ffi::{c_char, c_int};

// glibc `enum` from <ctype.h>: _ISbit(b) = b < 8 ? (1 << b) << 8 : (1 << b) >> 8
const IS_UPPER: u16 = 0x0100; // _ISbit(0)
const IS_LOWER: u16 = 0x0200; // _ISbit(1)
const IS_ALPHA: u16 = 0x0400; // _ISbit(2)
const IS_DIGIT: u16 = 0x0800; // _ISbit(3)
const IS_XDIGIT: u16 = 0x1000; // _ISbit(4)
const IS_SPACE: u16 = 0x2000; // _ISbit(5)
const IS_PRINT: u16 = 0x4000; // _ISbit(6)
const IS_GRAPH: u16 = 0x8000; // _ISbit(7)
const IS_BLANK: u16 = 0x0001; // _ISbit(8)
const IS_CNTRL: u16 = 0x0002; // _ISbit(9)
const IS_PUNCT: u16 = 0x0004; // _ISbit(10)
const IS_ALNUM: u16 = 0x0008; // _ISbit(11)

/// The `"C"` locale class bits for a single byte, mirroring glibc's
/// `__ctype_b` table. Bytes `>= 0x80` (i.e. negative `char` values) carry no
/// class bits in the `"C"` locale.
const fn class_bits(b: u8) -> u16 {
    let mut bits = 0u16;

    let upper = b.is_ascii_uppercase();
    let lower = b.is_ascii_lowercase();
    let digit = b.is_ascii_digit();
    let alpha = upper || lower;
    let alnum = alpha || digit;
    // 0x21..=0x7e
    let graph = b > 0x20 && b < 0x7f;
    // 0x20..=0x7e
    let print = b >= 0x20 && b < 0x7f;
    let space = b == b' ' || (b >= 0x09 && b <= 0x0d);
    let blank = b == b' ' || b == b'\t';
    let cntrl = b < 0x20 || b == 0x7f;

    if upper {
        bits |= IS_UPPER;
    }
    if lower {
        bits |= IS_LOWER;
    }
    if alpha {
        bits |= IS_ALPHA;
    }
    if digit {
        bits |= IS_DIGIT;
    }
    if digit || (b >= b'A' && b <= b'F') || (b >= b'a' && b <= b'f') {
        bits |= IS_XDIGIT;
    }
    if space {
        bits |= IS_SPACE;
    }
    if print {
        bits |= IS_PRINT;
    }
    if graph {
        bits |= IS_GRAPH;
    }
    if blank {
        bits |= IS_BLANK;
    }
    if cntrl {
        bits |= IS_CNTRL;
    }
    // punct == graph && !alnum in the "C" locale
    if graph && !alnum {
        bits |= IS_PUNCT;
    }
    if alnum {
        bits |= IS_ALNUM;
    }

    bits
}

/// glibc's `__ctype_b` table restricted to the index range a signed `char`
/// can produce, laid out by the raw byte value.
const CTYPE_B: [u16; 256] = {
    let mut table = [0u16; 256];
    let mut i = 0usize;
    while i < 256 {
        table[i] = class_bits(i as u8);
        i += 1;
    }
    table
};

/// `(*__ctype_b_loc())[(int) c] & mask`, promoted to `int` as the C macro does.
fn isctype(c: c_char, mask: u16) -> c_int {
    c_int::from(CTYPE_B[c as u8 as usize] & mask)
}

/// glibc `tolower` for the `"C"` locale. Entries outside `a-z`/`A-Z` — including
/// the negative `char` indices — map to themselves.
fn c_tolower(c: c_char) -> c_int {
    let b = c as u8;
    if b.is_ascii_uppercase() {
        // Value stored in glibc's table for an uppercase ASCII letter.
        c_int::from(b + 32)
    } else {
        // Negative indices map to the corresponding 0x80..=0xff value.
        c_int::from(b)
    }
}

/// glibc `toupper` for the `"C"` locale.
fn c_toupper(c: c_char) -> c_int {
    let b = c as u8;
    if b.is_ascii_lowercase() {
        c_int::from(b - 32)
    } else {
        c_int::from(b)
    }
}

// Use the C library directly so that stdout buffering, flushing and
// interleaving with any caller-side output are bit-for-bit identical to the
// original. (Rust's own `stdout` buffer would not be flushed when the host
// program exits, since `main` lives outside this cdylib.)
unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
    #[link_name = "setlocale"]
    unsafe fn c_setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

// <locale.h>, glibc: LC_ALL == 6
const LC_ALL: c_int = 6;

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(c: c_char) {
    unsafe {
        c_setlocale(LC_ALL, cstr!("C"));

        c_printf(cstr!("alphanumeric: %d\n"), isctype(c, IS_ALNUM));
        c_printf(cstr!("alphabetic: %d\n"), isctype(c, IS_ALPHA));
        c_printf(cstr!("lowercase: %d\n"), isctype(c, IS_LOWER));
        c_printf(cstr!("uppercase: %d\n"), isctype(c, IS_UPPER));
        c_printf(cstr!("digit: %d\n"), isctype(c, IS_DIGIT));
        c_printf(cstr!("hexadecimal: %d\n"), isctype(c, IS_XDIGIT));
        c_printf(cstr!("control: %d\n"), isctype(c, IS_CNTRL));
        c_printf(cstr!("graphical: %d\n"), isctype(c, IS_GRAPH));
        c_printf(cstr!("space: %d\n"), isctype(c, IS_SPACE));
        c_printf(cstr!("blank: %d\n"), isctype(c, IS_BLANK));
        c_printf(cstr!("printing: %d\n"), isctype(c, IS_PRINT));
        c_printf(cstr!("punctuation: %d\n"), isctype(c, IS_PUNCT));
        c_printf(cstr!("to lower: %c\n"), c_tolower(c));
        c_printf(cstr!("to upper: %c\n"), c_toupper(c));
    }
}
