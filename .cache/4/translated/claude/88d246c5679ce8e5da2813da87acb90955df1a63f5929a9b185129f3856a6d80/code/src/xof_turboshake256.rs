//! Translation of `c_src/libsodium/crypto_xof/turboshake256/xof_turboshake256.c`.
//!
//! None of the symbols in this file are renamed by `private/quirks.h`: they are
//! the public `crypto_xof_turboshake256*` API.
//!
//! The `turboshake256_ref*` back end lives in
//! `c_src/libsodium/crypto_xof/turboshake256/ref/turboshake256_ref.c`; quirks.h renames
//! those to `_sodium_turboshake256_ref*`, so they are declared here in a local
//! `extern "C"` block (per the cross-file convention).
//!
//! The `COMPILER_ASSERT(sizeof(crypto_xof_turboshake256_state) >=
//! sizeof(turboshake256_state_internal))` checks are compile-time only; they are
//! reproduced as `const` assertions.

use core::ffi::{c_int, c_ulonglong, c_void};

/* #define crypto_xof_turboshake256_BLOCKBYTES 136U */
const crypto_xof_turboshake256_BLOCKBYTES: usize = 136;
/* #define crypto_xof_turboshake256_STATEBYTES 256U */
const crypto_xof_turboshake256_STATEBYTES: usize = 256;
/* #define crypto_xof_turboshake256_DOMAIN_STANDARD 0x1FU */
const crypto_xof_turboshake256_DOMAIN_STANDARD: u8 = 0x1f;

/// ```c
/// typedef struct CRYPTO_ALIGN(16) crypto_xof_turboshake256_state {
///     unsigned char opaque[256];
/// } crypto_xof_turboshake256_state;
/// ```
#[repr(C, align(16))]
pub struct crypto_xof_turboshake256_state {
    pub opaque: [u8; 256],
}

/// `crypto_core_keccak1600_state`: 224 bytes, 16-byte aligned.
#[repr(C, align(16))]
struct crypto_core_keccak1600_state {
    opaque: [u8; 224],
}

/// `turboshake256_state_internal` from `ref/turboshake256_ref.h` (size 240, align 16).
#[repr(C)]
struct turboshake256_state_internal {
    state: crypto_core_keccak1600_state,
    offset: usize,
    phase: u8,
    domain: u8,
}

/* COMPILER_ASSERT(sizeof(crypto_xof_turboshake256_state) >= sizeof(turboshake256_state_internal)); */
const _: () = assert!(
    core::mem::size_of::<crypto_xof_turboshake256_state>()
        >= core::mem::size_of::<turboshake256_state_internal>()
);

extern "C" {
    /* turboshake256_ref -> _sodium_turboshake256_ref */
    fn _sodium_turboshake256_ref(out: *mut u8, outlen: usize, in_: *const u8, inlen: usize) -> c_int;
    /* turboshake256_ref_init -> _sodium_turboshake256_ref_init */
    fn _sodium_turboshake256_ref_init(state: *mut c_void) -> c_int;
    /* turboshake256_ref_init_with_domain -> _sodium_turboshake256_ref_init_with_domain */
    fn _sodium_turboshake256_ref_init_with_domain(state: *mut c_void, domain: u8) -> c_int;
    /* turboshake256_ref_update -> _sodium_turboshake256_ref_update */
    fn _sodium_turboshake256_ref_update(state: *mut c_void, in_: *const u8, inlen: usize) -> c_int;
    /* turboshake256_ref_squeeze -> _sodium_turboshake256_ref_squeeze */
    fn _sodium_turboshake256_ref_squeeze(state: *mut c_void, out: *mut u8, outlen: usize) -> c_int;
}

/// `size_t crypto_xof_turboshake256_blockbytes(void)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256_blockbytes() -> usize {
    crypto_xof_turboshake256_BLOCKBYTES
}

/// `size_t crypto_xof_turboshake256_statebytes(void)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256_statebytes() -> usize {
    crypto_xof_turboshake256_STATEBYTES
}

/// `unsigned char crypto_xof_turboshake256_domain_standard(void)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256_domain_standard() -> u8 {
    crypto_xof_turboshake256_DOMAIN_STANDARD
}

/// ```c
/// int crypto_xof_turboshake256(unsigned char *out, size_t outlen, const unsigned char *in,
///                         unsigned long long inlen)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: c_ulonglong,
) -> c_int {
    _sodium_turboshake256_ref(out, outlen, in_, inlen as usize)
}

/// `int crypto_xof_turboshake256_init(crypto_xof_turboshake256_state *state)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256_init(state: *mut crypto_xof_turboshake256_state) -> c_int {
    let st: *mut c_void = state as *mut c_void;

    _sodium_turboshake256_ref_init(st)
}

/// `int crypto_xof_turboshake256_init_with_domain(crypto_xof_turboshake256_state *state, unsigned char domain)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256_init_with_domain(
    state: *mut crypto_xof_turboshake256_state,
    domain: u8,
) -> c_int {
    let st: *mut c_void = state as *mut c_void;

    _sodium_turboshake256_ref_init_with_domain(st, domain)
}

/// ```c
/// int crypto_xof_turboshake256_update(crypto_xof_turboshake256_state *state,
///                                const unsigned char *in, unsigned long long inlen)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256_update(
    state: *mut crypto_xof_turboshake256_state,
    in_: *const u8,
    inlen: c_ulonglong,
) -> c_int {
    let st: *mut c_void = state as *mut c_void;

    _sodium_turboshake256_ref_update(st, in_, inlen as usize)
}

/// ```c
/// int crypto_xof_turboshake256_squeeze(crypto_xof_turboshake256_state *state, unsigned char *out,
///                                 size_t outlen)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256_squeeze(
    state: *mut crypto_xof_turboshake256_state,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let st: *mut c_void = state as *mut c_void;

    _sodium_turboshake256_ref_squeeze(st, out, outlen)
}
