use std::io::{self, Write};
use std::os::raw::c_int;

fn print_hex(p: &[u8]) {
    let mut stdout = io::stdout().lock();
    for byte in p {
        let _ = write!(stdout, "{:02x}", byte);
    }
    let _ = writeln!(stdout);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}
