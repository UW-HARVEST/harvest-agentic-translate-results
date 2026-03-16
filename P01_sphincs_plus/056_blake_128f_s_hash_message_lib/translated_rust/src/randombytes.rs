use std::fs::File;
use std::io::Read;
use std::sync::Mutex;

static URANDOM: Mutex<Option<File>> = Mutex::new(None);

pub fn randombytes(x: &mut [u8]) {
    let mut guard = URANDOM.lock().unwrap();
    if guard.is_none() {
        *guard = Some(File::open("/dev/urandom").expect("Failed to open /dev/urandom"));
    }
    let file = guard.as_mut().unwrap();
    let mut remaining = x;
    while !remaining.is_empty() {
        match file.read(remaining) {
            Ok(0) => continue,
            Ok(n) => remaining = &mut remaining[n..],
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue;
            }
        }
    }
}
