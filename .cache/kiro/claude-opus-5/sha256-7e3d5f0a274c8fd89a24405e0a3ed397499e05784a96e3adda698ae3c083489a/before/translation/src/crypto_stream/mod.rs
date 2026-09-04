pub mod chacha20;
pub mod salsa20;
pub mod salsa2012;
pub mod salsa208;
pub mod xchacha20;
pub mod xsalsa20;

// Translation of crypto_stream/crypto_stream.c.

use core::ffi::{c_char, c_int, c_void};

use crate::common::SODIUM_SIZE_MAX;
use crate::crypto_stream::xsalsa20::{crypto_stream_xsalsa20, crypto_stream_xsalsa20_xor};
use crate::randombytes::randombytes_buf;

pub const crypto_stream_KEYBYTES: usize = xsalsa20::crypto_stream_xsalsa20_KEYBYTES;
pub const crypto_stream_NONCEBYTES: usize = xsalsa20::crypto_stream_xsalsa20_NONCEBYTES;
pub const crypto_stream_MESSAGEBYTES_MAX: usize = SODIUM_SIZE_MAX;

const crypto_stream_PRIMITIVE: &[u8] = b"xsalsa20\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_keybytes() -> usize {
    crypto_stream_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_noncebytes() -> usize {
    crypto_stream_NONCEBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_messagebytes_max() -> usize {
    crypto_stream_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_primitive() -> *const c_char {
    crypto_stream_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_stream_xsalsa20(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_stream_xsalsa20_xor(c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_KEYBYTES);
}
