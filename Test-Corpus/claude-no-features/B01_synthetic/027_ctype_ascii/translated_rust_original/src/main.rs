// Translated from c_src/src/main.c
//
// The original C program reads a single character from stdin and prints
// the results of the ctype.h classification functions and the
// tolower/toupper conversions in the C locale.
//
// Notes on byte-identical output:
// - `getchar()` returns `int`; on EOF, the value is -1. The C source then
//   assigns it to `char c`. On the typical x86_64 Linux ABI, `char` is
//   signed, so EOF becomes the signed `char` value -1 (which round-trips
//   back to int -1 when passed to ctype/printf).
// - For high-bit bytes (0x80..=0xFF), `char c = getchar()` produces a
//   negative signed char. glibc's ctype functions return 0 for those
//   values, and tolower/toupper return the same negative value.
// - When printf prints a negative int with `%c`, it is converted to
//   `unsigned char`, so -1 prints as byte 0xFF and -128 as 0x80.
// - The classification functions in glibc (in the C locale) return the
//   raw bitmask from `__ctype_b`, not just 0/1. We replicate those exact
//   integer return values.

use std::io::{self, Read, Write};

// glibc <ctype.h> bitmask values (from /usr/include/ctype.h via _ISbit).
// These are the values returned by isalnum/isalpha/etc. in the C locale
// on glibc systems. Reproducing them matches the C program's exact output.
const IS_UPPER: i32 = 1 << 8; // 256
const IS_LOWER: i32 = 1 << 9; // 512
const IS_ALPHA: i32 = 1 << 10; // 1024
const IS_DIGIT: i32 = 1 << 11; // 2048
const IS_XDIGIT: i32 = 1 << 12; // 4096
const IS_SPACE: i32 = 1 << 13; // 8192
const IS_PRINT: i32 = 1 << 14; // 16384
const IS_GRAPH: i32 = 1 << 15; // 32768
const IS_BLANK: i32 = 1 << 0; // 1
const IS_CNTRL: i32 = 1 << 1; // 2
const IS_PUNCT: i32 = 1 << 2; // 4
const IS_ALNUM: i32 = 1 << 3; // 8

