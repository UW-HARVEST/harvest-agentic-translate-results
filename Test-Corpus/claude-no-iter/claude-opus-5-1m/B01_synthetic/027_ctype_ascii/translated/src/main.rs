// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Produces byte-identical output to the
// original C program. The C program prints the raw integer values
// returned by the glibc ctype.h *macros* (e.g. `isdigit`), which on
// glibc expand to a lookup into `__ctype_b_loc()` masked by a class
// bit. Those values are not 0/1 — for example `isdigit('5')` evaluates
// to 2048 (`_ISdigit`) — so we must replicate the exact macro
// behavior, not call the libc functions of the same names.

use std::io::{self, Read, Write};
use std::os::raw::{c_char, c_int};

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;

    // glibc-internal locale-table accessors used by the ctype macros.
    fn __ctype_b_loc() -> *mut *const u16;
    fn __ctype_tolower_loc() -> *mut *const i32;
    fn __ctype_toupper_loc() -> *mut *const i32;
}

// LC_ALL value on glibc/Linux
const LC_ALL: c_int = 6;

// glibc ctype class bits (see <ctype.h>: enum with `_ISbit(n)`).
const IS_UPPER: u16 = 1 << 8; // _ISupper  = (1<<0) << 8 = 256
const IS_LOWER: u16 = 1 << 9; // _ISlower  = (1<<1) << 8 = 512
const IS_ALPHA: u16 = 1 << 10; // _ISalpha  = 1024
const IS_DIGIT: u16 = 1 << 11; // _ISdigit  = 2048
const IS_XDIGIT: u16 = 1 << 12; // _ISxdigit = 4096
const IS_SPACE: u16 = 1 << 13; // _ISspace  = 8192
const IS_PRINT: u16 = 1 << 14; // _ISprint  = 16384
const IS_GRAPH: u16 = 1 << 15; // _ISgraph  = 32768
const IS_BLANK: u16 = 1 << 0; // _ISblank  = 1
const IS_CNTRL: u16 = 1 << 1; // _IScntrl  = 2
const IS_PUNCT: u16 = 1 << 2; // _ISpunct  = 4
const IS_ALNUM: u16 = 1 << 3; // _ISalnum  = 8

// Look up `c` (interpreted as `int`) in the per-thread ctype-class
// table and mask with `class_bit` — exactly what glibc's `isxxx(c)`
// macros do.
fn ctype_class(c: c_int, class_bit: u16) -> c_int {
    unsafe {
        let table_ptr_ptr = __ctype_b_loc();
        let table = *table_ptr_ptr;
        // The table is indexed by `int`, supporting the range
        // [-128, 256). We need a signed offset so negative chars work.
        let entry = *table.offset(c as isize);
        (entry & class_bit) as c_int
    }
}

fn ctype_tolower(c: c_int) -> c_int {
    unsafe {
        let table = *__ctype_tolower_loc();
        *table.offset(c as isize)
    }
}

fn ctype_toupper(c: c_int) -> c_int {
    unsafe {
        let table = *__ctype_toupper_loc();
        *table.offset(c as isize)
    }
}

// Write a single byte (matches C's `printf("%c", int)`, which writes
// the low 8 bits of the int as one byte).
fn write_byte(stdout: &mut impl Write, val: c_int) {
    let b: u8 = (val & 0xFF) as u8;
    let _ = stdout.write_all(&[b]);
}

fn driver(c: c_int) {
    // setlocale(LC_ALL, "C");
    unsafe {
        let loc = b"C\0";
        setlocale(LC_ALL, loc.as_ptr() as *const c_char);
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();

    let _ = write!(out, "alphanumeric: {}\n", ctype_class(c, IS_ALNUM));
    let _ = write!(out, "alphabetic: {}\n", ctype_class(c, IS_ALPHA));
    let _ = write!(out, "lowercase: {}\n", ctype_class(c, IS_LOWER));
    let _ = write!(out, "uppercase: {}\n", ctype_class(c, IS_UPPER));
    let _ = write!(out, "digit: {}\n", ctype_class(c, IS_DIGIT));
    let _ = write!(out, "hexadecimal: {}\n", ctype_class(c, IS_XDIGIT));
    let _ = write!(out, "control: {}\n", ctype_class(c, IS_CNTRL));
    let _ = write!(out, "graphical: {}\n", ctype_class(c, IS_GRAPH));
    let _ = write!(out, "space: {}\n", ctype_class(c, IS_SPACE));
    let _ = write!(out, "blank: {}\n", ctype_class(c, IS_BLANK));
    let _ = write!(out, "printing: {}\n", ctype_class(c, IS_PRINT));
    let _ = write!(out, "punctuation: {}\n", ctype_class(c, IS_PUNCT));

    let _ = write!(out, "to lower: ");
    write_byte(&mut out, ctype_tolower(c));
    let _ = out.write_all(b"\n");

    let _ = write!(out, "to upper: ");
    write_byte(&mut out, ctype_toupper(c));
    let _ = out.write_all(b"\n");

    let _ = out.flush();
}

fn main() {
    // Mirror C's `char c = getchar();` semantics. `getchar()` returns
    // an `int`: -1 (EOF) or an unsigned byte 0..=255. Storing that
    // result into `char` narrows it; on x86_64 Linux `char` is signed,
    // so bytes 128..=255 sign-extend to negative ints when promoted
    // back to `int` for the ctype calls. EOF -> -1 -> char(-1) ->
    // int(-1).
    let mut buf = [0u8; 1];
    let read_result = io::stdin().lock().read(&mut buf);

    let c_int_val: c_int = match read_result {
        Ok(0) => -1, // EOF
        Ok(_) => (buf[0] as i8) as c_int,
        Err(_) => -1,
    };

    driver(c_int_val);
}
