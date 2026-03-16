use std::fs::File;
use std::io::Read;

static mut FD: Option<File> = None;

pub fn randombytes(x: &mut [u8], mut xlen: u64) {
    unsafe {
        if FD.is_none() {
            loop {
                if let Ok(f) = File::open("/dev/urandom") {
                    FD = Some(f);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }

        let fd = FD.as_mut().unwrap();
        let mut offset = 0usize;
        while xlen > 0 {
            let to_read = if xlen < 1048576 { xlen as usize } else { 1048576 };
            match fd.read(&mut x[offset..offset + to_read]) {
                Ok(n) if n > 0 => {
                    offset += n;
                    xlen -= n as u64;
                }
                _ => {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        }
    }
}
