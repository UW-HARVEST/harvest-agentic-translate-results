//! Translation of `app/src/randombytes.c` (the `sphincs_core` variant).
//!
//! > This code was taken from the SPHINCS reference implementation and is
//! > public domain.
//!
//! Not exported with `#[no_mangle]` because `rng.rs` owns the `randombytes`
//! linker symbol in this crate (CMake builds these two `randombytes`
//! implementations into two separate `.so` files: `sphincs_core` uses this
//! `/dev/urandom` reader, `sphincs_core_det` uses the NIST KAT DRBG in
//! `rng.rs`).  The driver binary links the deterministic variant.

use std::fs::File;
use std::io::Read;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::thread::sleep;
use std::time::Duration;

/// `static int fd = -1;` -- the file-scope descriptor cached across calls.
static mut FD: i32 = -1;

#[inline]
fn fd_ref() -> &'static mut i32 {
    unsafe { &mut *core::ptr::addr_of_mut!(FD) }
}

/// `void randombytes(unsigned char *x, unsigned long long xlen)`
///
/// Reads `x.len()` bytes from `/dev/urandom`, in chunks of at most 1 MiB,
/// retrying (after a one second sleep) on open failure or a short/failed read,
/// exactly like the C original.
pub fn randombytes(x: &mut [u8]) {
    let fd = fd_ref();

    if *fd == -1 {
        loop {
            // open("/dev/urandom", O_RDONLY)
            match File::open("/dev/urandom") {
                Ok(f) => {
                    *fd = f.into_raw_fd();
                    break;
                }
                Err(_) => {
                    sleep(Duration::from_secs(1));
                }
            }
        }
    }

    // Wrap the raw fd without taking ownership; the C code keeps it open
    // forever, so the `File` must not close it on drop.
    let mut file = core::mem::ManuallyDrop::new(unsafe { File::from_raw_fd(*fd) });

    let mut offset: usize = 0;
    let mut xlen: u64 = x.len() as u64;

    while xlen > 0 {
        let mut i: u64 = if xlen < 1048576 { xlen } else { 1048576 };

        // i = (unsigned long long)read(fd, x, i);
        //
        // A failed read() yields (unsigned long long)(-1), which is huge and
        // therefore `>= 1`; the C code would then advance `x` by that amount,
        // which is undefined behaviour.  /dev/urandom reads do not fail in
        // practice, so we treat an error as the `i < 1` retry case, which is
        // what the C code does for a zero-length read.
        i = match file.read(&mut x[offset..offset + i as usize]) {
            Ok(n) => n as u64,
            Err(_) => {
                sleep(Duration::from_secs(1));
                continue;
            }
        };
        if i < 1 {
            sleep(Duration::from_secs(1));
            continue;
        }

        offset += i as usize;
        xlen -= i;
    }
}

/// Raw-pointer form matching the C prototype
/// `void randombytes(unsigned char *x, unsigned long long xlen)`.
///
/// Deliberately *not* `#[no_mangle]`: see the module docs.
///
/// # Safety
/// `x` must point to at least `xlen` writable bytes.
pub unsafe extern "C" fn randombytes_urandom(x: *mut u8, xlen: u64) {
    let xs = core::slice::from_raw_parts_mut(x, xlen as usize);
    randombytes(xs);
}
