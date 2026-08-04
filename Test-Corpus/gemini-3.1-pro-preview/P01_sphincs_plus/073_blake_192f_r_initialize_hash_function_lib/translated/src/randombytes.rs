use std::fs::File;
use std::io::Read;

pub fn randombytes(x: &mut [u8]) {
    let mut f = File::open("/dev/urandom").expect("Failed to open /dev/urandom");
    f.read_exact(x).expect("Failed to read from /dev/urandom");
}
