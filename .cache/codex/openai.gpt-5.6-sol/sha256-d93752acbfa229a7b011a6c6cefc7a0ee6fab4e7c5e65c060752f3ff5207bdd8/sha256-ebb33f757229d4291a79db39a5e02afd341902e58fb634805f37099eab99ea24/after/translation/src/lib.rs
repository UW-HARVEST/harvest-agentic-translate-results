use std::ffi::{c_char, c_int};

const LC_ALL: c_int = 6;

unsafe extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
    fn __ctype_b_loc() -> *mut *const u16;
    fn __ctype_tolower_loc() -> *mut *const c_int;
    fn __ctype_toupper_loc() -> *mut *const c_int;
}

unsafe fn ctype(loc: *mut *const u16, c: c_int, mask: u16) -> c_int {
    unsafe { ((*(*loc).offset(c as isize)) & mask) as c_int }
}

unsafe fn convert_case(loc: *mut *const c_int, c: c_int) -> c_int {
    unsafe { *(*loc).offset(c as isize) }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    let c = c as c_int;

    unsafe {
        setlocale(LC_ALL, c"C".as_ptr());
        let ctype_loc = __ctype_b_loc();

        printf(c"alphanumeric: %d\n".as_ptr(), ctype(ctype_loc, c, 0x0008));
        printf(c"alphabetic: %d\n".as_ptr(), ctype(ctype_loc, c, 0x0400));
        printf(c"lowercase: %d\n".as_ptr(), ctype(ctype_loc, c, 0x0200));
        printf(c"uppercase: %d\n".as_ptr(), ctype(ctype_loc, c, 0x0100));
        printf(c"digit: %d\n".as_ptr(), ctype(ctype_loc, c, 0x0800));
        printf(c"hexadecimal: %d\n".as_ptr(), ctype(ctype_loc, c, 0x1000));
        printf(c"control: %d\n".as_ptr(), ctype(ctype_loc, c, 0x0002));
        printf(c"graphical: %d\n".as_ptr(), ctype(ctype_loc, c, 0x8000));
        printf(c"space: %d\n".as_ptr(), ctype(ctype_loc, c, 0x2000));
        printf(c"blank: %d\n".as_ptr(), ctype(ctype_loc, c, 0x0001));
        printf(c"printing: %d\n".as_ptr(), ctype(ctype_loc, c, 0x4000));
        printf(c"punctuation: %d\n".as_ptr(), ctype(ctype_loc, c, 0x0004));
        printf(
            c"to lower: %c\n".as_ptr(),
            convert_case(__ctype_tolower_loc(), c),
        );
        printf(
            c"to upper: %c\n".as_ptr(),
            convert_case(__ctype_toupper_loc(), c),
        );
    }
}
