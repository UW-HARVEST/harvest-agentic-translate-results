use std::io::Read;

extern "C" {
    fn setlocale(category: libc::c_int, locale: *const libc::c_char) -> *mut libc::c_char;
    fn __ctype_b_loc() -> *const *const u16;
    fn tolower(c: libc::c_int) -> libc::c_int;
    fn toupper(c: libc::c_int) -> libc::c_int;
}

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

unsafe fn isctype(c: libc::c_int, mask: u16) -> u16 {
    let table = *__ctype_b_loc();
    (*table.offset(c as isize)) & mask
}

fn driver(c: libc::c_char) {
    unsafe {
        setlocale(libc::LC_ALL, b"C\0".as_ptr() as *const libc::c_char);
        let ci = c as libc::c_int;
        print!("alphanumeric: {}\n", isctype(ci, _ISALNUM));
        print!("alphabetic: {}\n", isctype(ci, _ISALPHA));
        print!("lowercase: {}\n", isctype(ci, _ISLOWER));
        print!("uppercase: {}\n", isctype(ci, _ISUPPER));
        print!("digit: {}\n", isctype(ci, _ISDIGIT));
        print!("hexadecimal: {}\n", isctype(ci, _ISXDIGIT));
        print!("control: {}\n", isctype(ci, _ISCNTRL));
        print!("graphical: {}\n", isctype(ci, _ISGRAPH));
        print!("space: {}\n", isctype(ci, _ISSPACE));
        print!("blank: {}\n", isctype(ci, _ISBLANK));
        print!("printing: {}\n", isctype(ci, _ISPRINT));
        print!("punctuation: {}\n", isctype(ci, _ISPUNCT));
        print!("to lower: {}\n", tolower(ci) as u8 as char);
        print!("to upper: {}\n", toupper(ci) as u8 as char);
    }
}

fn main() {
    let mut buf = [0u8; 1];
    std::io::stdin().read_exact(&mut buf).unwrap();
    driver(buf[0] as libc::c_char);
}
