use std::ffi::{c_char, c_int, c_ushort};

const LC_ALL: c_int = 6;

const IS_ALNUM: c_ushort = 8;
const IS_ALPHA: c_ushort = 1024;
const IS_LOWER: c_ushort = 512;
const IS_UPPER: c_ushort = 256;
const IS_DIGIT: c_ushort = 2048;
const IS_XDIGIT: c_ushort = 4096;
const IS_CNTRL: c_ushort = 2;
const IS_GRAPH: c_ushort = 32768;
const IS_SPACE: c_ushort = 8192;
const IS_BLANK: c_ushort = 1;
const IS_PRINT: c_ushort = 16384;
const IS_PUNCT: c_ushort = 4;

unsafe extern "C" {
    fn __ctype_b_loc() -> *const *const c_ushort;
    fn getchar() -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn tolower(c: c_int) -> c_int;
    fn toupper(c: c_int) -> c_int;
}

unsafe fn classification(c: c_char, mask: c_ushort) -> c_int {
    let table = unsafe { *__ctype_b_loc() };
    unsafe { (*table.offset(c as isize) & mask) as c_int }
}

#[no_mangle]
pub unsafe extern "C" fn driver(c: c_char) {
    unsafe {
        setlocale(LC_ALL, c"C".as_ptr());

        printf(c"alphanumeric: %d\n".as_ptr(), classification(c, IS_ALNUM));
        printf(c"alphabetic: %d\n".as_ptr(), classification(c, IS_ALPHA));
        printf(c"lowercase: %d\n".as_ptr(), classification(c, IS_LOWER));
        printf(c"uppercase: %d\n".as_ptr(), classification(c, IS_UPPER));
        printf(c"digit: %d\n".as_ptr(), classification(c, IS_DIGIT));
        printf(c"hexadecimal: %d\n".as_ptr(), classification(c, IS_XDIGIT));
        printf(c"control: %d\n".as_ptr(), classification(c, IS_CNTRL));
        printf(c"graphical: %d\n".as_ptr(), classification(c, IS_GRAPH));
        printf(c"space: %d\n".as_ptr(), classification(c, IS_SPACE));
        printf(c"blank: %d\n".as_ptr(), classification(c, IS_BLANK));
        printf(c"printing: %d\n".as_ptr(), classification(c, IS_PRINT));
        printf(c"punctuation: %d\n".as_ptr(), classification(c, IS_PUNCT));
        printf(c"to lower: %c\n".as_ptr(), tolower(c as c_int));
        printf(c"to upper: %c\n".as_ptr(), toupper(c as c_int));
    }
}

#[no_mangle]
pub unsafe extern "C" fn main() -> c_int {
    let c = unsafe { getchar() } as c_char;
    unsafe { driver(c) };
    0
}
