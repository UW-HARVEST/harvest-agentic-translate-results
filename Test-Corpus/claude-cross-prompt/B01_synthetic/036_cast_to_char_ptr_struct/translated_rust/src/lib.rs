use std::ffi::c_int;
use std::io::{self, Write};

#[repr(C)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
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
pub extern "C" fn driver(floors: c_int) {
    let house = HouseT {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    let size = std::mem::size_of::<HouseT>();
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(&house as *const HouseT as *const u8, size)
    };
    print_hex(bytes);
}
