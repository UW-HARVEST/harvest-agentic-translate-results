//! Translation of `app/src/randombytes.c` and `app/include/randombytes.h`.
//!
//! `app/CMakeLists.txt` builds two shared libraries out of the same core
//! objects: `sphincs_core` links `randombytes.c` (this file) and
//! `sphincs_core_det` links `rng.c`.  Both define a symbol named
//! `randombytes`, so only one of them can be exported at a time; the `urandom`
//! Cargo feature selects this one, and the default matches the `driver`
//! executable, which links the deterministic core.

use core::ffi::c_ulonglong;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::{FromRawFd, IntoRawFd};

static mut FD: i32 = -1;

/// `void randombytes(unsigned char *x, unsigned long long xlen)`
///
/// Reads from `/dev/urandom`, retrying forever (with a one second sleep) until
/// the requested number of bytes has been produced.
pub fn randombytes_urandom(x: &mut [u8]) {
    let fd = unsafe { &mut *core::ptr::addr_of_mut!(FD) };

    if *fd == -1 {
        loop {
            match File::open("/dev/urandom") {
                Ok(f) => {
                    *fd = f.into_raw_fd();
                    break;
                }
                Err(_) => std::thread::sleep(std::time::Duration::from_secs(1)),
            }
        }
    }

    let mut file = unsafe { std::mem::ManuallyDrop::new(File::from_raw_fd(*fd)) };

    let mut off = 0usize;
    let mut xlen = x.len();
    while xlen > 0 {
        let i = if xlen < 1048576 { xlen } else { 1048576 };

        let n = match file.read(&mut x[off..off + i]) {
            Ok(n) => n,
            Err(_) => 0,
        };
        if n < 1 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }

        off += n;
        xlen -= n;
    }
}

/// The `randombytes()` used by `sign.c`, resolved the same way the CMake
/// targets resolve it at link time.
#[inline]
pub fn randombytes_rs(x: &mut [u8]) {
    #[cfg(rand_urandom)]
    {
        randombytes_urandom(x);
    }
    #[cfg(not(rand_urandom))]
    {
        crate::rng::randombytes_drbg(x);
    }
}

#[cfg(rand_urandom)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: c_ulonglong) {
    let s = core::slice::from_raw_parts_mut(x, xlen as usize);
    randombytes_urandom(s);
}

#[cfg(not(rand_urandom))]
#[allow(dead_code)]
pub unsafe fn randombytes_nondet(x: *mut u8, xlen: c_ulonglong) {
    let s = core::slice::from_raw_parts_mut(x, xlen as usize);
    randombytes_urandom(s);
}
