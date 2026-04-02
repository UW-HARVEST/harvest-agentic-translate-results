use std::ffi::{c_char, c_int};

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
    fn __ctype_b_loc() -> *const *const u16;
    fn tolower(c: c_int) -> c_int;
    fn toupper(c: c_int) -> c_int;
}

const LC_ALL: c_int = 6;

const _ISBLANK: u16 = 1;
const _ISCNTRL: u16 = 2;
const _ISPUNCT: u16 = 4;
const _ISALNUM: u16 = 8;
const _ISUPPER: u16 = 256;
const _ISLOWER: u16 = 512;
const _ISALPHA: u16 = 1024;
const _ISDIGIT: u16 = 2048;
const _ISXDIGIT: u16 = 4096;
const _ISSPACE: u16 = 8192;
const _ISPRINT: u16 = 16384;
const _ISGRAPH: u16 = 32768;

unsafe fn ctype_test(c: c_int, mask: u16) -> c_int {
    let table = unsafe { *__ctype_b_loc() };
    let val = unsafe { *table.offset(c as isize) };
    (val & mask) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    let ci = c as c_int;
    unsafe {
        setlocale(LC_ALL, b"C\0".as_ptr() as *const c_char);

        printf(b"alphanumeric: %d\n\0".as_ptr() as *const c_char, ctype_test(ci, _ISALNUM));
        printf(b"alphabetic: %d\n\0".as_ptr() as *const c_char, ctype_test(ci, _ISALPHA));
        printf(b"lowercase: %d\n\0".as_ptr() as *const c_char, ctype_test(ci, _ISLOWER));
        printf(b"uppercase: %d\n\0".as_ptr() as *const c_char, ctype_test(ci, _ISUPPER));
        printf(b"digit: %d\n\0".as_ptr() as *const c_char, ctype_test(ci, _ISDIGIT));
        printf(b"hexadecimal: %d\n\0".as_ptr() as *const c_char, ctype_test(ci, _ISXDIGIT));
        printf(b"control: %d\n\0".as_ptr() as *const c_char, ctype_test(ci, _ISCNTRL));
        printf(b"graphical: %d\n\0".as_ptr() as *const c_char, ctype_test(ci, _ISGRAPH));
        printf(b"space: %d\n\0".as_ptr() as *const c_char, ctype_test(ci, _ISSPACE));
        printf(b"blank: %d\n\0".as_ptr() as *const c_char, ctype_test(ci, _ISBLANK));
        printf(b"printing: %d\n\0".as_ptr() as *const c_char, ctype_test(ci, _ISPRINT));
        printf(b"punctuation: %d\n\0".as_ptr() as *const c_char, ctype_test(ci, _ISPUNCT));
        printf(b"to lower: %c\n\0".as_ptr() as *const c_char, tolower(ci));
        printf(b"to upper: %c\n\0".as_ptr() as *const c_char, toupper(ci));
    }
}
