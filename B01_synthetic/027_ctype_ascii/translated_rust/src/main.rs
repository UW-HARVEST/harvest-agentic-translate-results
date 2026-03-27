use std::ffi::CString;
use std::io::Read;

extern "C" {
    fn __ctype_b_loc() -> *const *const u16;
    fn __ctype_tolower_loc() -> *const *const i32;
    fn __ctype_toupper_loc() -> *const *const i32;
}

unsafe fn ctype_check(c: i32, mask: u16) -> i32 {
    let table = *__ctype_b_loc();
    ((*table.offset(c as isize)) & mask) as i32
}

unsafe fn ctype_tolower(c: i32) -> i32 {
    let table = *__ctype_tolower_loc();
    *table.offset(c as isize)
}

unsafe fn ctype_toupper(c: i32) -> i32 {
    let table = *__ctype_toupper_loc();
    *table.offset(c as isize)
}

const IS_ALNUM: u16 = 8;
const IS_ALPHA: u16 = 1024;
const IS_LOWER: u16 = 512;
const IS_UPPER: u16 = 256;
const IS_DIGIT: u16 = 2048;
const IS_XDIGIT: u16 = 4096;
const IS_CNTRL: u16 = 2;
const IS_GRAPH: u16 = 32768;
const IS_SPACE: u16 = 8192;
const IS_BLANK: u16 = 1;
const IS_PRINT: u16 = 16384;
const IS_PUNCT: u16 = 4;

#[no_mangle]
pub extern "C" fn driver(c: libc::c_int) {
    unsafe {
        let locale = CString::new("C").unwrap();
        libc::setlocale(libc::LC_ALL, locale.as_ptr());

        libc::printf(b"alphanumeric: %d\n\0".as_ptr() as *const _, ctype_check(c, IS_ALNUM));
        libc::printf(b"alphabetic: %d\n\0".as_ptr() as *const _, ctype_check(c, IS_ALPHA));
        libc::printf(b"lowercase: %d\n\0".as_ptr() as *const _, ctype_check(c, IS_LOWER));
        libc::printf(b"uppercase: %d\n\0".as_ptr() as *const _, ctype_check(c, IS_UPPER));
        libc::printf(b"digit: %d\n\0".as_ptr() as *const _, ctype_check(c, IS_DIGIT));
        libc::printf(b"hexadecimal: %d\n\0".as_ptr() as *const _, ctype_check(c, IS_XDIGIT));
        libc::printf(b"control: %d\n\0".as_ptr() as *const _, ctype_check(c, IS_CNTRL));
        libc::printf(b"graphical: %d\n\0".as_ptr() as *const _, ctype_check(c, IS_GRAPH));
        libc::printf(b"space: %d\n\0".as_ptr() as *const _, ctype_check(c, IS_SPACE));
        libc::printf(b"blank: %d\n\0".as_ptr() as *const _, ctype_check(c, IS_BLANK));
        libc::printf(b"printing: %d\n\0".as_ptr() as *const _, ctype_check(c, IS_PRINT));
        libc::printf(b"punctuation: %d\n\0".as_ptr() as *const _, ctype_check(c, IS_PUNCT));
        libc::printf(b"to lower: %c\n\0".as_ptr() as *const _, ctype_tolower(c));
        libc::printf(b"to upper: %c\n\0".as_ptr() as *const _, ctype_toupper(c));
    }
}

fn main() {
    let mut buf = [0u8; 1];
    let c = if std::io::stdin().read(&mut buf).unwrap_or(0) == 1 {
        buf[0] as libc::c_int
    } else {
        libc::EOF
    };
    driver(c);
}
