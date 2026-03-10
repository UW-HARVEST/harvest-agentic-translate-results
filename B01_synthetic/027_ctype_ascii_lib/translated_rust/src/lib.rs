use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
    fn isalnum(c: c_int) -> c_int;
    fn isalpha(c: c_int) -> c_int;
    fn islower(c: c_int) -> c_int;
    fn isupper(c: c_int) -> c_int;
    fn isdigit(c: c_int) -> c_int;
    fn isxdigit(c: c_int) -> c_int;
    fn iscntrl(c: c_int) -> c_int;
    fn isgraph(c: c_int) -> c_int;
    fn isspace(c: c_int) -> c_int;
    fn isblank(c: c_int) -> c_int;
    fn isprint(c: c_int) -> c_int;
    fn ispunct(c: c_int) -> c_int;
    fn tolower(c: c_int) -> c_int;
    fn toupper(c: c_int) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    unsafe {
        setlocale(0 /* LC_ALL on Linux */, b"C\0".as_ptr() as *const c_char);

        let ci = c as c_int;
        printf(b"alphanumeric: %d\n\0".as_ptr() as *const c_char, isalnum(ci));
        printf(b"alphabetic: %d\n\0".as_ptr() as *const c_char, isalpha(ci));
        printf(b"lowercase: %d\n\0".as_ptr() as *const c_char, islower(ci));
        printf(b"uppercase: %d\n\0".as_ptr() as *const c_char, isupper(ci));
        printf(b"digit: %d\n\0".as_ptr() as *const c_char, isdigit(ci));
        printf(b"hexadecimal: %d\n\0".as_ptr() as *const c_char, isxdigit(ci));
        printf(b"control: %d\n\0".as_ptr() as *const c_char, iscntrl(ci));
        printf(b"graphical: %d\n\0".as_ptr() as *const c_char, isgraph(ci));
        printf(b"space: %d\n\0".as_ptr() as *const c_char, isspace(ci));
        printf(b"blank: %d\n\0".as_ptr() as *const c_char, isblank(ci));
        printf(b"printing: %d\n\0".as_ptr() as *const c_char, isprint(ci));
        printf(b"punctuation: %d\n\0".as_ptr() as *const c_char, ispunct(ci));
        printf(b"to lower: %c\n\0".as_ptr() as *const c_char, tolower(ci));
        printf(b"to upper: %c\n\0".as_ptr() as *const c_char, toupper(ci));
    }
}
