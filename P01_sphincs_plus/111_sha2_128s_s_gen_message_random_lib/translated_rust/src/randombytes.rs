pub fn randombytes(x: &mut [u8], xlen: u64) {
    use std::fs::File;
    use std::io::Read;

    let mut f = loop {
        if let Ok(f) = File::open("/dev/urandom") {
            break f;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    };

    let mut remaining = xlen as usize;
    let mut offset = 0usize;
    while remaining > 0 {
        let chunk = if remaining < 1048576 { remaining } else { 1048576 };
        match f.read(&mut x[offset..offset + chunk]) {
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
