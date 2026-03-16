use std::fs::File;
use std::io::Read;
use std::sync::Mutex;

static FD: Mutex<Option<File>> = Mutex::new(None);

pub fn randombytes(x: &mut [u8], mut xlen: u64) {
    let mut fd = FD.lock().unwrap();
    if fd.is_none() {
        *fd = Some(File::open("/dev/urandom").expect("Failed to open /dev/urandom"));
    }
    let file = fd.as_mut().unwrap();

    let mut offset = 0usize;
    while xlen > 0 {
        let to_read = if xlen < 1048576 { xlen as usize } else { 1048576 };
        match file.read(&mut x[offset..offset + to_read]) {
            Ok(0) => {
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue;
            }
            Ok(n) => {
                offset += n;
                xlen -= n as u64;
            }
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue;
            }
        }
    }
}
