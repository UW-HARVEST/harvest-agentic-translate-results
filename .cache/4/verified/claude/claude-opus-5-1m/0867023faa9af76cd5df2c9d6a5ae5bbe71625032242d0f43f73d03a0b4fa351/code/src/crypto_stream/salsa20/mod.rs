pub mod r#ref;

// Translation of `crypto_stream/salsa20/stream_salsa20.c` (and the
// implementation dispatch table declared in `stream_salsa20.h`).
//
// The reference build defines no `HAVE_*` macro, so `HAVE_AMD64_ASM` is
// undefined and the only available implementation is the portable `ref` one.

use core::ffi::{c_int, c_void};

use crate::common::SODIUM_SIZE_MAX;
use crate::randombytes::randombytes_buf;

// ---------------------------------------------------------------------------
// stream_salsa20.h
// ---------------------------------------------------------------------------

pub type stream_fn =
    unsafe extern "C" fn(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;

pub type stream_xor_ic_fn = unsafe extern "C" fn(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int;

/// `typedef struct crypto_stream_salsa20_implementation` from
/// `crypto_stream/salsa20/stream_salsa20.h`.
#[repr(C)]
pub struct crypto_stream_salsa20_implementation {
    pub stream: Option<stream_fn>,
    pub stream_xor_ic: Option<stream_xor_ic_fn>,
}

// ---------------------------------------------------------------------------
// crypto_stream_salsa20.h constants
// ---------------------------------------------------------------------------

pub const crypto_stream_salsa20_KEYBYTES: usize = 32;
pub const crypto_stream_salsa20_NONCEBYTES: usize = 8;

// ---------------------------------------------------------------------------
// stream_salsa20.c
// ---------------------------------------------------------------------------

/* HAVE_AMD64_ASM undefined -> the reference implementation */
static mut implementation: *const crypto_stream_salsa20_implementation =
    &raw const r#ref::crypto_stream_salsa20_ref_implementation;

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
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    unsafe {
        let imp = implementation;
        ((*imp).stream.unwrap())(c, clen, n, k)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    unsafe {
        let imp = implementation;
        ((*imp).stream_xor_ic.unwrap())(c, m, mlen, n, ic, k)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    unsafe {
        let imp = implementation;
        ((*imp).stream_xor_ic.unwrap())(c, m, mlen, n, 0u64, k)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_salsa20_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_stream_salsa20_pick_best_implementation() -> c_int {
    unsafe {
        implementation = &raw const r#ref::crypto_stream_salsa20_ref_implementation;
    }
    /* No SIMD implementation is compiled in, and every
     * sodium_runtime_has_*() returns 0 anyway. */
    0 /* LCOV_EXCL_LINE */
}
