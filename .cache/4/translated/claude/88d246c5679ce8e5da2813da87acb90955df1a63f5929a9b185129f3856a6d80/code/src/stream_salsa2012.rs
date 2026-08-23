//! Translation of `crypto_stream/salsa2012/stream_salsa2012.c`

use crate::common::*;
use core::ffi::c_void;

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

const crypto_stream_salsa2012_KEYBYTES: usize = 32;
const crypto_stream_salsa2012_NONCEBYTES: usize = 8;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa2012_keybytes() -> usize {
    crypto_stream_salsa2012_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa2012_noncebytes() -> usize {
    crypto_stream_salsa2012_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa2012_messagebytes_max() -> usize {
    SODIUM_SIZE_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa2012_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_salsa2012_KEYBYTES);
}
