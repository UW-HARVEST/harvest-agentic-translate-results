//! Translation of c_src/libsodium/crypto_stream/xchacha20/stream_xchacha20.c

use core::ffi::{c_int, c_void};

// crypto_core_hchacha20_OUTPUTBYTES 32U, crypto_core_hchacha20_INPUTBYTES 16U
const CRYPTO_CORE_HCHACHA20_OUTPUTBYTES: usize = 32;
const CRYPTO_CORE_HCHACHA20_INPUTBYTES: usize = 16;

const CRYPTO_STREAM_XCHACHA20_KEYBYTES: usize = 32;
const CRYPTO_STREAM_XCHACHA20_NONCEBYTES: usize = 24;
// crypto_stream_xchacha20_MESSAGEBYTES_MAX = SODIUM_SIZE_MAX
const CRYPTO_STREAM_XCHACHA20_MESSAGEBYTES_MAX: usize = usize::MAX;

extern "C" {
    fn crypto_core_hchacha20(
        out: *mut u8,
        in_: *const u8,
        k: *const u8,
        c: *const u8,
    ) -> c_int;
    fn crypto_stream_chacha20(
        c: *mut u8,
        clen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_chacha20_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_keybytes() -> usize {
    CRYPTO_STREAM_XCHACHA20_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_noncebytes() -> usize {
    CRYPTO_STREAM_XCHACHA20_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_messagebytes_max() -> usize {
    CRYPTO_STREAM_XCHACHA20_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut k2: [u8; CRYPTO_CORE_HCHACHA20_OUTPUTBYTES] =
        [0u8; CRYPTO_CORE_HCHACHA20_OUTPUTBYTES];

    crypto_core_hchacha20(k2.as_mut_ptr(), n, k, core::ptr::null());
    // COMPILER_ASSERTs dropped.

    crypto_stream_chacha20(
        c,
        clen,
        n.add(CRYPTO_CORE_HCHACHA20_INPUTBYTES),
        k2.as_ptr(),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    let mut k2: [u8; CRYPTO_CORE_HCHACHA20_OUTPUTBYTES] =
        [0u8; CRYPTO_CORE_HCHACHA20_OUTPUTBYTES];

    crypto_core_hchacha20(k2.as_mut_ptr(), n, k, core::ptr::null());
    crypto_stream_chacha20_xor_ic(
        c,
        m,
        mlen,
        n.add(CRYPTO_CORE_HCHACHA20_INPUTBYTES),
        ic,
        k2.as_ptr(),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_stream_xchacha20_xor_ic(c, m, mlen, n, 0u64, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_STREAM_XCHACHA20_KEYBYTES);
}
