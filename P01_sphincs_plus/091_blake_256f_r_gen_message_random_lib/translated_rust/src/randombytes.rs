use std::fs::File;
use std::io::Read;

pub fn randombytes(x: &mut [u8], mut xlen: u64) {
    let mut f = File::open("/dev/urandom").expect("Failed to open /dev/urandom");
    let mut offset = 0usize;
    while xlen > 0 {
        let chunk = if xlen < 1048576 { xlen as usize } else { 1048576 };
        let n = f.read(&mut x[offset..offset + chunk]).unwrap_or(0);
        if n < 1 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }
        offset += n;
        xlen -= n as u64;
    }
}
