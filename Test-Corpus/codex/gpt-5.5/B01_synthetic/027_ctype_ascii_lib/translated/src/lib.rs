use std::ffi::{c_char, c_int, c_ushort};

unsafe extern "C" {
    fn __ctype_b_loc() -> *mut *const c_ushort;
}

const IS_UPPER: c_ushort = 256;
const IS_LOWER: c_ushort = 512;
const IS_ALPHA: c_ushort = 1024;
const IS_DIGIT: c_ushort = 2048;
const IS_XDIGIT: c_ushort = 4096;
const IS_SPACE: c_ushort = 8192;
const IS_PRINT: c_ushort = 16384;
const IS_GRAPH: c_ushort = 32768;
const IS_BLANK: c_ushort = 1;
const IS_CNTRL: c_ushort = 2;
const IS_PUNCT: c_ushort = 4;
const IS_ALNUM: c_ushort = 8;

fn print_int(format: &'static [u8], value: c_int) {
    unsafe {
        libc::printf(format.as_ptr().cast::<c_char>(), value);
    }
}

fn print_char(format: &'static [u8], value: c_int) {
    unsafe {
        libc::printf(format.as_ptr().cast::<c_char>(), value);
    }
}

fn classify(c: c_int, mask: c_ushort) -> c_int {
    unsafe { (*(*__ctype_b_loc()).offset(c as isize) & mask) as c_int }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    let input = c as c_int;

    unsafe {
        libc::setlocale(libc::LC_ALL, c"C".as_ptr());
    }

    print_int(b"alphanumeric: %d\n\0", classify(input, IS_ALNUM));
    print_int(b"alphabetic: %d\n\0", classify(input, IS_ALPHA));
    print_int(b"lowercase: %d\n\0", classify(input, IS_LOWER));
    print_int(b"uppercase: %d\n\0", classify(input, IS_UPPER));
    print_int(b"digit: %d\n\0", classify(input, IS_DIGIT));
    print_int(b"hexadecimal: %d\n\0", classify(input, IS_XDIGIT));
    print_int(b"control: %d\n\0", classify(input, IS_CNTRL));
    print_int(b"graphical: %d\n\0", classify(input, IS_GRAPH));
    print_int(b"space: %d\n\0", classify(input, IS_SPACE));
    print_int(b"blank: %d\n\0", classify(input, IS_BLANK));
    print_int(b"printing: %d\n\0", classify(input, IS_PRINT));
    print_int(b"punctuation: %d\n\0", classify(input, IS_PUNCT));
    print_char(b"to lower: %c\n\0", unsafe { libc::tolower(input) });
    print_char(b"to upper: %c\n\0", unsafe { libc::toupper(input) });
}
