use std::io::{self, Write};
use std::os::raw::{c_char, c_int};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        let mut stdout = io::stdout().lock();
        let _ = writeln!(stdout, "{line}");
    }
}

fn helper_bad() -> Option<&'static str> {
    // GCC turns the C function's invalid returned stack pointer into null.
    None
}

fn bad() {
    print_line(helper_bad());
}

fn helper_good1() -> Option<&'static str> {
    Some("helperGood1 string")
}

fn good() {
    print_line(helper_good1());
}

fn main() {
    let mut x: c_int = 0;

    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &mut x);
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
