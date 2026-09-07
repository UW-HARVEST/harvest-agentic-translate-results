//! Translated from `app/src/randombytes.c` (public-domain SPHINCS reference).
//!
//! This is the `/dev/urandom`-based `randombytes`. Its C linker symbol is
//! `randombytes`, which would COLLIDE with the deterministic `randombytes`
//! defined in `rng.c` (translated in `crate::rng`). In the original CMake
//! build these two live in two separate shared libraries
//! (`sphincs_core` vs `sphincs_core_det`); the `driver` executable links the
//! deterministic one. Our crate builds a single library that uses the
//! deterministic `rng.c` `randombytes`, so to avoid the symbol clash this
//! function is renamed `randombytes_urandom` and carries NO
//! `#[unsafe(no_mangle)]`. Nothing in the crate calls it.

use std::io::Read;
use std::sync::Mutex;

// Cached file handle, mirroring the C `static int fd = -1;`.
static FD: Mutex<Option<std::fs::File>> = Mutex::new(None);

#[allow(dead_code)]
pub unsafe fn randombytes_urandom(x: *mut u8, xlen: core::ffi::c_ulonglong) {
    let mut x = x;
    let mut xlen = xlen;
    let mut i: core::ffi::c_ulonglong;

    let mut guard = FD.lock().unwrap();
    if guard.is_none() {
        loop {
            match std::fs::File::open("/dev/urandom") {
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

    while xlen > 0 {
        if xlen < 1048576 {
            i = xlen;
        } else {
            i = 1048576;
        }

        let file = guard.as_mut().unwrap();
        let buf = core::slice::from_raw_parts_mut(x, i as usize);
        let n = match file.read(buf) {
            Ok(n) => n as core::ffi::c_ulonglong,
            Err(_) => 0,
        };
        i = n;
        if i < 1 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }

        x = x.add(i as usize);
        xlen -= i;
    }
}
