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

//! Rust translation of `c_src/src/main.c`.
//!
//! The C program reads a single byte with `getchar()`, truncates it to a
//! (signed) `char`, and prints the result of every `<ctype.h>` classifier plus
//! `tolower`/`toupper` for that value in the `"C"` locale.
//!
//! Two glibc implementation details are reproduced here verbatim, because the
//! C program's observable output depends on them:
//!
//! 1. `isalnum` and friends are macros expanding to
//!    `(*__ctype_b_loc ())[(int) (c)] & _ISxxx`, so they yield the *raw class
//!    bit*, not a normalized `1`. `printf("%d")` therefore prints values such
//!    as `1024` for `isalpha('A')`.
//! 2. The `__ctype_b` / `__ctype_tolower` / `__ctype_toupper` tables are
//!    indexed from `-128` to `255`. Entries `-128..=-1` (which is what a
//!    negative `char` produces) classify as nothing at all, and map to
//!    themselves modulo 256 under `tolower`/`toupper`. `EOF` truncated to
//!    `char` is `-1`, so it behaves exactly like the byte `0xff`.

use std::io::{Read, Write};

// glibc <ctype.h> class bits: _ISbit(b) == if b < 8 { (1 << b) << 8 } else { (1 << b) >> 8 }
const IS_UPPER: i32 = 0x0100; // 256
const IS_LOWER: i32 = 0x0200; // 512
const IS_ALPHA: i32 = 0x0400; // 1024
const IS_DIGIT: i32 = 0x0800; // 2048
const IS_XDIGIT: i32 = 0x1000; // 4096
const IS_SPACE: i32 = 0x2000; // 8192
const IS_PRINT: i32 = 0x4000; // 16384
const IS_GRAPH: i32 = 0x8000; // 32768
const IS_BLANK: i32 = 0x0001; // 1
const IS_CNTRL: i32 = 0x0002; // 2
const IS_PUNCT: i32 = 0x0004; // 4
const IS_ALNUM: i32 = 0x0008; // 8

/// Class bitmask for one table index, matching glibc's `C` locale
/// `__ctype_b` table. Indices outside `0..=127` (i.e. the negative half of the
/// table, reached via a negative `char`) carry no class bits.
fn ctype_b(idx: i32) -> i32 {
    if !(0..=127).contains(&idx) {
        return 0;
    }
    let b = idx as u8;
    let mut m = 0;

    let upper = b.is_ascii_uppercase();
    let lower = b.is_ascii_lowercase();
    let alpha = upper || lower;
    let digit = b.is_ascii_digit();
    let alnum = alpha || digit;
    let graph = (0x21..=0x7e).contains(&b);
    let print = (0x20..=0x7e).contains(&b);

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
    if matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        m |= IS_SPACE;
    }
    if print {
        m |= IS_PRINT;
    }
    if graph {
        m |= IS_GRAPH;
    }
    if matches!(b, b' ' | b'\t') {
        m |= IS_BLANK;
    }
    if b <= 0x1f || b == 0x7f {
        m |= IS_CNTRL;
    }
    if graph && !alnum {
        m |= IS_PUNCT;
    }
    if alnum {
        m |= IS_ALNUM;
    }
    m
}

/// glibc `C` locale `__ctype_tolower` lookup. Negative indices map to
/// `idx + 256` (identity on the raw byte).
fn c_tolower(idx: i32) -> i32 {
    match idx {
        0x41..=0x5a => idx + 32,
        0..=255 => idx,
        _ => idx & 0xff,
    }
}

/// glibc `C` locale `__ctype_toupper` lookup. Negative indices map to
/// `idx + 256` (identity on the raw byte).
fn c_toupper(idx: i32) -> i32 {
    match idx {
        0x61..=0x7a => idx - 32,
        0..=255 => idx,
        _ => idx & 0xff,
    }
}

fn driver(c: i8, out: &mut Vec<u8>) {
    // setlocale(LC_ALL, "C") is a no-op here: the "C" locale tables above are
    // the ones glibc already starts with.
    let i = c as i32;
    let b = ctype_b(i);

    let mut line = |name: &str, value: i32| {
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(value.to_string().as_bytes());
        out.push(b'\n');
    };

    line("alphanumeric", b & IS_ALNUM);
    line("alphabetic", b & IS_ALPHA);
    line("lowercase", b & IS_LOWER);
    line("uppercase", b & IS_UPPER);
    line("digit", b & IS_DIGIT);
    line("hexadecimal", b & IS_XDIGIT);
    line("control", b & IS_CNTRL);
    line("graphical", b & IS_GRAPH);
    line("space", b & IS_SPACE);
    line("blank", b & IS_BLANK);
    line("printing", b & IS_PRINT);
    line("punctuation", b & IS_PUNCT);

    // printf("%c", x) converts the int argument to unsigned char.
    out.extend_from_slice(b"to lower: ");
    out.push((c_tolower(i) & 0xff) as u8);
    out.push(b'\n');

    out.extend_from_slice(b"to upper: ");
    out.push((c_toupper(i) & 0xff) as u8);
    out.push(b'\n');
}

// The Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main` runs, while a
// C program keeps the default disposition. Without restoring it, a write to a
// closed stdout makes this program exit 0 where the C program is killed by
// signal 13 (shell status 141). Reset it so the observable behaviour matches.
extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}

fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

/// `getchar()`: one byte from stdin, or `EOF` (-1) at end of input or on error.
fn getchar() -> i32 {
    let mut buf = [0u8; 1];
    let mut stdin = std::io::stdin().lock();
    loop {
        match stdin.read(&mut buf) {
            Ok(0) => return -1,
            Ok(_) => return i32::from(buf[0]),
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return -1,
        }
    }
}

fn main() {
    restore_default_sigpipe();

    // char c = getchar();  -- truncation to a signed char on this platform.
    let c = getchar() as i8;

    let mut out = Vec::with_capacity(256);
    driver(c, &mut out);

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let _ = stdout.write_all(&out);
    let _ = stdout.flush();
}
