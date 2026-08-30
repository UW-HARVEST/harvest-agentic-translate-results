use std::io::{self, Write};
use std::os::raw::{c_char, c_int};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn main() {
    let mut x: c_int = 0;

    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &mut x);
    }

    let mut stdout = io::stdout().lock();
    for byte in x.to_ne_bytes() {
        let _ = write!(stdout, "{byte:02x}");
    }
    let _ = writeln!(stdout);
}
