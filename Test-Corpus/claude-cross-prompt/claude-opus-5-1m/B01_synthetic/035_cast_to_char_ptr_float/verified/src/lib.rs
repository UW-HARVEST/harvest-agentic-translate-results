use std::io::Write;

fn print_hex(p: &[u8]) {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    for &b in p {
        let _ = write!(handle, "{:02x}", b);
    }
    let _ = writeln!(handle);
    let _ = handle.flush();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: core::ffi::c_float) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}
