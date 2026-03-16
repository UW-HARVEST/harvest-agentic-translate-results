// randombytes.rs - reads from /dev/urandom

use std::fs::File;
use std::io::Read;

pub fn randombytes(x: &mut [u8], xlen: u64) {
    let mut xlen = xlen as usize;
    let mut offset = 0usize;
    let mut f = loop {
        if let Ok(f) = File::open("/dev/urandom") {
            break f;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    };

    while xlen > 0 {
        let to_read = if xlen < 1048576 { xlen } else { 1048576 };
        match f.read(&mut x[offset..offset + to_read]) {
            Ok(n) if n >= 1 => {
                offset += n;
                xlen -= n;
            }
            _ => {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }
}
