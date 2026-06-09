// Translated from c_src/src/driver.c
// The original C provides a `driver(char c)` library function. This Rust
// executable wraps it with a main that reads a single character from stdin
// (via libc getchar, matching C's stdin behavior byte-for-byte) and invokes
// the driver. The ctype function outputs use libc directly so that values
// match glibc's exact bit-mask values byte-identically.

use std::hint::black_box;
use std::io::Write;

extern "C" {
    fn setlocale(category: libc::c_int, locale: *const libc::c_char) -> *mut libc::c_char;
    fn isalnum(c: libc::c_int) -> libc::c_int;
    fn isalpha(c: libc::c_int) -> libc::c_int;
    fn islower(c: libc::c_int) -> libc::c_int;
    fn isupper(c: libc::c_int) -> libc::c_int;
    fn isdigit(c: libc::c_int) -> libc::c_int;
    fn isxdigit(c: libc::c_int) -> libc::c_int;
    fn iscntrl(c: libc::c_int) -> libc::c_int;
    fn isgraph(c: libc::c_int) -> libc::c_int;
    fn isspace(c: libc::c_int) -> libc::c_int;
    fn isblank(c: libc::c_int) -> libc::c_int;
    fn isprint(c: libc::c_int) -> libc::c_int;
    fn ispunct(c: libc::c_int) -> libc::c_int;
    fn tolower(c: libc::c_int) -> libc::c_int;
    fn toupper(c: libc::c_int) -> libc::c_int;
    fn getchar() -> libc::c_int;
}

// Wrap each ctype call so LLVM cannot constant-fold or replace it with a
// simplified implementation; we need glibc's exact bit-mask return values.
type CtypeFn = unsafe extern "C" fn(libc::c_int) -> libc::c_int;

#[inline(never)]
fn opaque_call(f: CtypeFn, c: libc::c_int) -> libc::c_int {
    let f = black_box(f);
    unsafe { f(c) }
}

fn driver(c: libc::c_char) {
    // setlocale(LC_ALL, "C");
    let c_str = b"C\0";
    unsafe {
        setlocale(libc::LC_ALL, c_str.as_ptr() as *const libc::c_char);
    }

    // C passes `char` to the int-taking ctype functions; this performs the
    // default integer promotion. On platforms where char is signed, negative
    // values are passed through (which is undefined behaviour in C but
    // consistent with what the original program does).
    let ic: libc::c_int = c as libc::c_int;

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "alphanumeric: {}", opaque_call(isalnum, ic));
    let _ = writeln!(out, "alphabetic: {}", opaque_call(isalpha, ic));
    let _ = writeln!(out, "lowercase: {}", opaque_call(islower, ic));
    let _ = writeln!(out, "uppercase: {}", opaque_call(isupper, ic));
    let _ = writeln!(out, "digit: {}", opaque_call(isdigit, ic));
    let _ = writeln!(out, "hexadecimal: {}", opaque_call(isxdigit, ic));
    let _ = writeln!(out, "control: {}", opaque_call(iscntrl, ic));
    let _ = writeln!(out, "graphical: {}", opaque_call(isgraph, ic));
    let _ = writeln!(out, "space: {}", opaque_call(isspace, ic));
    let _ = writeln!(out, "blank: {}", opaque_call(isblank, ic));
    let _ = writeln!(out, "printing: {}", opaque_call(isprint, ic));
    let _ = writeln!(out, "punctuation: {}", opaque_call(ispunct, ic));
    // %c prints the low byte of the int as a character. Match by writing
    // a single raw byte rather than relying on UTF-8 char formatting.
    let lo = (opaque_call(tolower, ic) & 0xff) as u8;
    let up = (opaque_call(toupper, ic) & 0xff) as u8;
    let _ = out.write_all(b"to lower: ");
    let _ = out.write_all(&[lo]);
    let _ = out.write_all(b"\n");
    let _ = out.write_all(b"to upper: ");
    let _ = out.write_all(&[up]);
    let _ = out.write_all(b"\n");
}

fn main() {
    // Read a single byte from stdin via libc getchar to match C's stdin
    // behaviour. If EOF, fall back to char 0 — but the original C program
    // has no main, so this is our reasonable executable wrapper.
    let ch = unsafe { getchar() };
    let c: libc::c_char = if ch == libc::EOF {
        0
    } else {
        ch as libc::c_char
    };
    driver(c);
}
