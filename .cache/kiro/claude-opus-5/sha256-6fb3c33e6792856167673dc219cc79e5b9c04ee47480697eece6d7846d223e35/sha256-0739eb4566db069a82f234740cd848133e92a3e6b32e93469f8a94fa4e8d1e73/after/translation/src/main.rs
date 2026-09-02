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

//! Rust translation of `c_src/src/main.c`.
//!
//! Reads a single byte from stdin and reports every `<ctype.h>` classification
//! for it, then its lower- and upper-cased forms.

mod ctype;

use std::io::{Read, Write};

/// `printf("%c", v)` converts the `int` argument to `unsigned char`, so the
/// output is a single raw byte — including NUL, and including bytes that are
/// not valid UTF-8. Output is therefore assembled as bytes, not as a `String`.
fn push_char_line(out: &mut Vec<u8>, label: &str, v: i32) {
    out.extend_from_slice(label.as_bytes());
    out.push(v as u8);
    out.push(b'\n');
}

/// `printf("%d", v)`.
fn push_int_line(out: &mut Vec<u8>, label: &str, v: i32) {
    out.extend_from_slice(label.as_bytes());
    out.extend_from_slice(v.to_string().as_bytes());
    out.push(b'\n');
}

fn driver(c: i8) {
    // setlocale(LC_ALL, "C") in the original: the "C" locale is already what
    // the tables in `ctype` model, so there is nothing to switch to.

    // The C code passes a `char` to functions taking `int`; the default
    // argument promotion sign-extends it. Values 0x80..=0xFF therefore arrive
    // as -128..=-1.
    let c = i32::from(c);

    let mut out = Vec::new();
    push_int_line(&mut out, "alphanumeric: ", ctype::isalnum(c));
    push_int_line(&mut out, "alphabetic: ", ctype::isalpha(c));
    push_int_line(&mut out, "lowercase: ", ctype::islower(c));
    push_int_line(&mut out, "uppercase: ", ctype::isupper(c));
    push_int_line(&mut out, "digit: ", ctype::isdigit(c));
    push_int_line(&mut out, "hexadecimal: ", ctype::isxdigit(c));
    push_int_line(&mut out, "control: ", ctype::iscntrl(c));
    push_int_line(&mut out, "graphical: ", ctype::isgraph(c));
    push_int_line(&mut out, "space: ", ctype::isspace(c));
    push_int_line(&mut out, "blank: ", ctype::isblank(c));
    push_int_line(&mut out, "printing: ", ctype::isprint(c));
    push_int_line(&mut out, "punctuation: ", ctype::ispunct(c));
    push_char_line(&mut out, "to lower: ", ctype::tolower(c));
    push_char_line(&mut out, "to upper: ", ctype::toupper(c));

    // C's stdout is flushed once at exit; a single write matches that.
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let _ = stdout.write_all(&out);
    let _ = stdout.flush();
}

/// `getchar()`: one byte from stdin, or `EOF` (-1) at end of input.
fn getchar() -> i32 {
    let mut b = [0u8; 1];
    match std::io::stdin().read(&mut b) {
        Ok(1) => i32::from(b[0]),
        _ => -1,
    }
}

fn main() {
    // `char c = getchar();` — `char` is signed here, so EOF stays -1 and bytes
    // above 0x7F wrap to negative values. The truncation is deliberate.
    let c = getchar() as i8;
    driver(c);
}
