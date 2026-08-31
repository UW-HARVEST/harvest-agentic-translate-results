//! Translation of c_src/libsodium/crypto_stream/chacha20/stream_chacha20.c

use core::ffi::{c_int, c_void};

use crate::sodium::core::sodium_misuse;

// crypto_stream_chacha20_* constants
const CRYPTO_STREAM_CHACHA20_KEYBYTES: usize = 32;
const CRYPTO_STREAM_CHACHA20_NONCEBYTES: usize = 8;
// crypto_stream_chacha20_MESSAGEBYTES_MAX = SODIUM_SIZE_MAX = min(UINT64_MAX, SIZE_MAX)
const CRYPTO_STREAM_CHACHA20_MESSAGEBYTES_MAX: u64 = usize::MAX as u64;

const CRYPTO_STREAM_CHACHA20_IETF_KEYBYTES: usize = 32;
const CRYPTO_STREAM_CHACHA20_IETF_NONCEBYTES: usize = 12;
// crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX = min(SODIUM_SIZE_MAX, 64ULL*(1ULL<<32))
const CRYPTO_STREAM_CHACHA20_IETF_MESSAGEBYTES_MAX: u64 = {
    let a = usize::MAX as u64;
    let b = 64u64 * (1u64 << 32);
    if a < b {
        a
    } else {
        b
    }
};

// #[repr(C)] mirror of `crypto_stream_chacha20_implementation` from
// crypto_stream/chacha20/stream_chacha20.h.
#[repr(C)]
pub struct CryptoStreamChacha20Implementation {
    pub stream: Option<
        unsafe extern "C" fn(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int,
    >,
    pub stream_ietf_ext: Option<
        unsafe extern "C" fn(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int,
    >,
    pub stream_xor_ic: Option<
        unsafe extern "C" fn(
            c: *mut u8,
            m: *const u8,
            mlen: u64,
            n: *const u8,
            ic: u64,
            k: *const u8,
        ) -> c_int,
    >,
    pub stream_ietf_ext_xor_ic: Option<
        unsafe extern "C" fn(
            c: *mut u8,
            m: *const u8,
            mlen: u64,
            n: *const u8,
            ic: u32,
            k: *const u8,
        ) -> c_int,
    >,
}

unsafe impl Sync for CryptoStreamChacha20Implementation {}

extern "C" {
    // Defined in ref/chacha20_ref.c
    static crypto_stream_chacha20_ref_implementation: CryptoStreamChacha20Implementation;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

// static const crypto_stream_chacha20_implementation *implementation =
//     &crypto_stream_chacha20_ref_implementation;
// Only the ref implementation is available in this build.
static mut IMPLEMENTATION: *const CryptoStreamChacha20Implementation =
    core::ptr::addr_of!(crypto_stream_chacha20_ref_implementation);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_keybytes() -> usize {
    CRYPTO_STREAM_CHACHA20_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_noncebytes() -> usize {
    CRYPTO_STREAM_CHACHA20_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_messagebytes_max() -> usize {
    CRYPTO_STREAM_CHACHA20_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_keybytes() -> usize {
    CRYPTO_STREAM_CHACHA20_IETF_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_noncebytes() -> usize {
    CRYPTO_STREAM_CHACHA20_IETF_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_messagebytes_max() -> usize {
    CRYPTO_STREAM_CHACHA20_IETF_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > CRYPTO_STREAM_CHACHA20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ((*IMPLEMENTATION).stream.unwrap_unchecked())(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    if mlen > CRYPTO_STREAM_CHACHA20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ((*IMPLEMENTATION).stream_xor_ic.unwrap_unchecked())(c, m, mlen, n, ic, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > CRYPTO_STREAM_CHACHA20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ((*IMPLEMENTATION).stream_xor_ic.unwrap_unchecked())(c, m, mlen, n, 0u64, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_ext(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > CRYPTO_STREAM_CHACHA20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ((*IMPLEMENTATION).stream_ietf_ext.unwrap_unchecked())(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_ext_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u32,
    k: *const u8,
) -> c_int {
    if mlen > CRYPTO_STREAM_CHACHA20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ((*IMPLEMENTATION).stream_ietf_ext_xor_ic.unwrap_unchecked())(c, m, mlen, n, ic, k)
}

unsafe fn crypto_stream_chacha20_ietf_ext_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > CRYPTO_STREAM_CHACHA20_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ((*IMPLEMENTATION).stream_ietf_ext_xor_ic.unwrap_unchecked())(c, m, mlen, n, 0u32, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > CRYPTO_STREAM_CHACHA20_IETF_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf_ext(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u32,
    k: *const u8,
) -> c_int {
    if (ic as u64)
        > (64u64 * (1u64 << 32)) / 64u64 - (mlen + 63u64) / 64u64
    {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf_ext_xor_ic(c, m, mlen, n, ic, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > CRYPTO_STREAM_CHACHA20_IETF_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf_ext_xor(c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_STREAM_CHACHA20_IETF_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_STREAM_CHACHA20_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_stream_chacha20_pick_best_implementation() -> c_int {
    // All SIMD HAVE_* macros undefined and sodium_runtime_has_*() == 0:
    // only the ref implementation is selected.
    IMPLEMENTATION = core::ptr::addr_of!(crypto_stream_chacha20_ref_implementation);
    0
}
