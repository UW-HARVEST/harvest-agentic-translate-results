use std::ffi::{c_char, c_int};
use std::io::{self, Write};

#[repr(C)]
struct DivResult {
    quot: c_int,
    rem: c_int,
}

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn div(numerator: c_int, denominator: c_int) -> DivResult;
}

fn main() {
    let mut x: c_int = 1;
    let mut y: c_int = 1;

    unsafe {
        scanf(b"%d %d\0".as_ptr().cast(), &mut x, &mut y);
    }

    let result = unsafe { div(x, y) };
    let mut stdout = io::stdout().lock();
    let _ = writeln!(
        stdout,
        "quotient: {}, remainder: {}",
        result.quot, result.rem
    );
}
