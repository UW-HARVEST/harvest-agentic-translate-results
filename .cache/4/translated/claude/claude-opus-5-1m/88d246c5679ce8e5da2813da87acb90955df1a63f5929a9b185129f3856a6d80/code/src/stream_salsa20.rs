//! Translation of `crypto_stream/salsa20/stream_salsa20.c`
//!
//! The reference build defines none of `HAVE_AMD64_ASM`, `HAVE_EMMINTRIN_H`,
//! `HAVE_AVX2INTRIN_H`, `HAVE_AVX512FINTRIN_H`, `__ARM_NEON`, ... so the only
//! implementation that exists is the portable reference one, and
//! `_crypto_stream_salsa20_pick_best_implementation()` unconditionally selects
//! it.

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/// `typedef struct crypto_stream_salsa20_implementation` from
/// `crypto_stream/salsa20/stream_salsa20.h`.
#[repr(C)]
pub struct crypto_stream_salsa20_implementation {
    pub stream: unsafe extern "C" fn(
        c: *mut u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int,
    pub stream_xor_ic: unsafe extern "C" fn(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> c_int,
}

extern "C" {
    /// Defined in `crypto_stream/salsa20/ref/salsa20_ref.c`.
    static crypto_stream_salsa20_ref_implementation: crypto_stream_salsa20_implementation;

    fn randombytes_buf(buf: *mut c_void, size: usize);
}

const crypto_stream_salsa20_KEYBYTES: usize = 32;
const crypto_stream_salsa20_NONCEBYTES: usize = 8;

static mut implementation: *const crypto_stream_salsa20_implementation =
    core::ptr::addr_of!(crypto_stream_salsa20_ref_implementation);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_keybytes() -> usize {
    crypto_stream_salsa20_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_noncebytes() -> usize {
    crypto_stream_salsa20_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_messagebytes_max() -> usize {
    SODIUM_SIZE_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20(
    c: *mut u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    ((*implementation).stream)(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    ((*implementation).stream_xor_ic)(c, m, mlen, n, ic, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    ((*implementation).stream_xor_ic)(c, m, mlen, n, 0, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_salsa20_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_stream_salsa20_pick_best_implementation() -> c_int {
    implementation = core::ptr::addr_of!(crypto_stream_salsa20_ref_implementation);

    0 /* LCOV_EXCL_LINE */
}
