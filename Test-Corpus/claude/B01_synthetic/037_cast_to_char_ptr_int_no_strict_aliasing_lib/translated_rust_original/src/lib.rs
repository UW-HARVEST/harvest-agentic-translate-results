use std::ffi::c_int;
use std::io::Write;

fn print_hex(p: &[u8]) {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    for b in p {
        // Match printf("%02x") behavior — lowercase, zero-padded, two digits
        let _ = write!(handle, "{:02x}", b);
    }
    let _ = writeln!(handle);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let raw: [u8; std::mem::size_of::<c_int>()] = x.to_ne_bytes();
    print_hex(&raw);
}
