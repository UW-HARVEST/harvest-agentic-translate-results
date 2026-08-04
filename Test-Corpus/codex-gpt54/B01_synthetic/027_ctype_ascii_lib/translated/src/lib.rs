use libc::{c_char, c_int, c_ushort, LC_ALL};

unsafe extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
    fn __ctype_b_loc() -> *const *const c_ushort;
    fn __ctype_tolower_loc() -> *const *const c_int;
    fn __ctype_toupper_loc() -> *const *const c_int;
}

#[cfg(target_endian = "little")]
const fn isbit(bit: u32) -> c_ushort {
    if bit < 8 {
        ((1u16 << bit) << 8) as c_ushort
    } else {
        ((1u16 << bit) >> 8) as c_ushort
    }
}

#[cfg(target_endian = "big")]
const fn isbit(bit: u32) -> c_ushort {
    (1u16 << bit) as c_ushort
}

const IS_UPPER: c_ushort = isbit(0);
const IS_LOWER: c_ushort = isbit(1);
const IS_ALPHA: c_ushort = isbit(2);
const IS_DIGIT: c_ushort = isbit(3);
const IS_XDIGIT: c_ushort = isbit(4);
const IS_SPACE: c_ushort = isbit(5);
const IS_PRINT: c_ushort = isbit(6);
const IS_GRAPH: c_ushort = isbit(7);
const IS_BLANK: c_ushort = isbit(8);
const IS_CNTRL: c_ushort = isbit(9);
const IS_PUNCT: c_ushort = isbit(10);
const IS_ALNUM: c_ushort = isbit(11);

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    unsafe {
        setlocale(LC_ALL, b"C\0".as_ptr().cast());

        let c = c as c_int;
        let ctype = *__ctype_b_loc();
        let tolower = *__ctype_tolower_loc();
        let toupper = *__ctype_toupper_loc();

        printf(
            b"alphanumeric: %d\n\0".as_ptr().cast(),
            *ctype.offset(c as isize) as c_int & IS_ALNUM as c_int,
        );
        printf(
            b"alphabetic: %d\n\0".as_ptr().cast(),
            *ctype.offset(c as isize) as c_int & IS_ALPHA as c_int,
        );
        printf(
            b"lowercase: %d\n\0".as_ptr().cast(),
            *ctype.offset(c as isize) as c_int & IS_LOWER as c_int,
        );
        printf(
            b"uppercase: %d\n\0".as_ptr().cast(),
            *ctype.offset(c as isize) as c_int & IS_UPPER as c_int,
        );
        printf(
            b"digit: %d\n\0".as_ptr().cast(),
            *ctype.offset(c as isize) as c_int & IS_DIGIT as c_int,
        );
        printf(
            b"hexadecimal: %d\n\0".as_ptr().cast(),
            *ctype.offset(c as isize) as c_int & IS_XDIGIT as c_int,
        );
        printf(
            b"control: %d\n\0".as_ptr().cast(),
            *ctype.offset(c as isize) as c_int & IS_CNTRL as c_int,
        );
        printf(
            b"graphical: %d\n\0".as_ptr().cast(),
            *ctype.offset(c as isize) as c_int & IS_GRAPH as c_int,
        );
        printf(
            b"space: %d\n\0".as_ptr().cast(),
            *ctype.offset(c as isize) as c_int & IS_SPACE as c_int,
        );
        printf(
            b"blank: %d\n\0".as_ptr().cast(),
            *ctype.offset(c as isize) as c_int & IS_BLANK as c_int,
        );
        printf(
            b"printing: %d\n\0".as_ptr().cast(),
            *ctype.offset(c as isize) as c_int & IS_PRINT as c_int,
        );
        printf(
            b"punctuation: %d\n\0".as_ptr().cast(),
            *ctype.offset(c as isize) as c_int & IS_PUNCT as c_int,
        );
        printf(b"to lower: %c\n\0".as_ptr().cast(), *tolower.offset(c as isize));
        printf(b"to upper: %c\n\0".as_ptr().cast(), *toupper.offset(c as isize));
    }
}
