//! Translated from xsalsa20/stream_xsalsa20.c, xchacha20/stream_xchacha20.c, crypto_stream.c
use crate::primitives::cutil::*;
use core::ffi::{c_char, c_void};

extern "C" {
    fn crypto_core_hsalsa20(out: *mut u8, inp: *const u8, k: *const u8, c: *const u8) -> i32;
    fn crypto_core_hchacha20(out: *mut u8, inp: *const u8, k: *const u8, c: *const u8) -> i32;
    fn crypto_stream_salsa20(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> i32;
    fn crypto_stream_salsa20_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> i32;
    fn crypto_stream_chacha20(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> i32;
    fn crypto_stream_chacha20_xor_ic(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> i32;
}

#[inline(always)]
fn messagebytes_max() -> u64 {
    core::cmp::min(u64::MAX, usize::MAX as u64)
}

// ---- xsalsa20 ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xsalsa20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    let mut subkey = [0u8; 32];
    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());
    let ret = crypto_stream_salsa20(c, clen, n.add(16), subkey.as_ptr());
    sodium_memzero(subkey.as_mut_ptr() as *mut c_void, 32);
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
) -> i32 {
    let mut subkey = [0u8; 32];
    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());
    let ret = crypto_stream_salsa20_xor_ic(c, m, mlen, n.add(16), ic, subkey.as_ptr());
    sodium_memzero(subkey.as_mut_ptr() as *mut c_void, 32);
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xsalsa20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    crypto_stream_xsalsa20_xor_ic(c, m, mlen, n, 0, k)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_xsalsa20_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_xsalsa20_noncebytes() -> usize {
    24
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_xsalsa20_messagebytes_max() -> usize {
    messagebytes_max() as usize
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xsalsa20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}

// ---- xchacha20 ----

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_xchacha20_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_xchacha20_noncebytes() -> usize {
    24
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_xchacha20_messagebytes_max() -> usize {
    messagebytes_max() as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    let mut k2 = [0u8; 32];
    crypto_core_hchacha20(k2.as_mut_ptr(), n, k, core::ptr::null());
    crypto_stream_chacha20(c, clen, n.add(16), k2.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> i32 {
    let mut k2 = [0u8; 32];
    crypto_core_hchacha20(k2.as_mut_ptr(), n, k, core::ptr::null());
    crypto_stream_chacha20_xor_ic(c, m, mlen, n.add(16), ic, k2.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    crypto_stream_xchacha20_xor_ic(c, m, mlen, n, 0, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xchacha20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}

// ---- crypto_stream dispatch (xsalsa20) ----

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_noncebytes() -> usize {
    24
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_messagebytes_max() -> usize {
    messagebytes_max() as usize
}

static STREAM_PRIMITIVE: &[u8] = b"xsalsa20\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_primitive() -> *const c_char {
    STREAM_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    crypto_stream_xsalsa20(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    crypto_stream_xsalsa20_xor(c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}
