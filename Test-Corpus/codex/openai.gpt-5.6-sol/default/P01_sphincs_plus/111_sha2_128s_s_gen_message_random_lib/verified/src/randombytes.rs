//! Translation of `app/src/randombytes.c` (the `/dev/urandom` implementation).
//!
//! In the C project this file is compiled into the *non-deterministic*
//! `sphincs_core` library, while the driver links the *deterministic* core
//! (`rng.c`). Both define a symbol named `randombytes`; since a single Rust
//! artifact cannot export the symbol twice, this complete translation is
//! provided under the name `randombytes_urandom` (the deterministic
//! `randombytes` from `rng.rs` is the one exported for the C ABI, matching the
//! driver's behaviour).

use std::fs::File;
use std::io::Read;
use std::sync::Mutex;

static FD: Mutex<Option<File>> = Mutex::new(None);

pub fn randombytes_urandom(x: &mut [u8]) {
    let mut guard = FD.lock().unwrap();
    if guard.is_none() {
        loop {
            match File::open("/dev/urandom") {
                Ok(f) => {
                    *guard = Some(f);
                    break;
                }
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        }
    }

    let file = guard.as_mut().unwrap();
    let mut xlen = x.len();
    let mut off = 0usize;
    while xlen > 0 {
        let want = if xlen < 1_048_576 { xlen } else { 1_048_576 };
        match file.read(&mut x[off..off + want]) {
            Ok(i) if i >= 1 => {
                off += i;
                xlen -= i;
            }
            _ => {
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue;
            }
        }
    }
}
