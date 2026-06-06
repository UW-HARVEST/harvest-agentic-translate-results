use std::io::{Read, Write};
use std::os::raw::{c_char, c_int, c_ushort};

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn __ctype_b_loc() -> *mut *const c_ushort;
    fn __ctype_tolower_loc() -> *mut *const c_int;
    fn __ctype_toupper_loc() -> *mut *const c_int;
}

const LC_ALL: c_int = 6; // glibc value

// glibc ctype bitmasks (from <ctype.h>)
const _ISUPPER: c_int = 256;
const _ISLOWER: c_int = 512;
const _ISALPHA: c_int = 1024;
const _ISDIGIT: c_int = 2048;
const _ISXDIGIT: c_int = 4096;
const _ISSPACE: c_int = 8192;
const _ISPRINT: c_int = 16384;
const _ISGRAPH: c_int = 32768;
const _ISBLANK: c_int = 1;
const _ISCNTRL: c_int = 2;
const _ISPUNCT: c_int = 4;
const _ISALNUM: c_int = 8;

unsafe fn ctype_lookup(c: c_int) -> c_int {
    // glibc macros do (*__ctype_b_loc())[(int)c]; the table is offset by 128
    // entries so that values -128..255 are valid indices.
    let table = *__ctype_b_loc();
    *table.offset(c as isize) as c_int
}

unsafe fn rs_isalnum(c: c_int) -> c_int { ctype_lookup(c) & _ISALNUM }
unsafe fn rs_isalpha(c: c_int) -> c_int { ctype_lookup(c) & _ISALPHA }
unsafe fn rs_islower(c: c_int) -> c_int { ctype_lookup(c) & _ISLOWER }
unsafe fn rs_isupper(c: c_int) -> c_int { ctype_lookup(c) & _ISUPPER }
unsafe fn rs_isdigit(c: c_int) -> c_int { ctype_lookup(c) & _ISDIGIT }
unsafe fn rs_isxdigit(c: c_int) -> c_int { ctype_lookup(c) & _ISXDIGIT }
unsafe fn rs_iscntrl(c: c_int) -> c_int { ctype_lookup(c) & _ISCNTRL }
unsafe fn rs_isgraph(c: c_int) -> c_int { ctype_lookup(c) & _ISGRAPH }
unsafe fn rs_isspace(c: c_int) -> c_int { ctype_lookup(c) & _ISSPACE }
unsafe fn rs_isblank(c: c_int) -> c_int { ctype_lookup(c) & _ISBLANK }
unsafe fn rs_isprint(c: c_int) -> c_int { ctype_lookup(c) & _ISPRINT }
unsafe fn rs_ispunct(c: c_int) -> c_int { ctype_lookup(c) & _ISPUNCT }

unsafe fn rs_tolower(c: c_int) -> c_int {
    let table = *__ctype_tolower_loc();
    *table.offset(c as isize)
}

unsafe fn rs_toupper(c: c_int) -> c_int {
    let table = *__ctype_toupper_loc();
    *table.offset(c as isize)
}

fn driver(c: c_int, out: &mut impl Write) {
    unsafe {
        let locale = b"C\0";
        setlocale(LC_ALL, locale.as_ptr() as *const c_char);

        write!(out, "alphanumeric: {}\n", rs_isalnum(c)).unwrap();
        write!(out, "alphabetic: {}\n", rs_isalpha(c)).unwrap();
        write!(out, "lowercase: {}\n", rs_islower(c)).unwrap();
        write!(out, "uppercase: {}\n", rs_isupper(c)).unwrap();
        write!(out, "digit: {}\n", rs_isdigit(c)).unwrap();
        write!(out, "hexadecimal: {}\n", rs_isxdigit(c)).unwrap();
        write!(out, "control: {}\n", rs_iscntrl(c)).unwrap();
        write!(out, "graphical: {}\n", rs_isgraph(c)).unwrap();
        write!(out, "space: {}\n", rs_isspace(c)).unwrap();
        write!(out, "blank: {}\n", rs_isblank(c)).unwrap();
        write!(out, "printing: {}\n", rs_isprint(c)).unwrap();
        write!(out, "punctuation: {}\n", rs_ispunct(c)).unwrap();

        // %c in printf writes (unsigned char) of the int argument as a single byte.
        let lower = rs_tolower(c);
        let upper = rs_toupper(c);

        out.write_all(b"to lower: ").unwrap();
        out.write_all(&[(lower as u32 & 0xFF) as u8]).unwrap();
        out.write_all(b"\n").unwrap();

        out.write_all(b"to upper: ").unwrap();
        out.write_all(&[(upper as u32 & 0xFF) as u8]).unwrap();
        out.write_all(b"\n").unwrap();
    }
}

fn main() {
    // Mirror `char c = getchar();` then pass to driver(char c).
    // getchar() returns int (0..255 on success, -1 on EOF).
    // The cast to `char` (signed on x86_64 Linux) and back to int sign-extends.
    let mut buf = [0u8; 1];
    let c_val: c_int = match std::io::stdin().read(&mut buf) {
        Ok(0) => -1,
        Ok(_) => (buf[0] as i8) as c_int,
        Err(_) => -1,
    };

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    driver(c_val, &mut out);
}
