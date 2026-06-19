use std::ffi::c_char;
use std::fmt::Write as _;
use std::io::{self, Write as _};
use std::os::raw::c_int;

const LC_ALL: c_int = 6;

extern "C" {
    fn getchar() -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;

    fn __ctype_b_loc() -> *mut *const u16;
    fn tolower(c: c_int) -> c_int;
    fn toupper(c: c_int) -> c_int;
}

const IS_UPPER: c_int = 256;
const IS_LOWER: c_int = 512;
const IS_ALPHA: c_int = 1024;
const IS_DIGIT: c_int = 2048;
const IS_XDIGIT: c_int = 4096;
const IS_SPACE: c_int = 8192;
const IS_PRINT: c_int = 16384;
const IS_GRAPH: c_int = 32768;
const IS_BLANK: c_int = 1;
const IS_CNTRL: c_int = 2;
const IS_PUNCT: c_int = 4;
const IS_ALNUM: c_int = 8;

fn append_line(out: &mut Vec<u8>, label: &str, value: c_int) {
    let mut line = String::new();
    write!(&mut line, "{}: {}\n", label, value).unwrap();
    out.extend_from_slice(line.as_bytes());
}

fn append_char_line(out: &mut Vec<u8>, label: &str, value: c_int) {
    out.extend_from_slice(label.as_bytes());
    out.extend_from_slice(b": ");
    out.push(value as u8);
    out.push(b'\n');
}

unsafe fn classify(c: c_int, mask: c_int) -> c_int {
    let table = *__ctype_b_loc();
    (*table.offset(c as isize) as c_int) & mask
}

fn driver(c: c_char) -> io::Result<()> {
    unsafe {
        setlocale(LC_ALL, c"C".as_ptr());
    }

    let c = c as c_int;
    let mut out = Vec::new();

    unsafe {
        append_line(&mut out, "alphanumeric", classify(c, IS_ALNUM));
        append_line(&mut out, "alphabetic", classify(c, IS_ALPHA));
        append_line(&mut out, "lowercase", classify(c, IS_LOWER));
        append_line(&mut out, "uppercase", classify(c, IS_UPPER));
        append_line(&mut out, "digit", classify(c, IS_DIGIT));
        append_line(&mut out, "hexadecimal", classify(c, IS_XDIGIT));
        append_line(&mut out, "control", classify(c, IS_CNTRL));
        append_line(&mut out, "graphical", classify(c, IS_GRAPH));
        append_line(&mut out, "space", classify(c, IS_SPACE));
        append_line(&mut out, "blank", classify(c, IS_BLANK));
        append_line(&mut out, "printing", classify(c, IS_PRINT));
        append_line(&mut out, "punctuation", classify(c, IS_PUNCT));
        append_char_line(&mut out, "to lower", tolower(c));
        append_char_line(&mut out, "to upper", toupper(c));
    }

    io::stdout().write_all(&out)
}

fn main() -> io::Result<()> {
    let c = unsafe { getchar() } as c_char;
    driver(c)
}
