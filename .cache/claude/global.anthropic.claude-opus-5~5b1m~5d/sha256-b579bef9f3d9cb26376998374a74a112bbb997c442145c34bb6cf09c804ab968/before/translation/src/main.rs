// Rust translation of c_src/src/main.c
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

use std::io::{Read, Write};

// glibc <ctype.h> classification bits (_ISbit values), which are what the
// is*() macros/functions actually return (the masked table entry, not 1).
//
//   _ISbit(bit) = (bit) < 8 ? ((1 << (bit)) << 8) : ((1 << (bit)) >> 8)
const IS_UPPER: i32 = 1 << 8; // 256
const IS_LOWER: i32 = 2 << 8; // 512
const IS_ALPHA: i32 = 4 << 8; // 1024
const IS_DIGIT: i32 = 8 << 8; // 2048
const IS_XDIGIT: i32 = 16 << 8; // 4096
const IS_SPACE: i32 = 32 << 8; // 8192
const IS_PRINT: i32 = 64 << 8; // 16384
const IS_GRAPH: i32 = 128 << 8; // 32768
const IS_BLANK: i32 = (1 << 8) >> 8; // 1
const IS_CNTRL: i32 = (1 << 9) >> 8; // 2
const IS_PUNCT: i32 = (1 << 10) >> 8; // 4
const IS_ALNUM: i32 = (1 << 11) >> 8; // 8

/// The `__ctype_b` table entry for the "C" locale, for the index range that a
/// (signed) `char` can produce: -128..=127.  In the "C" locale the negative
/// half of the table (indices -128..=-1) is all zeroes.
fn ctype_b(c: i8) -> i32 {
    if c < 0 {
        return 0;
    }
    let b = c as u8;
    let mut m = 0i32;

    let is_upper = b.is_ascii_uppercase();
    let is_lower = b.is_ascii_lowercase();
    let is_digit = b.is_ascii_digit();
    let is_alpha = is_upper || is_lower;
    let is_alnum = is_alpha || is_digit;
    let is_graph = (0x21..=0x7e).contains(&b);
    let is_print = (0x20..=0x7e).contains(&b);
    let is_space = b == b' ' || (0x09..=0x0d).contains(&b);
    let is_blank = b == b' ' || b == 0x09;
    let is_cntrl = b <= 0x1f || b == 0x7f;
    let is_xdigit = is_digit || (b'a'..=b'f').contains(&b) || (b'A'..=b'F').contains(&b);

    if is_upper {
        m |= IS_UPPER;
    }
    if is_lower {
        m |= IS_LOWER;
    }
    if is_alpha {
        m |= IS_ALPHA;
    }
    if is_digit {
        m |= IS_DIGIT;
    }
    if is_xdigit {
        m |= IS_XDIGIT;
    }
    if is_space {
        m |= IS_SPACE;
    }
    if is_print {
        m |= IS_PRINT;
    }
    if is_graph {
        m |= IS_GRAPH;
    }
    if is_blank {
        m |= IS_BLANK;
    }
    if is_cntrl {
        m |= IS_CNTRL;
    }
    if is_graph && !is_alnum {
        m |= IS_PUNCT;
    }
    if is_alnum {
        m |= IS_ALNUM;
    }
    m
}

fn isalnum(c: i8) -> i32 {
    ctype_b(c) & IS_ALNUM
}
fn isalpha(c: i8) -> i32 {
    ctype_b(c) & IS_ALPHA
}
fn islower(c: i8) -> i32 {
    ctype_b(c) & IS_LOWER
}
fn isupper(c: i8) -> i32 {
    ctype_b(c) & IS_UPPER
}
fn isdigit(c: i8) -> i32 {
    ctype_b(c) & IS_DIGIT
}
fn isxdigit(c: i8) -> i32 {
    ctype_b(c) & IS_XDIGIT
}
fn iscntrl(c: i8) -> i32 {
    ctype_b(c) & IS_CNTRL
}
fn isgraph(c: i8) -> i32 {
    ctype_b(c) & IS_GRAPH
}
fn isspace(c: i8) -> i32 {
    ctype_b(c) & IS_SPACE
}
fn isblank(c: i8) -> i32 {
    ctype_b(c) & IS_BLANK
}
fn isprint(c: i8) -> i32 {
    ctype_b(c) & IS_PRINT
}
fn ispunct(c: i8) -> i32 {
    ctype_b(c) & IS_PUNCT
}

/// glibc `__ctype_tolower` table for the "C" locale over indices -128..=127.
/// Index -1 (EOF) maps to -1; indices -128..=-2 map to index + 256; indices
/// 0..=127 use the ASCII mapping.
fn tolower(c: i8) -> i32 {
    let i = c as i32;
    if i < 0 {
        if i == -1 {
            -1
        } else {
            i + 256
        }
    } else {
        let b = c as u8;
        if b.is_ascii_uppercase() {
            (b + 32) as i32
        } else {
            i
        }
    }
}

fn toupper(c: i8) -> i32 {
    let i = c as i32;
    if i < 0 {
        if i == -1 {
            -1
        } else {
            i + 256
        }
    } else {
        let b = c as u8;
        if b.is_ascii_lowercase() {
            (b - 32) as i32
        } else {
            i
        }
    }
}

/// Emits `printf("<label>: %d\n", value)`.
fn print_int(out: &mut Vec<u8>, label: &str, value: i32) {
    out.extend_from_slice(label.as_bytes());
    out.extend_from_slice(b": ");
    out.extend_from_slice(value.to_string().as_bytes());
    out.push(b'\n');
}

/// Emits `printf("<label>: %c\n", value)`; `%c` writes `(unsigned char)value`.
fn print_char(out: &mut Vec<u8>, label: &str, value: i32) {
    out.extend_from_slice(label.as_bytes());
    out.extend_from_slice(b": ");
    out.push((value as u32 & 0xff) as u8);
    out.push(b'\n');
}

fn driver(c: i8) {
    // setlocale(LC_ALL, "C") -- the "C" locale is what the tables above model.

    let mut out: Vec<u8> = Vec::new();
    print_int(&mut out, "alphanumeric", isalnum(c));
    print_int(&mut out, "alphabetic", isalpha(c));
    print_int(&mut out, "lowercase", islower(c));
    print_int(&mut out, "uppercase", isupper(c));
    print_int(&mut out, "digit", isdigit(c));
    print_int(&mut out, "hexadecimal", isxdigit(c));
    print_int(&mut out, "control", iscntrl(c));
    print_int(&mut out, "graphical", isgraph(c));
    print_int(&mut out, "space", isspace(c));
    print_int(&mut out, "blank", isblank(c));
    print_int(&mut out, "printing", isprint(c));
    print_int(&mut out, "punctuation", ispunct(c));
    print_char(&mut out, "to lower", tolower(c));
    print_char(&mut out, "to upper", toupper(c));

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(&out);
    let _ = lock.flush();
}

/// getchar(): returns the next byte from stdin as an int, or EOF (-1).
fn getchar() -> i32 {
    let mut buf = [0u8; 1];
    match std::io::stdin().read(&mut buf) {
        Ok(1) => buf[0] as i32,
        _ => -1,
    }
}

fn main() {
    let c = getchar() as i8; // char c = getchar();
    driver(c);
}
