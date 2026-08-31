//! Translation of c_src/libsodium/crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c

use core::ffi::{c_int, c_void};

const CRYPTO_SECRETBOX_XSALSA20POLY1305_KEYBYTES: usize = 32;
const CRYPTO_SECRETBOX_XSALSA20POLY1305_NONCEBYTES: usize = 24;
const CRYPTO_SECRETBOX_XSALSA20POLY1305_MACBYTES: usize = 16;
const CRYPTO_SECRETBOX_XSALSA20POLY1305_BOXZEROBYTES: usize = 16;
const CRYPTO_SECRETBOX_XSALSA20POLY1305_ZEROBYTES: usize = 32; // BOXZEROBYTES + MACBYTES
// (crypto_stream_xsalsa20_MESSAGEBYTES_MAX - MACBYTES) == SODIUM_SIZE_MAX - MACBYTES
const CRYPTO_SECRETBOX_XSALSA20POLY1305_MESSAGEBYTES_MAX: usize =
    usize::MAX - CRYPTO_SECRETBOX_XSALSA20POLY1305_MACBYTES;

extern "C" {
    fn crypto_stream_xsalsa20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_xsalsa20(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
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
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

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
    crypto_stream_xsalsa20_xor(c, m, mlen, n, k);
    crypto_onetimeauth_poly1305(c.add(16), c.add(32), mlen - 32, c);
    i = 0;
    while i < 16 {
        *c.add(i as usize) = 0;
        i += 1;
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
    let mut subkey = [0u8; 32];
    let mut i: c_int;

    if clen < 32 {
        return -1;
    }
    crypto_stream_xsalsa20(subkey.as_mut_ptr(), 32, n, k);
    if crypto_onetimeauth_poly1305_verify(c.add(16), c.add(32), clen - 32, subkey.as_ptr()) != 0 {
        return -1;
    }
    // ACQUIRE_FENCE;
    crypto_stream_xsalsa20_xor(m, c, clen, n, k);
    i = 0;
    while i < 32 {
        *m.add(i as usize) = 0;
        i += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_keybytes() -> usize {
    CRYPTO_SECRETBOX_XSALSA20POLY1305_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_noncebytes() -> usize {
    CRYPTO_SECRETBOX_XSALSA20POLY1305_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_zerobytes() -> usize {
    CRYPTO_SECRETBOX_XSALSA20POLY1305_ZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_boxzerobytes() -> usize {
    CRYPTO_SECRETBOX_XSALSA20POLY1305_BOXZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_macbytes() -> usize {
    CRYPTO_SECRETBOX_XSALSA20POLY1305_MACBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_messagebytes_max() -> usize {
    CRYPTO_SECRETBOX_XSALSA20POLY1305_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_SECRETBOX_XSALSA20POLY1305_KEYBYTES);
}
