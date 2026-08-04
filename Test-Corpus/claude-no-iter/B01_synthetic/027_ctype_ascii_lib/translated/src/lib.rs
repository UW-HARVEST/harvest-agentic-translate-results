use std::ffi::CString;
use std::os::raw::{c_char, c_int};

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn tolower(c: c_int) -> c_int;
    fn toupper(c: c_int) -> c_int;

    // glibc-internal: returns a thread-local pointer to a 384-entry table of
    // unsigned shorts (indexable by signed char [-128,255] or EOF). The
    // ctype.h macros (isalpha, isdigit, ...) expand to a bitwise AND between
    // an entry of this table and a class bit, which yields the bit value
    // itself (e.g. 2048 for _ISdigit) rather than 0/1. This is exactly what
    // the C source observes because it includes <ctype.h>.
    fn __ctype_b_loc() -> *mut *const u16;
}

const LC_ALL: c_int = 6; // glibc's LC_ALL value (matches libc::LC_ALL)

// glibc ctype class bits (see <ctype.h>'s `enum` and `_ISbit`).
// On little-endian glibc:  bits  < 8  -> (1 << bit) << 8
//                          bits >= 8  -> (1 << bit) >> 8
const _ISUPPER: u16  = (1 << 0) << 8;   // 256
const _ISLOWER: u16  = (1 << 1) << 8;   // 512
const _ISALPHA: u16  = (1 << 2) << 8;   // 1024
const _ISDIGIT: u16  = (1 << 3) << 8;   // 2048
const _ISXDIGIT: u16 = (1 << 4) << 8;   // 4096
const _ISSPACE: u16  = (1 << 5) << 8;   // 8192
const _ISPRINT: u16  = (1 << 6) << 8;   // 16384
const _ISGRAPH: u16  = (1 << 7) << 8;   // 32768
const _ISBLANK: u16  = (1 << 8) >> 8;   // 1
const _ISCNTRL: u16  = (1 << 9) >> 8;   // 2
const _ISPUNCT: u16  = (1 << 10) >> 8;  // 4
const _ISALNUM: u16  = (1 << 11) >> 8;  // 8

#[inline]
fn ctype_class(c: c_int, mask: u16) -> c_int {
    // Reproduces __isctype(c, type) macro:
    //   ((*__ctype_b_loc ())[(int) (c)] & (unsigned short int) type)
    // Result is widened to int when passed to printf("%d", ...).
    unsafe {
        let table_ptr = *__ctype_b_loc();
        // Table is offset by 128 from its base; __ctype_b_loc returns the
        // already-offset pointer, so it can be indexed with values in
        // [-128, 255].
        let entry = *table_ptr.offset(c as isize);
        (entry & mask) as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    // setlocale(LC_ALL, "C");
    let locale_c = CString::new("C").unwrap();
    unsafe {
        setlocale(LC_ALL, locale_c.as_ptr());
    }

    // The C code passes a `char` directly into the ctype macros, which take
    // an int. On platforms where `char` is signed (default on x86_64 Linux)
    // the value is sign-extended; on unsigned-char platforms it's zero-
    // extended. Mirror that by using the same `c_char as c_int` conversion.
    let arg: c_int = c as c_int;

    let fmt_alnum   = CString::new("alphanumeric: %d\n").unwrap();
    let fmt_alpha   = CString::new("alphabetic: %d\n").unwrap();
    let fmt_lower   = CString::new("lowercase: %d\n").unwrap();
    let fmt_upper   = CString::new("uppercase: %d\n").unwrap();
    let fmt_digit   = CString::new("digit: %d\n").unwrap();
    let fmt_hex     = CString::new("hexadecimal: %d\n").unwrap();
    let fmt_cntrl   = CString::new("control: %d\n").unwrap();
    let fmt_graph   = CString::new("graphical: %d\n").unwrap();
    let fmt_space   = CString::new("space: %d\n").unwrap();
    let fmt_blank   = CString::new("blank: %d\n").unwrap();
    let fmt_print   = CString::new("printing: %d\n").unwrap();
    let fmt_punct   = CString::new("punctuation: %d\n").unwrap();
    let fmt_to_low  = CString::new("to lower: %c\n").unwrap();
    let fmt_to_up   = CString::new("to upper: %c\n").unwrap();

    unsafe {
        printf(fmt_alnum.as_ptr(),  ctype_class(arg, _ISALNUM));
        printf(fmt_alpha.as_ptr(),  ctype_class(arg, _ISALPHA));
        printf(fmt_lower.as_ptr(),  ctype_class(arg, _ISLOWER));
        printf(fmt_upper.as_ptr(),  ctype_class(arg, _ISUPPER));
        printf(fmt_digit.as_ptr(),  ctype_class(arg, _ISDIGIT));
        printf(fmt_hex.as_ptr(),    ctype_class(arg, _ISXDIGIT));
        printf(fmt_cntrl.as_ptr(),  ctype_class(arg, _ISCNTRL));
        printf(fmt_graph.as_ptr(),  ctype_class(arg, _ISGRAPH));
        printf(fmt_space.as_ptr(),  ctype_class(arg, _ISSPACE));
        printf(fmt_blank.as_ptr(),  ctype_class(arg, _ISBLANK));
        printf(fmt_print.as_ptr(),  ctype_class(arg, _ISPRINT));
        printf(fmt_punct.as_ptr(),  ctype_class(arg, _ISPUNCT));
        printf(fmt_to_low.as_ptr(), tolower(arg));
        printf(fmt_to_up.as_ptr(),  toupper(arg));
    }
}
