// randombytes.c - reads from /dev/urandom

use std::fs::File;
use std::io::Read;
use std::sync::Mutex;

static FD: Mutex<Option<File>> = Mutex::new(None);

pub fn randombytes(x: &mut [u8], mut xlen: usize) {
    let mut guard = FD.lock().unwrap();
    if guard.is_none() {
        loop {
            if let Ok(f) = File::open("/dev/urandom") {
                *guard = Some(f);
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    let f = guard.as_mut().unwrap();
    let mut off = 0usize;
    while xlen > 0 {
        let chunk = if xlen < 1048576 { xlen } else { 1048576 };
        match f.read(&mut x[off..off + chunk]) {
            Ok(0) | Err(_) => {
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue;
            }
            Ok(n) => {
                off += n;
                xlen -= n;
            }
        }
    }
}
