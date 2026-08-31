//! Translation of c_src/libsodium/crypto_secretbox/crypto_secretbox.c

use core::ffi::{c_char, c_int, c_void};

const CRYPTO_SECRETBOX_KEYBYTES: usize = 32; // xsalsa20poly1305_KEYBYTES
const CRYPTO_SECRETBOX_NONCEBYTES: usize = 24; // xsalsa20poly1305_NONCEBYTES
const CRYPTO_SECRETBOX_MACBYTES: usize = 16; // xsalsa20poly1305_MACBYTES
const CRYPTO_SECRETBOX_BOXZEROBYTES: usize = 16; // xsalsa20poly1305_BOXZEROBYTES
const CRYPTO_SECRETBOX_ZEROBYTES: usize = 32; // BOXZEROBYTES + MACBYTES
// (crypto_stream_xsalsa20_MESSAGEBYTES_MAX - MACBYTES) == SODIUM_SIZE_MAX - MACBYTES
const CRYPTO_SECRETBOX_MESSAGEBYTES_MAX: usize = usize::MAX - CRYPTO_SECRETBOX_MACBYTES;

// crypto_secretbox_PRIMITIVE "xsalsa20poly1305"
const CRYPTO_SECRETBOX_PRIMITIVE: &[u8] = b"xsalsa20poly1305\0";

extern "C" {
    fn crypto_secretbox_xsalsa20poly1305(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_secretbox_xsalsa20poly1305_open(
        m: *mut u8,
        c: *const u8,
        clen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_keybytes() -> usize {
    CRYPTO_SECRETBOX_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_noncebytes() -> usize {
    CRYPTO_SECRETBOX_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_zerobytes() -> usize {
    CRYPTO_SECRETBOX_ZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_boxzerobytes() -> usize {
    CRYPTO_SECRETBOX_BOXZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_macbytes() -> usize {
    CRYPTO_SECRETBOX_MACBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_messagebytes_max() -> usize {
    CRYPTO_SECRETBOX_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_primitive() -> *const c_char {
    CRYPTO_SECRETBOX_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_xsalsa20poly1305(c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_xsalsa20poly1305_open(m, c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_SECRETBOX_KEYBYTES);
}
