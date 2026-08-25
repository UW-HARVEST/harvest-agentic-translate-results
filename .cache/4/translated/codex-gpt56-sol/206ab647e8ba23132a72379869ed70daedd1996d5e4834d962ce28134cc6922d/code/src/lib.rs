use std::ffi::{c_char, c_int, c_ushort};

const LC_ALL: c_int = 6;

unsafe extern "C" {
    fn __ctype_b_loc() -> *const *const c_ushort;
    fn __ctype_tolower_loc() -> *const *const c_int;
    fn __ctype_toupper_loc() -> *const *const c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

#[inline]
unsafe fn print_class(format: &'static [u8], classes: c_ushort, mask: c_ushort) {
    unsafe {
        printf(
            format.as_ptr().cast(),
            c_int::from(classes & mask),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(c: c_char) {
    unsafe {
        setlocale(LC_ALL, c"C".as_ptr());

        let index = c as isize;
        let classes = *(*__ctype_b_loc()).offset(index);

        print_class(b"alphanumeric: %d\n\0", classes, 0x0008);
        print_class(b"alphabetic: %d\n\0", classes, 0x0400);
        print_class(b"lowercase: %d\n\0", classes, 0x0200);
        print_class(b"uppercase: %d\n\0", classes, 0x0100);
        print_class(b"digit: %d\n\0", classes, 0x0800);
        print_class(b"hexadecimal: %d\n\0", classes, 0x1000);
        print_class(b"control: %d\n\0", classes, 0x0002);
        print_class(b"graphical: %d\n\0", classes, 0x8000);
        print_class(b"space: %d\n\0", classes, 0x2000);
        print_class(b"blank: %d\n\0", classes, 0x0001);
        print_class(b"printing: %d\n\0", classes, 0x4000);
        print_class(b"punctuation: %d\n\0", classes, 0x0004);

        let lower = *(*__ctype_tolower_loc()).offset(index);
        printf(b"to lower: %c\n\0".as_ptr().cast(), lower);

        let upper = *(*__ctype_toupper_loc()).offset(index);
        printf(b"to upper: %c\n\0".as_ptr().cast(), upper);
    }
}
