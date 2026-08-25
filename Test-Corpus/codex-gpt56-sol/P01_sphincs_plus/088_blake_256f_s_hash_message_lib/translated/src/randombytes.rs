//! Translation of `app/src/randombytes.c` / `app/include/randombytes.h` — the
//! non-deterministic `/dev/urandom` based `randombytes()` of the SPHINCS+
//! reference implementation (public domain).
//!
//! In C this file is compiled into a *separate* shared library (`sphincs_core`)
//! than `rng.c` (`sphincs_core_det`), so both may define the symbol
//! `randombytes`.  A single Rust cdylib cannot export that symbol twice and the
//! KAT driver links the deterministic DRBG variant, therefore this translation
//! keeps the full logic but lives under a distinct, non-exported Rust name.

use std::fs::File;
use std::io::Read;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// `static int fd = -1;` — the lazily opened, process-wide `/dev/urandom`
/// handle.  `None` corresponds to C's `fd == -1`.
fn urandom_fd() -> &'static Mutex<Option<File>> {
    static FD: OnceLock<Mutex<Option<File>>> = OnceLock::new();
    FD.get_or_init(|| Mutex::new(None))
}

/// Translation of `app/src/randombytes.c` (`void randombytes(unsigned char *x, unsigned long long xlen)`).
/// Exported under a distinct Rust name because `rng.rs` owns the `randombytes` linker symbol.
///
/// ```c
/// void randombytes(unsigned char *x, unsigned long long xlen)
/// {
///     unsigned long long i;
///
///     if (fd == -1) {
///         for (;;) {
///             fd = open("/dev/urandom", O_RDONLY);
///             if (fd != -1) {
///                 break;
///             }
///             sleep(1);
///         }
///     }
///
///     while (xlen > 0) {
///         if (xlen < 1048576) {
///             i = xlen;
///         }
///         else {
///             i = 1048576;
///         }
///
///         i = (unsigned long long)read(fd, x, i);
///         if (i < 1) {
///             sleep(1);
///             continue;
///         }
///
///         x += i;
///         xlen -= i;
///     }
/// }
/// ```
pub unsafe extern "C" fn randombytes_urandom(x: *mut u8, xlen: u64) {
    let mut x = x;
    let mut xlen = xlen;
    let mut i: u64;

    let fd = urandom_fd();
    // A poisoned lock still holds a perfectly usable handle (C has no lock at
    // all), so recover from it instead of panicking.
    let mut guard = match fd.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    if guard.is_none() {
        loop {
            match File::open("/dev/urandom") {
                Ok(file) => {
                    *guard = Some(file);
                    break;
                }
                Err(_) => std::thread::sleep(Duration::from_secs(1)),
            }
        }
    }
    let file = match guard.as_mut() {
        Some(file) => file,
        // Unreachable: the loop above only exits with an open handle.
        None => return,
    };

    while xlen > 0 {
        if xlen < 1048576 {
            i = xlen;
        } else {
            i = 1048576;
        }

        let buf = core::slice::from_raw_parts_mut(x, i as usize);
        // `read()` failures are treated like a short read (C's `i < 1`): sleep
        // and retry.  Casting C's -1 to `unsigned long long` would otherwise
        // walk the pointer off into space.
        i = match file.read(buf) {
            Ok(n) => n as u64,
            Err(_) => 0,
        };
        if i < 1 {
            std::thread::sleep(Duration::from_secs(1));
            continue;
        }

        x = x.add(i as usize);
        xlen -= i;
    }
}
