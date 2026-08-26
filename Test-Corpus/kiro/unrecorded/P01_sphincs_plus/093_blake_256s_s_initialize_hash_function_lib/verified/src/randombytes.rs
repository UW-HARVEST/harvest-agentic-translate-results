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

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) {
    unsafe {
        let mut remaining = xlen as usize;
        let mut ptr = x;
        let file = get_urandom();
        let mut f = file.lock().unwrap();

        while remaining > 0 {
            let chunk = if remaining < 1048576 { remaining } else { 1048576 };
            let buf = std::slice::from_raw_parts_mut(ptr, chunk);
            match f.read(buf) {
                Ok(0) | Err(_) => {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
                Ok(n) => {
                    ptr = ptr.add(n);
                    remaining -= n;
                }
            }
        }
    }
}
