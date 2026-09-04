pub mod easy;
pub mod xchacha20poly1305;
pub mod xsalsa20poly1305;

// Translation of crypto_secretbox/crypto_secretbox.c and include/sodium/crypto_secretbox.h

use core::ffi::{c_char, c_int, c_void};

use crate::crypto_secretbox::xsalsa20poly1305::{
    crypto_secretbox_xsalsa20poly1305, crypto_secretbox_xsalsa20poly1305_BOXZEROBYTES,
    crypto_secretbox_xsalsa20poly1305_KEYBYTES, crypto_secretbox_xsalsa20poly1305_MACBYTES,
    crypto_secretbox_xsalsa20poly1305_MESSAGEBYTES_MAX,
    crypto_secretbox_xsalsa20poly1305_NONCEBYTES, crypto_secretbox_xsalsa20poly1305_ZEROBYTES,
    crypto_secretbox_xsalsa20poly1305_open,
};
use crate::randombytes::randombytes_buf;

pub const crypto_secretbox_KEYBYTES: usize = crypto_secretbox_xsalsa20poly1305_KEYBYTES;
pub const crypto_secretbox_NONCEBYTES: usize = crypto_secretbox_xsalsa20poly1305_NONCEBYTES;
pub const crypto_secretbox_MACBYTES: usize = crypto_secretbox_xsalsa20poly1305_MACBYTES;
pub const crypto_secretbox_PRIMITIVE: &[u8] = b"xsalsa20poly1305\0";
pub const crypto_secretbox_MESSAGEBYTES_MAX: usize =
    crypto_secretbox_xsalsa20poly1305_MESSAGEBYTES_MAX;
pub const crypto_secretbox_ZEROBYTES: usize = crypto_secretbox_xsalsa20poly1305_ZEROBYTES;
pub const crypto_secretbox_BOXZEROBYTES: usize = crypto_secretbox_xsalsa20poly1305_BOXZEROBYTES;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_keybytes() -> usize {
    crypto_secretbox_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_noncebytes() -> usize {
    crypto_secretbox_NONCEBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_zerobytes() -> usize {
    crypto_secretbox_ZEROBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_boxzerobytes() -> usize {
    crypto_secretbox_BOXZEROBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_macbytes() -> usize {
    crypto_secretbox_MACBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_messagebytes_max() -> usize {
    crypto_secretbox_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_secretbox_primitive() -> *const c_char {
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
    randombytes_buf(k as *mut c_void, crypto_secretbox_KEYBYTES);
}
