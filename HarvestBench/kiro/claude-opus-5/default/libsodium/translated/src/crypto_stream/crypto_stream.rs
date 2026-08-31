//! Translation of c_src/libsodium/crypto_stream/crypto_stream.c

use core::ffi::{c_char, c_int};

// crypto_stream_* constants alias the xsalsa20 ones.
const CRYPTO_STREAM_KEYBYTES: usize = 32; // crypto_stream_xsalsa20_KEYBYTES
const CRYPTO_STREAM_NONCEBYTES: usize = 24; // crypto_stream_xsalsa20_NONCEBYTES
// crypto_stream_MESSAGEBYTES_MAX = crypto_stream_xsalsa20_MESSAGEBYTES_MAX
// = SODIUM_SIZE_MAX = min(UINT64_MAX, SIZE_MAX). On x86_64 that is usize::MAX.
const CRYPTO_STREAM_MESSAGEBYTES_MAX: usize = usize::MAX;
// crypto_stream_PRIMITIVE "xsalsa20"
const CRYPTO_STREAM_PRIMITIVE: &[u8] = b"xsalsa20\0";

extern "C" {
    fn crypto_stream_xsalsa20(
        c: *mut u8,
        clen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_xsalsa20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn randombytes_buf(buf: *mut core::ffi::c_void, size: usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_keybytes() -> usize {
    CRYPTO_STREAM_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_noncebytes() -> usize {
    CRYPTO_STREAM_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_messagebytes_max() -> usize {
    CRYPTO_STREAM_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_primitive() -> *const c_char {
    CRYPTO_STREAM_PRIMITIVE.as_ptr() as *const c_char
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
    randombytes_buf(k as *mut core::ffi::c_void, CRYPTO_STREAM_KEYBYTES);
}
