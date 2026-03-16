use std::fs::File;
use std::io::Read;
use std::sync::OnceLock;
use std::sync::Mutex;

static URANDOM: OnceLock<Mutex<File>> = OnceLock::new();

fn get_urandom() -> &'static Mutex<File> {
    URANDOM.get_or_init(|| {
        loop {
            if let Ok(f) = File::open("/dev/urandom") {
                return Mutex::new(f);
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    })
}

pub fn urandom_randombytes(x: &mut [u8], mut xlen: u64) {
    let mut offset = 0usize;
    let mut f = get_urandom().lock().unwrap();
    while xlen > 0 {
        let chunk = if xlen < 1048576 { xlen as usize } else { 1048576 };
        match f.read(&mut x[offset..offset + chunk]) {
            Ok(n) if n > 0 => {
                offset += n;
                xlen -= n as u64;
            }
            _ => {
                drop(f);
                std::thread::sleep(std::time::Duration::from_secs(1));
                f = get_urandom().lock().unwrap();
            }
        }
    }
}
