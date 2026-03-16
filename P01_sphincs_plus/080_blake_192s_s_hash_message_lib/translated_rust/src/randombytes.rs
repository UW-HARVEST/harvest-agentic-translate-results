use std::fs::File;
use std::io::Read;

pub fn randombytes(x: &mut [u8]) {
    let mut remaining = x.len();
    let mut offset = 0;
    let mut f = loop {
        if let Ok(f) = File::open("/dev/urandom") { break f; }
        std::thread::sleep(std::time::Duration::from_secs(1));
    };
    while remaining > 0 {
        let chunk = if remaining < 1048576 { remaining } else { 1048576 };
        match f.read(&mut x[offset..offset + chunk]) {
            Ok(n) if n >= 1 => { offset += n; remaining -= n; }
            _ => { std::thread::sleep(std::time::Duration::from_secs(1)); }
        }
    }
}
