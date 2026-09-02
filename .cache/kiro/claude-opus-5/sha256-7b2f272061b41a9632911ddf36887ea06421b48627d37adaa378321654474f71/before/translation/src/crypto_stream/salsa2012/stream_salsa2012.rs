//! Translation of c_src/libsodium/crypto_stream/salsa2012/stream_salsa2012.c

use core::ffi::c_void;

const CRYPTO_STREAM_SALSA2012_KEYBYTES: usize = 32;
const CRYPTO_STREAM_SALSA2012_NONCEBYTES: usize = 8;
// crypto_stream_salsa2012_MESSAGEBYTES_MAX = SODIUM_SIZE_MAX
const CRYPTO_STREAM_SALSA2012_MESSAGEBYTES_MAX: usize = usize::MAX;

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa2012_keybytes() -> usize {
    CRYPTO_STREAM_SALSA2012_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa2012_noncebytes() -> usize {
    CRYPTO_STREAM_SALSA2012_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa2012_messagebytes_max() -> usize {
    CRYPTO_STREAM_SALSA2012_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa2012_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_STREAM_SALSA2012_KEYBYTES);
}
