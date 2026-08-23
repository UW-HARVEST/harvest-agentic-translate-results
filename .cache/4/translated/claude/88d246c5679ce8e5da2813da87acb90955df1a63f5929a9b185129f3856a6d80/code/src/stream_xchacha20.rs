//! Translation of `crypto_stream/xchacha20/stream_xchacha20.c`

#![allow(non_upper_case_globals)]

use core::ffi::{c_int, c_ulonglong, c_void};

extern "C" {
    /* crypto_core/hchacha20/core_hchacha20.c */
    fn crypto_core_hchacha20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;
    /* crypto_stream/chacha20/stream_chacha20.c */
    fn crypto_stream_chacha20(
        c: *mut u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_chacha20_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> c_int;
    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

const crypto_core_hchacha20_OUTPUTBYTES: usize = 32;
const crypto_core_hchacha20_INPUTBYTES: usize = 16;

const crypto_stream_chacha20_KEYBYTES: usize = 32;
const crypto_stream_chacha20_NONCEBYTES: usize = 8;

const crypto_stream_xchacha20_KEYBYTES: usize = 32;
const crypto_stream_xchacha20_NONCEBYTES: usize = 24;
/* SODIUM_SIZE_MAX */
const crypto_stream_xchacha20_MESSAGEBYTES_MAX: u64 = crate::common::SODIUM_SIZE_MAX;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_keybytes() -> usize {
    crypto_stream_xchacha20_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_noncebytes() -> usize {
    crypto_stream_xchacha20_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_messagebytes_max() -> usize {
    crypto_stream_xchacha20_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20(
    c: *mut u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut k2: [u8; crypto_core_hchacha20_OUTPUTBYTES] = [0; crypto_core_hchacha20_OUTPUTBYTES];

    crypto_core_hchacha20(k2.as_mut_ptr(), n, k, core::ptr::null());
    const _: () = assert!(crypto_stream_chacha20_KEYBYTES <= crypto_core_hchacha20_OUTPUTBYTES);
    const _: () = assert!(
        crypto_stream_chacha20_NONCEBYTES
            == crypto_stream_xchacha20_NONCEBYTES - crypto_core_hchacha20_INPUTBYTES
    );

    crypto_stream_chacha20(
        c,
        clen,
        n.add(crypto_core_hchacha20_INPUTBYTES),
        k2.as_ptr(),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    let mut k2: [u8; crypto_core_hchacha20_OUTPUTBYTES] = [0; crypto_core_hchacha20_OUTPUTBYTES];

    crypto_core_hchacha20(k2.as_mut_ptr(), n, k, core::ptr::null());
    crypto_stream_chacha20_xor_ic(
        c,
        m,
        mlen,
        n.add(crypto_core_hchacha20_INPUTBYTES),
        ic,
        k2.as_ptr(),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_stream_xchacha20_xor_ic(c, m, mlen, n, 0u64, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_xchacha20_KEYBYTES);
}
