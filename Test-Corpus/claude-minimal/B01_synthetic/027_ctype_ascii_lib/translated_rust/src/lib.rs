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

use std::os::raw::c_char;

// C-locale equivalents of <ctype.h> functions. They operate on the
// unsigned-char value of `c` and return 1 (true) or 0 (false), matching
// the semantics specified by the C standard for the "C" locale.

fn c_isalnum(c: u8) -> i32 {
    if c.is_ascii_alphanumeric() { 1 } else { 0 }
}

fn c_isalpha(c: u8) -> i32 {
    if c.is_ascii_alphabetic() { 1 } else { 0 }
}

fn c_islower(c: u8) -> i32 {
    if c.is_ascii_lowercase() { 1 } else { 0 }
}

fn c_isupper(c: u8) -> i32 {
    if c.is_ascii_uppercase() { 1 } else { 0 }
}

fn c_isdigit(c: u8) -> i32 {
    if c.is_ascii_digit() { 1 } else { 0 }
}

fn c_isxdigit(c: u8) -> i32 {
    if c.is_ascii_hexdigit() { 1 } else { 0 }
}

fn c_iscntrl(c: u8) -> i32 {
    if c.is_ascii_control() { 1 } else { 0 }
}

fn c_isgraph(c: u8) -> i32 {
    if c.is_ascii_graphic() { 1 } else { 0 }
}

fn c_isspace(c: u8) -> i32 {
    // C standard: space, \t, \n, \v, \f, \r
    match c {
        b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r' => 1,
        _ => 0,
    }
}

fn c_isblank(c: u8) -> i32 {
    match c {
        b' ' | b'\t' => 1,
        _ => 0,
    }
}

fn c_isprint(c: u8) -> i32 {
    // Printable: graphic chars plus space.
    if c == b' ' || c.is_ascii_graphic() { 1 } else { 0 }
}

fn c_ispunct(c: u8) -> i32 {
    if c.is_ascii_punctuation() { 1 } else { 0 }
}

fn c_tolower(c: u8) -> u8 {
    c.to_ascii_lowercase()
}

fn c_toupper(c: u8) -> u8 {
    c.to_ascii_uppercase()
}

/// Public Rust API matching the original C library entry point.
///
/// Exposed with the `driver` symbol via `extern "C"` so that this Rust
/// crate can be used as a drop-in replacement for the C shared library.
#[no_mangle]
pub extern "C" fn driver(c: c_char) {
    // The original C code calls setlocale(LC_ALL, "C"); we always operate
    // in the C locale here, so no locale change is necessary.

    // Mirror C's behavior of converting `char` to its unsigned-char value
    // before passing to <ctype.h> functions.
    let uc: u8 = c as u8;

    println!("alphanumeric: {}", c_isalnum(uc));
    println!("alphabetic: {}", c_isalpha(uc));
    println!("lowercase: {}", c_islower(uc));
    println!("uppercase: {}", c_isupper(uc));
    println!("digit: {}", c_isdigit(uc));
    println!("hexadecimal: {}", c_isxdigit(uc));
    println!("control: {}", c_iscntrl(uc));
    println!("graphical: {}", c_isgraph(uc));
    println!("space: {}", c_isspace(uc));
    println!("blank: {}", c_isblank(uc));
    println!("printing: {}", c_isprint(uc));
    println!("punctuation: {}", c_ispunct(uc));
    println!("to lower: {}", c_tolower(uc) as char);
    println!("to upper: {}", c_toupper(uc) as char);
}
