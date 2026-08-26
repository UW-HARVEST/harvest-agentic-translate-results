pub mod easy;
pub mod xchacha20poly1305;
pub mod xsalsa20poly1305;

// Translation of `crypto_secretbox/crypto_secretbox.c`.

use core::ffi::{c_char, c_int, c_void};

use crate::randombytes::randombytes_buf;

use self::xsalsa20poly1305::{
    crypto_secretbox_xsalsa20poly1305, crypto_secretbox_xsalsa20poly1305_open,
    crypto_secretbox_xsalsa20poly1305_BOXZEROBYTES as crypto_secretbox_BOXZEROBYTES,
    crypto_secretbox_xsalsa20poly1305_KEYBYTES as crypto_secretbox_KEYBYTES,
    crypto_secretbox_xsalsa20poly1305_MACBYTES as crypto_secretbox_MACBYTES,
    crypto_secretbox_xsalsa20poly1305_MESSAGEBYTES_MAX as crypto_secretbox_MESSAGEBYTES_MAX,
    crypto_secretbox_xsalsa20poly1305_NONCEBYTES as crypto_secretbox_NONCEBYTES,
    crypto_secretbox_xsalsa20poly1305_ZEROBYTES as crypto_secretbox_ZEROBYTES,
};

/// `#define crypto_secretbox_PRIMITIVE "xsalsa20poly1305"`
static crypto_secretbox_PRIMITIVE: [u8; 17] = *b"xsalsa20poly1305\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_keybytes() -> usize {
    crypto_secretbox_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_noncebytes() -> usize {
    crypto_secretbox_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_zerobytes() -> usize {
    crypto_secretbox_ZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_boxzerobytes() -> usize {
    crypto_secretbox_BOXZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_macbytes() -> usize {
    crypto_secretbox_MACBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_messagebytes_max() -> usize {
    crypto_secretbox_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_primitive() -> *const c_char {
    crypto_secretbox_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    unsafe { crypto_secretbox_xsalsa20poly1305(c, m, mlen, n, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    unsafe { crypto_secretbox_xsalsa20poly1305_open(m, c, clen, n, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_secretbox_KEYBYTES);
}
