use std::io::{self, Write};

fn print_hex(p: &[u8]) {
    let mut out = io::stdout().lock();
    for b in p {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}
