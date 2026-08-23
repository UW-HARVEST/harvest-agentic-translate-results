//! Translation of `crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c`.
//!
//! `ACQUIRE_FENCE` expands to `(void) 0` because neither
//! `HAVE_GCC_MEMORY_FENCES` nor `HAVE_C11_MEMORY_FENCES` is defined in the
//! reference build.

use core::ffi::{c_int, c_void};

use crate::common::SODIUM_SIZE_MAX;
use crate::randombytes::randombytes_buf;

unsafe extern "C" {
    fn crypto_stream_xsalsa20(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
    fn crypto_stream_xsalsa20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305(
        out: *mut u8,
        in_: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_verify(
        h: *const u8,
        in_: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> c_int;
}

pub const crypto_secretbox_xsalsa20poly1305_KEYBYTES: usize = 32;
pub const crypto_secretbox_xsalsa20poly1305_NONCEBYTES: usize = 24;
pub const crypto_secretbox_xsalsa20poly1305_MACBYTES: usize = 16;
pub const crypto_secretbox_xsalsa20poly1305_BOXZEROBYTES: usize = 16;
pub const crypto_secretbox_xsalsa20poly1305_ZEROBYTES: usize =
    crypto_secretbox_xsalsa20poly1305_BOXZEROBYTES + crypto_secretbox_xsalsa20poly1305_MACBYTES;
/// `crypto_stream_xsalsa20_MESSAGEBYTES_MAX - crypto_secretbox_xsalsa20poly1305_MACBYTES`
pub const crypto_secretbox_xsalsa20poly1305_MESSAGEBYTES_MAX: u64 =
    SODIUM_SIZE_MAX - crypto_secretbox_xsalsa20poly1305_MACBYTES as u64;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut i: c_int;

    if mlen < 32 {
        return -1;
    }
    unsafe {
        crypto_stream_xsalsa20_xor(c, m, mlen, n, k);
        crypto_onetimeauth_poly1305(c.add(16), c.add(32), mlen - 32, c);
        i = 0;
        while i < 16 {
            *c.add(i as usize) = 0;
            i += 1;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut subkey: [u8; 32] = [0; 32];
    let mut i: c_int;

    if clen < 32 {
        return -1;
    }
    unsafe {
        crypto_stream_xsalsa20(subkey.as_mut_ptr(), 32, n, k);
        if crypto_onetimeauth_poly1305_verify(c.add(16), c.add(32), clen - 32, subkey.as_ptr()) != 0
        {
            return -1;
        }
        /* ACQUIRE_FENCE */
        crypto_stream_xsalsa20_xor(m, c, clen, n, k);
        i = 0;
        while i < 32 {
            *m.add(i as usize) = 0;
            i += 1;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_keybytes() -> usize {
    crypto_secretbox_xsalsa20poly1305_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_noncebytes() -> usize {
    crypto_secretbox_xsalsa20poly1305_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_zerobytes() -> usize {
    crypto_secretbox_xsalsa20poly1305_ZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_boxzerobytes() -> usize {
    crypto_secretbox_xsalsa20poly1305_BOXZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_macbytes() -> usize {
    crypto_secretbox_xsalsa20poly1305_MACBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_messagebytes_max() -> usize {
    crypto_secretbox_xsalsa20poly1305_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_keygen(k: *mut u8) {
    randombytes_buf(
        k as *mut c_void,
        crypto_secretbox_xsalsa20poly1305_KEYBYTES,
    );
}
