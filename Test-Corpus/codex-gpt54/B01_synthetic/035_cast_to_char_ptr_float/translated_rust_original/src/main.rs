use std::io::{self, Write};

fn print_hex(bytes: &[u8]) {
    let mut stdout = io::stdout().lock();
    for byte in bytes {
        write!(stdout, "{byte:02x}").unwrap();
    }
    writeln!(stdout).unwrap();
}

fn driver(x: f32) {
    print_hex(&x.to_ne_bytes());
}

fn main() {
    let mut x = 0.0f32;

    unsafe {
        libc::scanf(c"%f".as_ptr(), &mut x);
    }

    driver(x);
}
