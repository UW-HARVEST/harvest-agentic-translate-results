//! Translation of crypto_stream/xsalsa20/stream_xsalsa20.c.

use core::ffi::{c_int, c_void};
use core::ptr;

use crate::common::SODIUM_SIZE_MAX;
use crate::crypto_core::hsalsa20::crypto_core_hsalsa20;
use crate::crypto_stream::salsa20::{crypto_stream_salsa20, crypto_stream_salsa20_xor_ic};
use crate::randombytes::randombytes_buf;
use crate::sodium_utils::sodium_memzero;

pub const crypto_stream_xsalsa20_KEYBYTES: usize = 32;
pub const crypto_stream_xsalsa20_NONCEBYTES: usize = 24;
pub const crypto_stream_xsalsa20_MESSAGEBYTES_MAX: usize = SODIUM_SIZE_MAX;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xsalsa20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut subkey: [u8; 32] = [0; 32];
    let ret: c_int;

    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, ptr::null());
    ret = crypto_stream_salsa20(c, clen, n.add(16), subkey.as_ptr());
    sodium_memzero(
        subkey.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&subkey),
    );

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xsalsa20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    let mut subkey: [u8; 32] = [0; 32];
    let ret: c_int;

    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, ptr::null());
    ret = crypto_stream_salsa20_xor_ic(c, m, mlen, n.add(16), ic, subkey.as_ptr());
    sodium_memzero(
        subkey.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&subkey),
    );

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xsalsa20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_stream_xsalsa20_xor_ic(c, m, mlen, n, 0u64, k)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_xsalsa20_keybytes() -> usize {
    crypto_stream_xsalsa20_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_xsalsa20_noncebytes() -> usize {
    crypto_stream_xsalsa20_NONCEBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_xsalsa20_messagebytes_max() -> usize {
    crypto_stream_xsalsa20_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xsalsa20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_xsalsa20_KEYBYTES);
}
