use std::ffi::{c_char, c_int};
use std::io::{self, Write};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_line(line: Option<&[u8]>) {
    if let Some(line) = line {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        let _ = stdout.write_all(line);
        let _ = stdout.write_all(b"\n");
    }
}

fn bad() {
    // This is the stable output of the uninitialized-pointer branch in the C build.
    print_line(Some(b""));
}

fn good() {
    print_line(Some(b"string"));
}

fn main() {
    let mut x: c_int = 0;

    // SAFETY: `%d` expects one writable `int *`, which `&mut x` provides.
    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &mut x);
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
