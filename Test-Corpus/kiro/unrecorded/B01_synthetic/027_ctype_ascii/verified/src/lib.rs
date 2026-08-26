use std::io::Write;

extern "C" {
    fn setlocale(category: libc::c_int, locale: *const libc::c_char) -> *mut libc::c_char;
    fn __ctype_b_loc() -> *mut *const u16;
    fn __ctype_tolower_loc() -> *mut *const i32;
    fn __ctype_toupper_loc() -> *mut *const i32;
}

fn ctype_check(c: i32, mask: u16) -> u16 {
    unsafe { *(*__ctype_b_loc()).offset(c as isize) & mask }
}

#[no_mangle]
pub extern "C" fn driver(c: libc::c_char) {
    let ci = c as libc::c_int;
    unsafe {
        setlocale(libc::LC_ALL, b"C\0".as_ptr() as *const libc::c_char);
    }

    const IS_ALNUM: u16 = 0x0008;
    const IS_ALPHA: u16 = 0x0400;
    const IS_LOWER: u16 = 0x0200;
    const IS_UPPER: u16 = 0x0100;
    const IS_DIGIT: u16 = 0x0800;
    const IS_XDIGIT: u16 = 0x1000;
    const IS_CNTRL: u16 = 0x0002;
    const IS_GRAPH: u16 = 0x8000;
    const IS_SPACE: u16 = 0x2000;
    const IS_BLANK: u16 = 0x0001;
    const IS_PRINT: u16 = 0x4000;
    const IS_PUNCT: u16 = 0x0004;

    let out = std::io::stdout();
    let mut w = out.lock();
    macro_rules! p {
        ($label:expr, $mask:expr) => {
            let _ = write!(w, "{}: {}\n", $label, ctype_check(ci, $mask));
        };
    }
    p!("alphanumeric", IS_ALNUM);
    p!("alphabetic", IS_ALPHA);
    p!("lowercase", IS_LOWER);
    p!("uppercase", IS_UPPER);
    p!("digit", IS_DIGIT);
    p!("hexadecimal", IS_XDIGIT);
    p!("control", IS_CNTRL);
    p!("graphical", IS_GRAPH);
    p!("space", IS_SPACE);
    p!("blank", IS_BLANK);
    p!("printing", IS_PRINT);
    p!("punctuation", IS_PUNCT);

    let lo = unsafe { *(*__ctype_tolower_loc()).offset(ci as isize) } as u8;
    let up = unsafe { *(*__ctype_toupper_loc()).offset(ci as isize) } as u8;
    let _ = w.write_all(b"to lower: ");
    let _ = w.write_all(&[lo, b'\n']);
    let _ = w.write_all(b"to upper: ");
    let _ = w.write_all(&[up, b'\n']);
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> libc::c_int {
    let mut buf = [0u8; 1];
    use std::io::Read;
    let c: libc::c_char = if std::io::stdin().read(&mut buf).unwrap_or(0) == 1 {
        buf[0] as i8 as libc::c_char
    } else {
        -1i8 as libc::c_char
    };
    driver(c);
    0
}
