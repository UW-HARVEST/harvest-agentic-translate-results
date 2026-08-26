//! Translation of `crypto_stream/xsalsa20/stream_xsalsa20.c`

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

extern "C" {
    fn crypto_core_hsalsa20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;
    fn crypto_stream_salsa20(c: *mut u8, clen: c_ulonglong, n: *const u8, k: *const u8) -> c_int;
    fn crypto_stream_salsa20_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

const crypto_stream_xsalsa20_KEYBYTES: usize = 32;
const crypto_stream_xsalsa20_NONCEBYTES: usize = 24;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xsalsa20(
    c: *mut u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut subkey: [u8; 32] = [0; 32];
    let ret: c_int;

    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());
    ret = crypto_stream_salsa20(c, clen, n.add(16), subkey.as_ptr());
    sodium_memzero(subkey.as_mut_ptr() as *mut c_void, 32);

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xsalsa20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    let mut subkey: [u8; 32] = [0; 32];
    let ret: c_int;

    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());
    ret = crypto_stream_salsa20_xor_ic(c, m, mlen, n.add(16), ic, subkey.as_ptr());
    sodium_memzero(subkey.as_mut_ptr() as *mut c_void, 32);

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xsalsa20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_stream_xsalsa20_xor_ic(c, m, mlen, n, 0, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xsalsa20_keybytes() -> usize {
    crypto_stream_xsalsa20_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xsalsa20_noncebytes() -> usize {
    crypto_stream_xsalsa20_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xsalsa20_messagebytes_max() -> usize {
    SODIUM_SIZE_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xsalsa20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_xsalsa20_KEYBYTES);
}
