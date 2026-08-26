use std::ffi::c_float;
use std::io::Write;

fn print_hex(p: &[u8]) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for &b in p {
        let _ = write!(out, "{:02x}", b);
    }
    let _ = writeln!(out);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_float) {
    let raw: [u8; 4] = x.to_ne_bytes();
    print_hex(&raw);
}
