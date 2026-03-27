use std::io::Read;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: u64) {
    let mut remaining = xlen as usize;
    let mut offset = 0usize;
    let mut file = std::fs::File::open("/dev/urandom").expect("Failed to open /dev/urandom");
    while remaining > 0 {
        let chunk = if remaining < 1048576 { remaining } else { 1048576 };
        let buf = core::slice::from_raw_parts_mut(x.add(offset), chunk);
        match file.read(buf) {
            Ok(n) if n > 0 => {
                offset += n;
                remaining -= n;
            }
            _ => {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }
}
