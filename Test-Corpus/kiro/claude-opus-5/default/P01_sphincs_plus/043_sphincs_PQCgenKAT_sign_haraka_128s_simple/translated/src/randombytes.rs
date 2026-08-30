//! Translation of `app/src/randombytes.c` and `app/include/randombytes.h`.
//!
//! This code was taken from the SPHINCS reference implementation and is public
//! domain.

use std::fs::File;
use std::io::Read;
use std::sync::Mutex;
use std::sync::OnceLock;

/// Stands in for `static int fd = -1;`.
fn urandom() -> &'static Mutex<File> {
    static FD: OnceLock<Mutex<File>> = OnceLock::new();
    FD.get_or_init(|| {
        loop {
            match File::open("/dev/urandom") {
                Ok(f) => return Mutex::new(f),
                Err(_) => std::thread::sleep(std::time::Duration::from_secs(1)),
            }
        }
    })
}

/// The `randombytes()` from `randombytes.c`.
pub fn randombytes_urandom(x: &mut [u8]) {
    let file = urandom();
    let mut guard = file.lock().unwrap();

    let mut off = 0usize;
    let mut xlen = x.len();

    while xlen > 0 {
        let want = if xlen < 1048576 { xlen } else { 1048576 };

        let i = match guard.read(&mut x[off..off + want]) {
            Ok(i) => i,
            Err(_) => 0,
        };
        if i < 1 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }

        off += i;
        xlen -= i;
    }
}

/// `void randombytes(unsigned char *x, unsigned long long xlen)`
///
/// Exported only with the `urandom` feature; see [`crate::rng::randombytes`]
/// for the default (deterministic) implementation and the note in
/// `Cargo.toml`.
#[cfg(feature = "urandom")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: core::ffi::c_ulonglong) {
    unsafe { randombytes_urandom(core::slice::from_raw_parts_mut(x, xlen as usize)) }
}
