use std::ffi::c_int;
use std::io::Write;

fn print_hex(p: &[u8]) {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let mut buf = String::with_capacity(p.len() * 2 + 1);
    for &b in p {
        use std::fmt::Write as _;
        let _ = write!(buf, "{:02x}", b);
    }
    buf.push('\n');
    let _ = handle.write_all(buf.as_bytes());
    let _ = handle.flush();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let bytes = x.to_ne_bytes();
    print_hex(&bytes);
}
