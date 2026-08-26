//! Translation of `app/src/randombytes.c` (the `/dev/urandom` based
//! `randombytes`, used by the non-deterministic `sphincs_core` library).
//!
//! In the C project this `randombytes` lives in a *separate* shared library
//! (`sphincs_core`) from the deterministic DRBG `randombytes` in `rng.c`
//! (`sphincs_core_det`). Since a single Rust `cdylib` cannot export the same
//! `#[no_mangle]` symbol twice, and the driver links against the deterministic
//! variant, this faithful translation is exposed as an ordinary Rust function
//! (`randombytes_urandom`) rather than a `#[no_mangle]` export.

use std::fs::File;
use std::io::Read;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::sync::Mutex;

static FD: Mutex<i32> = Mutex::new(-1);

/// Fills `x` (a slice of `xlen` bytes) with data read from `/dev/urandom`,
/// mirroring `randombytes` from `app/src/randombytes.c`.
pub fn randombytes_urandom(x: &mut [u8]) {
    let mut fd_guard = FD.lock().unwrap();

    if *fd_guard == -1 {
        loop {
            match File::open("/dev/urandom") {
                Ok(f) => {
                    *fd_guard = f.into_raw_fd();
                    break;
                }
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        }
    }

    // SAFETY: FD holds a valid, open file descriptor for /dev/urandom.
    let mut file = unsafe { File::from_raw_fd(*fd_guard) };

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

    // Keep the descriptor open (as the C code does); don't close on drop.
    let _ = file.into_raw_fd();
}
