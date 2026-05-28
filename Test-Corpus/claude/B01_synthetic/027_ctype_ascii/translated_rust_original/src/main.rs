// Translation of c_src/src/main.c to Rust.
//
// The original C code includes <ctype.h>, where each is*() name expands to a
// macro that indexes the per-thread classification table returned by
// __ctype_b_loc() and masks against the appropriate _IS* bit. The macro
// expansion returns the masked bitmask value (e.g. isdigit('5') == 2048),
// not 0/1. We replicate that exact behavior here using direct FFI to glibc
// internals so the output is byte-identical to the C build.

use std::ffi::CString;
use std::io::{self, Read, Write};

#[link(name = "c")]
extern "C" {
    fn setlocale(category: i32, locale: *const i8) -> *mut i8;

    // Pointers to per-thread (or shared) classification / case-mapping tables.
    fn __ctype_b_loc() -> *mut *const u16;
    fn __ctype_tolower_loc() -> *mut *const i32;
    fn __ctype_toupper_loc() -> *mut *const i32;
}

const LC_ALL: i32 = 6;

// Masks corresponding to the _IS* enum in glibc bits/types.h.
// _ISbit(b) = (b < 8) ? (1 << b) << 8 : (1 << b) >> 8
const IS_UPPER: u16 = 0x0100; // bit 0
const IS_LOWER: u16 = 0x0200; // bit 1
const IS_ALPHA: u16 = 0x0400; // bit 2
const IS_DIGIT: u16 = 0x0800; // bit 3
const IS_XDIGIT: u16 = 0x1000; // bit 4
const IS_SPACE: u16 = 0x2000; // bit 5
const IS_PRINT: u16 = 0x4000; // bit 6
const IS_GRAPH: u16 = 0x8000; // bit 7
const IS_BLANK: u16 = 0x0001; // bit 8
const IS_CNTRL: u16 = 0x0002; // bit 9
const IS_PUNCT: u16 = 0x0004; // bit 10
const IS_ALNUM: u16 = 0x0008; // bit 11

/// Return the classification flags for `c` from the glibc table. The table is
/// indexable from -1..=255; for the typical signed-char case we replicate the
/// macro expansion exactly.
unsafe fn ctype_lookup(c: i32) -> u16 {
    let tbl_ptr = *__ctype_b_loc();
    // tbl_ptr points at index 0; the table is valid for indices -1..=255.
    *tbl_ptr.offset(c as isize)
}

unsafe fn ctype_tolower(c: i32) -> i32 {
    let tbl_ptr = *__ctype_tolower_loc();
    *tbl_ptr.offset(c as isize)
}

unsafe fn ctype_toupper(c: i32) -> i32 {
    let tbl_ptr = *__ctype_toupper_loc();
    *tbl_ptr.offset(c as isize)
}

fn driver(c: i8) {
    // setlocale(LC_ALL, "C");
    let lc = CString::new("C").unwrap();
    unsafe {
        setlocale(LC_ALL, lc.as_ptr());
    }

    // In C, the `char c` parameter is promoted to int when passed to is*()
    // macros. On platforms where char is signed (the typical case for the
    // original C code), values like 0xFF become -1 after promotion.
    let ci: i32 = c as i32;

    let stdout = io::stdout();
    let mut out = stdout.lock();

    unsafe {
        let flags = ctype_lookup(ci);
        let _ = writeln!(out, "alphanumeric: {}", flags & IS_ALNUM);
        let _ = writeln!(out, "alphabetic: {}", flags & IS_ALPHA);
        let _ = writeln!(out, "lowercase: {}", flags & IS_LOWER);
        let _ = writeln!(out, "uppercase: {}", flags & IS_UPPER);
        let _ = writeln!(out, "digit: {}", flags & IS_DIGIT);
        let _ = writeln!(out, "hexadecimal: {}", flags & IS_XDIGIT);
        let _ = writeln!(out, "control: {}", flags & IS_CNTRL);
        let _ = writeln!(out, "graphical: {}", flags & IS_GRAPH);
        let _ = writeln!(out, "space: {}", flags & IS_SPACE);
        let _ = writeln!(out, "blank: {}", flags & IS_BLANK);
        let _ = writeln!(out, "printing: {}", flags & IS_PRINT);
        let _ = writeln!(out, "punctuation: {}", flags & IS_PUNCT);

        // tolower/toupper return int; %c prints the low byte as a single byte.
        let lower = ctype_tolower(ci);
        let upper = ctype_toupper(ci);
        let lower_byte = (lower & 0xff) as u8;
        let upper_byte = (upper & 0xff) as u8;
        let _ = out.write_all(b"to lower: ");
        let _ = out.write_all(&[lower_byte]);
        let _ = out.write_all(b"\n");
        let _ = out.write_all(b"to upper: ");
        let _ = out.write_all(&[upper_byte]);
        let _ = out.write_all(b"\n");
    }
}

fn main() {
    // Replicate `char c = getchar();` — getchar() returns int. On EOF, it
    // returns -1, and assigning to `char` in C truncates to the low byte
    // (0xFF), which on signed-char platforms is -1.
    let mut buf = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let n = handle.read(&mut buf).unwrap_or(0);
    let c: i8 = if n == 0 {
        // EOF: getchar() returned -1, truncated to char.
        -1i8
    } else {
        buf[0] as i8
    };
    driver(c);
}
