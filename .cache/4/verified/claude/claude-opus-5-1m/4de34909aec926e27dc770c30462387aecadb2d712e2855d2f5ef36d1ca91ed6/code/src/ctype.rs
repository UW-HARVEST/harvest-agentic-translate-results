// Core translation of c_src/src/main.c.
//
// The C program reads a single character with getchar() and stores it in a
// `char` (signed on the reference platform), then reports the result of every
// <ctype.h> classification function for that value, followed by tolower() and
// toupper().
//
// On the reference platform (glibc), the classification macros expand to a
// lookup in the locale's ctype table combined with a bit mask, so they return
// the mask bit value itself (e.g. isalpha('a') == 1024) rather than 1. The
// tables in `tables.rs` are exact copies of glibc's "C" locale tables for the
// index range that a `char` can produce (-128..=127), which is why the entries
// for bytes 0x80..=0xFF (negative `char` values) are all zero.
//
// getchar() returning EOF (-1) yields the same `char` value as the input byte
// 0xFF, so both cases are handled identically -- matching the C code, which
// performs no EOF check.

use std::io::{Read, Write};

use crate::tables::{CTYPE_CLASS, CTYPE_TOLOWER, CTYPE_TOUPPER};

// glibc <ctype.h> bit masks (_ISbit values).
const IS_UPPER: u16 = 1 << 8; // 256
const IS_LOWER: u16 = 2 << 8; // 512
const IS_ALPHA: u16 = 4 << 8; // 1024
const IS_DIGIT: u16 = 8 << 8; // 2048
const IS_XDIGIT: u16 = 16 << 8; // 4096
const IS_SPACE: u16 = 32 << 8; // 8192
const IS_PRINT: u16 = 64 << 8; // 16384
const IS_GRAPH: u16 = 128 << 8; // 32768
const IS_BLANK: u16 = 1; // (1 << 8) >> 8
const IS_CNTRL: u16 = 2; // (1 << 9) >> 8
const IS_PUNCT: u16 = 4; // (1 << 10) >> 8
const IS_ALNUM: u16 = 8; // (1 << 11) >> 8

/// Emulates glibc's `__isctype(c, mask)`: the table entry masked with `mask`,
/// promoted to `int` (always non-negative here).
fn isctype(index: u8, mask: u16) -> i32 {
    i32::from(CTYPE_CLASS[index as usize] & mask)
}

/// The bytes `driver(c)` writes to stdout, for the `char` value `c`.
///
/// The ctype tables are indexed by the (possibly negative) char value; using
/// the equivalent unsigned byte keeps the indexing in bounds and selects the
/// same entries, because glibc's tables repeat 0x80..=0xFF for -128..=-1.
pub fn driver_bytes(c: i8) -> Vec<u8> {
    let index = c as u8;
    let mut out = Vec::with_capacity(256);

    // setlocale(LC_ALL, "C") -- the tables already encode the "C" locale.
    write_line(&mut out, "alphanumeric", isctype(index, IS_ALNUM));
    write_line(&mut out, "alphabetic", isctype(index, IS_ALPHA));
    write_line(&mut out, "lowercase", isctype(index, IS_LOWER));
    write_line(&mut out, "uppercase", isctype(index, IS_UPPER));
    write_line(&mut out, "digit", isctype(index, IS_DIGIT));
    write_line(&mut out, "hexadecimal", isctype(index, IS_XDIGIT));
    write_line(&mut out, "control", isctype(index, IS_CNTRL));
    write_line(&mut out, "graphical", isctype(index, IS_GRAPH));
    write_line(&mut out, "space", isctype(index, IS_SPACE));
    write_line(&mut out, "blank", isctype(index, IS_BLANK));
    write_line(&mut out, "printing", isctype(index, IS_PRINT));
    write_line(&mut out, "punctuation", isctype(index, IS_PUNCT));
    write_char_line(&mut out, "to lower", CTYPE_TOLOWER[index as usize]);
    write_char_line(&mut out, "to upper", CTYPE_TOUPPER[index as usize]);

    out
}

/// `void driver(char c)`
pub fn driver(c: i8) {
    let out = driver_bytes(c);
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(&out);
    let _ = handle.flush();
}

/// printf("<label>: %d\n", value)
fn write_line(out: &mut Vec<u8>, label: &str, value: i32) {
    out.extend_from_slice(label.as_bytes());
    out.extend_from_slice(b": ");
    out.extend_from_slice(value.to_string().as_bytes());
    out.push(b'\n');
}

/// printf("<label>: %c\n", value) -- %c writes the low byte of the int.
fn write_char_line(out: &mut Vec<u8>, label: &str, byte: u8) {
    out.extend_from_slice(label.as_bytes());
    out.extend_from_slice(b": ");
    out.push(byte);
    out.push(b'\n');
}

/// getchar(): the next byte of stdin, or EOF (-1) at end of input / on error.
pub fn getchar() -> i32 {
    let mut buf = [0u8; 1];
    match std::io::stdin().read(&mut buf) {
        Ok(1) => i32::from(buf[0]),
        _ => -1,
    }
}

/// `int main()` -- reads one character and runs the driver. The C function
/// falls off the end of its body, so it returns 0.
pub fn c_main() -> i32 {
    // char c = getchar();  -- narrowing conversion to a signed char.
    let c = getchar() as i8;
    driver(c);
    0
}
