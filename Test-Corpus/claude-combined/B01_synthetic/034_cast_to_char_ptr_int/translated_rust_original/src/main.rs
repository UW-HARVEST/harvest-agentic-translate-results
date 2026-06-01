// Translated from C to Rust. Produces byte-identical output to the C reference.
//
// Behavior match notes:
// - We use libc::scanf to read an `int` so the parsing semantics (whitespace
//   skipping across newlines, sign handling, saturation on overflow, leaving
//   `x` unchanged on no match) match C exactly.
// - print_hex prints the bytes of an `int` in machine (little-endian on
//   x86_64 / aarch64) order, matching what `(unsigned char*)&x` does in C.

use std::io::{self, Write};

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut buf = String::with_capacity(p.len() * 2 + 1);
    for b in p {
        // %02x: lowercase hex, zero-padded to width 2
        buf.push_str(&format!("{:02x}", b));
    }
    buf.push('\n');
    out.write_all(buf.as_bytes()).expect("write failed");
}

fn driver(x: i32) {
    // Equivalent to: print_hex((unsigned char *)&x, sizeof(x));
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}

fn main() {
    let mut x: libc::c_int = 0;
    // Use C's scanf so behavior on whitespace, sign, overflow saturation,
    // and unmatched input matches the C reference exactly.
    let fmt = b"%d\0";
    unsafe {
        libc::scanf(fmt.as_ptr() as *const libc::c_char, &mut x as *mut libc::c_int);
    }
    driver(x as i32);
    // Ensure stdout is flushed (return 0 in C also flushes via exit).
    io::stdout().flush().ok();
}
