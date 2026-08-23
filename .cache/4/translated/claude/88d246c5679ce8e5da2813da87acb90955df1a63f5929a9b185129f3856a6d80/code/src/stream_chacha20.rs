//! Translation of `crypto_stream/chacha20/stream_chacha20.c`
//!
//! The reference build defines none of `HAVE_AVX512FINTRIN_H`,
//! `HAVE_AVX2INTRIN_H`, `HAVE_EMMINTRIN_H`, `HAVE_TMMINTRIN_H`,
//! `HAVE_SMMINTRIN_H`, and is not aarch64/NEON, so all of the dolbeau
//! implementations are compiled out and only the reference one remains.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use crate::common::SODIUM_SIZE_MAX;
use core::ffi::{c_int, c_ulonglong, c_void};

/* `struct crypto_stream_chacha20_implementation` from
 * `crypto_stream/chacha20/stream_chacha20.h` (private header, duplicated here). */
#[repr(C)]
pub struct crypto_stream_chacha20_implementation {
    pub stream: unsafe extern "C" fn(
        c: *mut u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int,
    pub stream_ietf_ext: unsafe extern "C" fn(
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
    pub stream_ietf_ext_xor_ic: unsafe extern "C" fn(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        ic: u32,
        k: *const u8,
    ) -> c_int,
}

extern "C" {
    /* crypto_stream/chacha20/ref/chacha20_ref.c */
    static crypto_stream_chacha20_ref_implementation: crypto_stream_chacha20_implementation;
    /* sodium/core.c */
    fn sodium_misuse() -> !;
    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

const crypto_stream_chacha20_KEYBYTES: usize = 32;
const crypto_stream_chacha20_NONCEBYTES: usize = 8;
const crypto_stream_chacha20_MESSAGEBYTES_MAX: u64 = SODIUM_SIZE_MAX;

const crypto_stream_chacha20_ietf_KEYBYTES: usize = 32;
const crypto_stream_chacha20_ietf_NONCEBYTES: usize = 12;
/* SODIUM_MIN(SODIUM_SIZE_MAX, 64ULL * (1ULL << 32)) */
const crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX: u64 =
    if SODIUM_SIZE_MAX < 64u64 * (1u64 << 32) {
        SODIUM_SIZE_MAX
    } else {
        64u64 * (1u64 << 32)
    };

static mut implementation: *const crypto_stream_chacha20_implementation =
    unsafe { &crypto_stream_chacha20_ref_implementation };

#[inline(always)]
unsafe fn imp() -> &'static crypto_stream_chacha20_implementation {
    &*implementation
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_keybytes() -> usize {
    crypto_stream_chacha20_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_noncebytes() -> usize {
    crypto_stream_chacha20_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_messagebytes_max() -> usize {
    crypto_stream_chacha20_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_keybytes() -> usize {
    crypto_stream_chacha20_ietf_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_noncebytes() -> usize {
    crypto_stream_chacha20_ietf_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_messagebytes_max() -> usize {
    crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20(
    c: *mut u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > crypto_stream_chacha20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    (imp().stream)(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    if mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    (imp().stream_xor_ic)(c, m, mlen, n, ic, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    (imp().stream_xor_ic)(c, m, mlen, n, 0u64, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_ext(
    c: *mut u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > crypto_stream_chacha20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    (imp().stream_ietf_ext)(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_ext_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    ic: u32,
    k: *const u8,
) -> c_int {
    if mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    (imp().stream_ietf_ext_xor_ic)(c, m, mlen, n, ic, k)
}

unsafe fn crypto_stream_chacha20_ietf_ext_xor(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    (imp().stream_ietf_ext_xor_ic)(c, m, mlen, n, 0u32, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf(
    c: *mut u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf_ext(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    ic: u32,
    k: *const u8,
) -> c_int {
    /* if ((unsigned long long) ic >
     *     (64ULL * (1ULL << 32)) / 64ULL - (mlen + 63ULL) / 64ULL) */
    if (ic as c_ulonglong)
        > ((64u64 * (1u64 << 32)) / 64u64).wrapping_sub(mlen.wrapping_add(63u64) / 64u64)
    {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf_ext_xor_ic(c, m, mlen, n, ic, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_xor(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf_ext_xor(c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_keygen(k: *mut u8) {
    randombytes_buf(
        k as *mut c_void,
        crypto_stream_chacha20_ietf_KEYBYTES,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_chacha20_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_stream_chacha20_pick_best_implementation() -> c_int {
    implementation = &crypto_stream_chacha20_ref_implementation;
    /* No AVX512 / AVX2 / SSSE3 / NEON implementations in this build. */
    0
}
