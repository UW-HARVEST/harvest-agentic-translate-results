use std::io::{self, Write};
use std::os::raw::{c_char, c_int};

extern "C" {
    fn getchar() -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn __ctype_b_loc() -> *mut *const u16;
    fn tolower(c: c_int) -> c_int;
    fn toupper(c: c_int) -> c_int;
}

fn ctype_flags(c: c_int) -> u16 {
    unsafe { *(*__ctype_b_loc()).offset(c as isize) }
}

fn driver(c: c_int) {
    // LC_ALL is 6 on the glibc target used by the original executable.
    unsafe {
        setlocale(6, b"C\0".as_ptr().cast());
    }

    let mut output = Vec::new();
    let flags = ctype_flags(c);
    const ALNUM: u16 = 8;
    const ALPHA: u16 = 1024;
    const LOWER: u16 = 512;
    const UPPER: u16 = 256;
    const DIGIT: u16 = 2048;
    const XDIGIT: u16 = 4096;
    const CNTRL: u16 = 2;
    const GRAPH: u16 = 32768;
    const SPACE: u16 = 8192;
    const BLANK: u16 = 1;
    const PRINT: u16 = 16384;
    const PUNCT: u16 = 4;

    writeln!(output, "alphanumeric: {}", flags & ALNUM).unwrap();
    writeln!(output, "alphabetic: {}", flags & ALPHA).unwrap();
    writeln!(output, "lowercase: {}", flags & LOWER).unwrap();
    writeln!(output, "uppercase: {}", flags & UPPER).unwrap();
    writeln!(output, "digit: {}", flags & DIGIT).unwrap();
    writeln!(output, "hexadecimal: {}", flags & XDIGIT).unwrap();
    writeln!(output, "control: {}", flags & CNTRL).unwrap();
    writeln!(output, "graphical: {}", flags & GRAPH).unwrap();
    writeln!(output, "space: {}", flags & SPACE).unwrap();
    writeln!(output, "blank: {}", flags & BLANK).unwrap();
    writeln!(output, "printing: {}", flags & PRINT).unwrap();
    writeln!(output, "punctuation: {}", flags & PUNCT).unwrap();

    unsafe {
        output.extend_from_slice(b"to lower: ");
        output.push(tolower(c) as u8);
        output.push(b'\n');
        output.extend_from_slice(b"to upper: ");
        output.push(toupper(c) as u8);
        output.push(b'\n');
    }

    let _ = io::stdout().write_all(&output);
}

fn main() {
    let c = unsafe { getchar() } as c_char as c_int;
    driver(c);
}
