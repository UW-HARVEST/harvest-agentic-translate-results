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
//! The C program reads a single byte with `getchar()`, stores it in a `char`
//! (signed on the reference platform), and prints the result of every
//! `<ctype.h>` classification query plus `tolower`/`toupper`.
//!
//! The `is*` functions in glibc are table lookups that return the *masked bit*
//! rather than a normalized 1, so `printf("%d", isalpha('a'))` prints `1024`,
//! not `1`. Those exact values are reproduced here. The lookup table is indexed
//! by the (possibly negative) `int` value of the `char`; in the "C" locale every
//! negative index has an all-zero class entry and an identity case mapping,
//! which is why bytes >= 0x80 report `0` for every class.

use std::io::{self, Read, Write};

/// glibc `_ISbit` class masks for the "C" locale.
///
/// `_ISbit(bit)` is `((bit) < 8 ? ((1 << bit) << 8) : ((1 << bit) >> 8))`,
/// which is what makes the printed values look arbitrary.
mod mask {
    pub const UPPER: i32 = 1 << 8; // 256
    pub const LOWER: i32 = 2 << 8; // 512
    pub const ALPHA: i32 = 4 << 8; // 1024
    pub const DIGIT: i32 = 8 << 8; // 2048
    pub const XDIGIT: i32 = 16 << 8; // 4096
    pub const SPACE: i32 = 32 << 8; // 8192
    pub const PRINT: i32 = 64 << 8; // 16384
    pub const GRAPH: i32 = 128 << 8; // 32768
    pub const BLANK: i32 = (1 << 8) >> 8; // 1
    pub const CNTRL: i32 = (1 << 9) >> 8; // 2
    pub const PUNCT: i32 = (1 << 10) >> 8; // 4
    pub const ALNUM: i32 = (1 << 11) >> 8; // 8
}

/// Class entry for one ASCII code point in the "C" locale.
const fn class_entry(b: u8) -> i32 {
    let mut m: i32 = 0;

    let upper = b >= b'A' && b <= b'Z';
    let lower = b >= b'a' && b <= b'z';
    let digit = b >= b'0' && b <= b'9';
    let hex_letter = (b >= b'a' && b <= b'f') || (b >= b'A' && b <= b'F');
    let print = b >= 0x20 && b <= 0x7e;
    let graph = b >= 0x21 && b <= 0x7e;

    if upper {
        m |= mask::UPPER;
    }
    if lower {
        m |= mask::LOWER;
    }
    if upper || lower {
        m |= mask::ALPHA;
    }
    if digit {
        m |= mask::DIGIT;
    }
    if digit || hex_letter {
        m |= mask::XDIGIT;
    }
    // "C" locale whitespace: space, \t, \n, \v, \f, \r
    if b == b' ' || (b >= 0x09 && b <= 0x0d) {
        m |= mask::SPACE;
    }
    if b == b' ' || b == b'\t' {
        m |= mask::BLANK;
    }
    if print {
        m |= mask::PRINT;
    }
    if graph {
        m |= mask::GRAPH;
    }
    if b <= 0x1f || b == 0x7f {
        m |= mask::CNTRL;
    }
    // punctuation is everything graphical that is not alphanumeric
    if graph && !(upper || lower || digit) {
        m |= mask::PUNCT;
    }
    if upper || lower || digit {
        m |= mask::ALNUM;
    }

    m
}

/// The `__ctype_b` lookup for a `char`, matching glibc's "C" locale table.
///
/// Indices outside `0..=127` (i.e. the negative half of a signed `char`) have
/// an all-zero entry, so no class matches.
fn ctype_class(c: i8) -> i32 {
    if c < 0 {
        0
    } else {
        class_entry(c as u8)
    }
}

/// `tolower` as an `int`-returning table lookup; identity outside `A..=Z`.
fn c_tolower(c: i8) -> i32 {
    if c >= b'A' as i8 && c <= b'Z' as i8 {
        c as i32 + 32
    } else {
        c as i32
    }
}

/// `toupper` as an `int`-returning table lookup; identity outside `a..=z`.
fn c_toupper(c: i8) -> i32 {
    if c >= b'a' as i8 && c <= b'z' as i8 {
        c as i32 - 32
    } else {
        c as i32
    }
}

/// Emit one `printf("<label>: %d\n", value)` line.
fn print_int(out: &mut impl Write, label: &str, value: i32) -> io::Result<()> {
    write!(out, "{}: {}\n", label, value)
}

/// Emit one `printf("<label>: %c\n", value)` line.
///
/// `%c` converts the `int` argument to `unsigned char`, so only the low byte is
/// written and it is written raw (never UTF-8 encoded).
fn print_char(out: &mut impl Write, label: &str, value: i32) -> io::Result<()> {
    write!(out, "{}: ", label)?;
    out.write_all(&[value as u8])?;
    out.write_all(b"\n")
}

fn driver(out: &mut impl Write, c: i8) -> io::Result<()> {
    // setlocale(LC_ALL, "C") -- the "C" locale is what the tables above encode.

    let class = ctype_class(c);

    print_int(out, "alphanumeric", class & mask::ALNUM)?;
    print_int(out, "alphabetic", class & mask::ALPHA)?;
    print_int(out, "lowercase", class & mask::LOWER)?;
    print_int(out, "uppercase", class & mask::UPPER)?;
    print_int(out, "digit", class & mask::DIGIT)?;
    print_int(out, "hexadecimal", class & mask::XDIGIT)?;
    print_int(out, "control", class & mask::CNTRL)?;
    print_int(out, "graphical", class & mask::GRAPH)?;
    print_int(out, "space", class & mask::SPACE)?;
    print_int(out, "blank", class & mask::BLANK)?;
    print_int(out, "printing", class & mask::PRINT)?;
    print_int(out, "punctuation", class & mask::PUNCT)?;
    print_char(out, "to lower", c_tolower(c))?;
    print_char(out, "to upper", c_toupper(c))?;

    Ok(())
}

/// `char c = getchar();` -- one byte from stdin, or `(char)EOF` == -1 at EOF.
fn getchar_as_char() -> i8 {
    let mut byte = [0u8; 1];
    match io::stdin().read(&mut byte) {
        Ok(1) => byte[0] as i8,
        _ => -1, // (char)EOF
    }
}

extern "C" {
    /// POSIX `signal(2)`; libc is already linked on the gnu targets, so no
    /// external crate is needed to reach it.
    fn signal(signum: i32, handler: usize) -> usize;
}

/// Restore the default `SIGPIPE` disposition.
///
/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main`, so a write to a
/// broken pipe returns `EPIPE` instead of killing the process. A C program
/// keeps the inherited default, so it dies from signal 13 and reports exit
/// status 141 through a shell. Resetting it here keeps the exit status
/// identical when stdout has no reader.
fn reset_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    reset_sigpipe();

    let c = getchar_as_char();

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    // Mirror C's stdio: output is flushed as the process exits. A closed pipe
    // is not an error the C program reports, so ignore write failures.
    let _ = driver(&mut out, c);
    let _ = out.flush();
}
