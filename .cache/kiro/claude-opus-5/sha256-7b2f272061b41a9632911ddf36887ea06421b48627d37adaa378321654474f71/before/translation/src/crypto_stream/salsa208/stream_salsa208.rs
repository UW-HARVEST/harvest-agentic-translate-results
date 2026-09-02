//! Translation of c_src/libsodium/crypto_stream/salsa208/stream_salsa208.c

use core::ffi::c_void;

const CRYPTO_STREAM_SALSA208_KEYBYTES: usize = 32;
const CRYPTO_STREAM_SALSA208_NONCEBYTES: usize = 8;
// crypto_stream_salsa208_MESSAGEBYTES_MAX = SODIUM_SIZE_MAX
const CRYPTO_STREAM_SALSA208_MESSAGEBYTES_MAX: usize = usize::MAX;

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

/* LCOV_EXCL_START */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_keybytes() -> usize {
    CRYPTO_STREAM_SALSA208_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_noncebytes() -> usize {
    CRYPTO_STREAM_SALSA208_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_messagebytes_max() -> usize {
    CRYPTO_STREAM_SALSA208_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_STREAM_SALSA208_KEYBYTES);
}

/* LCOV_EXCL_STOP */
