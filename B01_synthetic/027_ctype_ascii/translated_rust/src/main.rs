use std::io::Read;

extern "C" {
    fn setlocale(category: libc::c_int, locale: *const libc::c_char) -> *mut libc::c_char;
    fn __ctype_b_loc() -> *mut *const u16;
    fn tolower(c: libc::c_int) -> libc::c_int;
    fn toupper(c: libc::c_int) -> libc::c_int;
}

// Glibc ctype bit flags (from <ctype.h> / bits/ctype-inl.h)
const _ISUPPER: u16 = 256;
const _ISLOWER: u16 = 512;
const _ISALPHA: u16 = 1024;
const _ISDIGIT: u16 = 2048;
const _ISXDIGIT: u16 = 4096;
const _ISSPACE: u16 = 8192;
const _ISPRINT: u16 = 16384;
const _ISGRAPH: u16 = 32768;
const _ISBLANK: u16 = 1;
const _ISCNTRL: u16 = 2;
const _ISPUNCT: u16 = 4;
const _ISALNUM: u16 = 8;

unsafe fn ctype_test(c: libc::c_int, mask: u16) -> libc::c_int {
    let table = *__ctype_b_loc();
    ((*table.offset(c as isize)) & mask) as libc::c_int
}

fn driver(c: libc::c_char) {
    unsafe {
        setlocale(libc::LC_ALL, b"C\0".as_ptr() as *const libc::c_char);
    }
    let ci = c as libc::c_int;
    unsafe {
        print!("alphanumeric: {}\n", ctype_test(ci, _ISALNUM));
        print!("alphabetic: {}\n", ctype_test(ci, _ISALPHA));
        print!("lowercase: {}\n", ctype_test(ci, _ISLOWER));
        print!("uppercase: {}\n", ctype_test(ci, _ISUPPER));
        print!("digit: {}\n", ctype_test(ci, _ISDIGIT));
        print!("hexadecimal: {}\n", ctype_test(ci, _ISXDIGIT));
        print!("control: {}\n", ctype_test(ci, _ISCNTRL));
        print!("graphical: {}\n", ctype_test(ci, _ISGRAPH));
        print!("space: {}\n", ctype_test(ci, _ISSPACE));
        print!("blank: {}\n", ctype_test(ci, _ISBLANK));
        print!("printing: {}\n", ctype_test(ci, _ISPRINT));
        print!("punctuation: {}\n", ctype_test(ci, _ISPUNCT));
        print!("to lower: {}\n", tolower(ci) as u8 as char);
        print!("to upper: {}\n", toupper(ci) as u8 as char);
    }
}

fn main() {
    let mut buf = [0u8; 1];
    std::io::stdin().read_exact(&mut buf).unwrap();
    driver(buf[0] as libc::c_char);
}
