use std::ffi::c_int;
use std::io::Write;

fn print_hex(p: &[u8]) {
    let mut stdout = std::io::stdout().lock();
    let mut buf = String::with_capacity(p.len() * 2 + 1);
    for &b in p {
        use std::fmt::Write as _;
        let _ = write!(buf, "{:02x}", b);
    }
    buf.push('\n');
    let _ = stdout.write_all(buf.as_bytes());
    let _ = stdout.flush();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let raw: [u8; std::mem::size_of::<c_int>()] = x.to_ne_bytes();
    print_hex(&raw);
}
