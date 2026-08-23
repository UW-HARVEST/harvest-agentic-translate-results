pub mod r#ref;

// Translation of `crypto_stream/chacha20/stream_chacha20.c`.

use core::ffi::{c_int, c_void};

use crate::randombytes::randombytes_buf;
use crate::sodium::core::sodium_misuse;

pub const crypto_stream_chacha20_KEYBYTES: usize = 32;
pub const crypto_stream_chacha20_NONCEBYTES: usize = 8;
/// `SODIUM_SIZE_MAX` == `SODIUM_MIN(UINT64_MAX, SIZE_MAX)`
pub const crypto_stream_chacha20_MESSAGEBYTES_MAX: usize = crate::common::SIZE_MAX;

pub const crypto_stream_chacha20_ietf_KEYBYTES: usize = 32;
pub const crypto_stream_chacha20_ietf_NONCEBYTES: usize = 12;
/// `SODIUM_MIN(SODIUM_SIZE_MAX, 64ULL * (1ULL << 32))`
pub const crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX: usize = 64 * (1u64 << 32) as usize;

/// `typedef struct crypto_stream_chacha20_implementation`
#[repr(C)]
pub struct crypto_stream_chacha20_implementation {
    pub stream:
        Option<unsafe extern "C" fn(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int>,
    pub stream_ietf_ext:
        Option<unsafe extern "C" fn(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int>,
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

static mut implementation: *const crypto_stream_chacha20_implementation =
    &raw const r#ref::crypto_stream_chacha20_ref_implementation;

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
    crypto_stream_chacha20_MESSAGEBYTES_MAX
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
    crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > crypto_stream_chacha20_MESSAGEBYTES_MAX as u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    unsafe { ((*implementation).stream.unwrap())(c, clen, n, k) }
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
    if mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX as u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    unsafe { ((*implementation).stream_xor_ic.unwrap())(c, m, mlen, n, ic, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX as u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    unsafe { ((*implementation).stream_xor_ic.unwrap())(c, m, mlen, n, 0u64, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_ext(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > crypto_stream_chacha20_MESSAGEBYTES_MAX as u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    unsafe { ((*implementation).stream_ietf_ext.unwrap())(c, clen, n, k) }
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
    if mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX as u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    unsafe { ((*implementation).stream_ietf_ext_xor_ic.unwrap())(c, m, mlen, n, ic, k) }
}

unsafe fn crypto_stream_chacha20_ietf_ext_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX as u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    unsafe { ((*implementation).stream_ietf_ext_xor_ic.unwrap())(c, m, mlen, n, 0u32, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX as u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    unsafe { crypto_stream_chacha20_ietf_ext(c, clen, n, k) }
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
        > (64u64 * (1u64 << 32)) / 64u64 - (mlen.wrapping_add(63u64)) / 64u64
    {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    unsafe { crypto_stream_chacha20_ietf_ext_xor_ic(c, m, mlen, n, ic, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX as u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    unsafe { crypto_stream_chacha20_ietf_ext_xor(c, m, mlen, n, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_ietf_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_chacha20_ietf_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_chacha20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_chacha20_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_stream_chacha20_pick_best_implementation() -> c_int {
    unsafe {
        implementation = &raw const r#ref::crypto_stream_chacha20_ref_implementation;
    }
    0
}
