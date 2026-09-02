//! Translation of crypto_stream/xchacha20/stream_xchacha20.c.

use core::ffi::{c_int, c_void};
use core::ptr;

use crate::common::SODIUM_SIZE_MAX;
use crate::crypto_core::hchacha20::crypto_core_hchacha20;
use crate::crypto_stream::chacha20::{crypto_stream_chacha20, crypto_stream_chacha20_xor_ic};
use crate::randombytes::randombytes_buf;

pub const crypto_stream_xchacha20_KEYBYTES: usize = 32;
pub const crypto_stream_xchacha20_NONCEBYTES: usize = 24;
pub const crypto_stream_xchacha20_MESSAGEBYTES_MAX: usize = SODIUM_SIZE_MAX;

const crypto_core_hchacha20_OUTPUTBYTES: usize = 32;
const crypto_core_hchacha20_INPUTBYTES: usize = 16;

const crypto_stream_chacha20_KEYBYTES: usize = 32;
const crypto_stream_chacha20_NONCEBYTES: usize = 8;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_xchacha20_keybytes() -> usize {
    crypto_stream_xchacha20_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_xchacha20_noncebytes() -> usize {
    crypto_stream_xchacha20_NONCEBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_xchacha20_messagebytes_max() -> usize {
    crypto_stream_xchacha20_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut k2: [u8; crypto_core_hchacha20_OUTPUTBYTES] =
        [0; crypto_core_hchacha20_OUTPUTBYTES];

    crypto_core_hchacha20(k2.as_mut_ptr(), n, k, ptr::null());
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
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    let mut k2: [u8; crypto_core_hchacha20_OUTPUTBYTES] =
        [0; crypto_core_hchacha20_OUTPUTBYTES];

    crypto_core_hchacha20(k2.as_mut_ptr(), n, k, ptr::null());
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
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_stream_xchacha20_xor_ic(c, m, mlen, n, 0u64, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_xchacha20_KEYBYTES);
}