/// Return the glibc ctype bitmask for byte `b` in the C locale, or 0 for
/// values outside 0..=255 (i.e. EOF).
fn ctype_mask(c: i32) -> i32 {
    // Only bytes 0..=255 have a nonzero mask. EOF (-1) returns 0.
    if !(0..=255).contains(&c) {
        return 0;
    }
    let b = c as u8;
    let mut m = 0;

    let is_upper = (b'A'..=b'Z').contains(&b);
    let is_lower = (b'a'..=b'z').contains(&b);
    let is_digit = (b'0'..=b'9').contains(&b);
    let is_xdigit = is_digit || (b'A'..=b'F').contains(&b) || (b'a'..=b'f').contains(&b);

    if is_upper {
        m |= IS_UPPER;
    }
    if is_lower {
        m |= IS_LOWER;
    }
    if is_upper || is_lower {
        m |= IS_ALPHA;
    }
    if is_digit {
        m |= IS_DIGIT;
    }
    if is_xdigit {
        m |= IS_XDIGIT;
    }
    if is_upper || is_lower || is_digit {
        m |= IS_ALNUM;
    }

    // space: \t \n \v \f \r and ' '
    let is_space = matches!(b, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r');
    if is_space {
        m |= IS_SPACE;
    }
    // blank: ' ' and \t
    if matches!(b, b' ' | b'\t') {
        m |= IS_BLANK;
    }

    // cntrl: 0..=31 and 127
    if b <= 31 || b == 127 {
        m |= IS_CNTRL;
    }
    // print: 0x20..=0x7E
    if (0x20..=0x7E).contains(&b) {
        m |= IS_PRINT;
    }
    // graph: 0x21..=0x7E
    if (0x21..=0x7E).contains(&b) {
        m |= IS_GRAPH;
    }
    // punct: printable, not space, not alnum
    if (0x21..=0x7E).contains(&b) && !is_upper && !is_lower && !is_digit {
        m |= IS_PUNCT;
    }

    m
}

fn c_isalnum(c: i32) -> i32 {
    ctype_mask(c) & IS_ALNUM
}
fn c_isalpha(c: i32) -> i32 {
    ctype_mask(c) & IS_ALPHA
}
fn c_islower(c: i32) -> i32 {
    ctype_mask(c) & IS_LOWER
}
fn c_isupper(c: i32) -> i32 {
    ctype_mask(c) & IS_UPPER
}
fn c_isdigit(c: i32) -> i32 {
    ctype_mask(c) & IS_DIGIT
}
fn c_isxdigit(c: i32) -> i32 {
    ctype_mask(c) & IS_XDIGIT
}
fn c_iscntrl(c: i32) -> i32 {
    ctype_mask(c) & IS_CNTRL
}
fn c_isgraph(c: i32) -> i32 {
    ctype_mask(c) & IS_GRAPH
}
fn c_isspace(c: i32) -> i32 {
    ctype_mask(c) & IS_SPACE
}
fn c_isblank(c: i32) -> i32 {
    ctype_mask(c) & IS_BLANK
}
fn c_isprint(c: i32) -> i32 {
    ctype_mask(c) & IS_PRINT
}
fn c_ispunct(c: i32) -> i32 {
    ctype_mask(c) & IS_PUNCT
}

/// Replicates glibc tolower(c) in the C locale.
/// - For ASCII A-Z, returns the lowercase letter.
/// - For all other values in the int range, returns `c` unchanged.
fn c_tolower(c: i32) -> i32 {
    if (b'A' as i32..=b'Z' as i32).contains(&c) {
        c + 32
    } else {
        c
    }
}

/// Replicates glibc toupper(c) in the C locale.
fn c_toupper(c: i32) -> i32 {
    if (b'a' as i32..=b'z' as i32).contains(&c) {
        c - 32
    } else {
        c
    }
}

/// Convert an `i32` to the byte that printf("%c", x) would emit:
/// the value cast to `unsigned char`.
fn percent_c_byte(c: i32) -> u8 {
    (c as u32 & 0xFF) as u8
}

fn driver<W: Write>(out: &mut W, c: i32) -> io::Result<()> {
    // setlocale(LC_ALL, "C") in the original; the Rust translation always
    // uses the C-locale classification tables, so this is implicit.

    writeln!(out, "alphanumeric: {}", c_isalnum(c))?;
    writeln!(out, "alphabetic: {}", c_isalpha(c))?;
    writeln!(out, "lowercase: {}", c_islower(c))?;
    writeln!(out, "uppercase: {}", c_isupper(c))?;
    writeln!(out, "digit: {}", c_isdigit(c))?;
    writeln!(out, "hexadecimal: {}", c_isxdigit(c))?;
    writeln!(out, "control: {}", c_iscntrl(c))?;
    writeln!(out, "graphical: {}", c_isgraph(c))?;
    writeln!(out, "space: {}", c_isspace(c))?;
    writeln!(out, "blank: {}", c_isblank(c))?;
    writeln!(out, "printing: {}", c_isprint(c))?;
    writeln!(out, "punctuation: {}", c_ispunct(c))?;

    // printf("to lower: %c\n", tolower(c))
    let lo = percent_c_byte(c_tolower(c));
    out.write_all(b"to lower: ")?;
    out.write_all(&[lo])?;
    out.write_all(b"\n")?;

    let up = percent_c_byte(c_toupper(c));
    out.write_all(b"to upper: ")?;
    out.write_all(&[up])?;
    out.write_all(b"\n")?;

    Ok(())
}

fn main() {
    // Replicate `char c = getchar();`
    //   - getchar() returns int (-1 for EOF, otherwise 0..=255).
    //   - The result is then truncated to `char`. On the standard
    //     x86_64 Linux ABI `char` is signed, so 0x80..=0xFF map to
    //     -128..=-1, and EOF stays -1 after the round-trip through char.
    //   - When passed to ctype/tolower/toupper, the signed char is then
    //     sign-extended back to int.
    let mut buf = [0u8; 1];
    let getchar_result: i32 = match io::stdin().read(&mut buf) {
        Ok(0) => -1,             // EOF
        Ok(_) => buf[0] as i32,  // 0..=255
        Err(_) => -1,            // treat read errors like EOF (matches getchar on hard error)
    };

    // char c = getchar_result;  -> truncate to signed 8-bit, then sign-extend.
    let c_as_signed_char = getchar_result as i8;
    let c_as_int = c_as_signed_char as i32;

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(&mut out, c_as_int).expect("write to stdout failed");
}
