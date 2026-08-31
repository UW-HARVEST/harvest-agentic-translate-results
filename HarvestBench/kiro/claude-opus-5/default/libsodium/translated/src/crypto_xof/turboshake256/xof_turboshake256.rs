//! Translation of c_src/libsodium/crypto_xof/turboshake256/xof_turboshake256.c

use core::ffi::{c_int, c_void};

// Local repr(C) copy of the public API struct (rule 4), matching
// include/sodium/crypto_xof_turboshake256.h. CRYPTO_ALIGN(16), no #pragma pack.
#[repr(C, align(16))]
pub struct crypto_xof_turboshake256_state {
    pub opaque: [u8; 256],
}

// #define crypto_xof_turboshake256_BLOCKBYTES 136U
const crypto_xof_turboshake256_BLOCKBYTES: usize = 136;
// #define crypto_xof_turboshake256_STATEBYTES 256U
const crypto_xof_turboshake256_STATEBYTES: usize = 256;
// #define crypto_xof_turboshake256_DOMAIN_STANDARD 0x1FU
const crypto_xof_turboshake256_DOMAIN_STANDARD: u8 = 0x1F;

extern "C" {
    fn _sodium_turboshake256_ref(
        out: *mut u8,
        outlen: usize,
        in_: *const u8,
        inlen: usize,
    ) -> c_int;
    fn _sodium_turboshake256_ref_init(state: *mut c_void) -> c_int;
    fn _sodium_turboshake256_ref_init_with_domain(state: *mut c_void, domain: u8) -> c_int;
    fn _sodium_turboshake256_ref_update(
        state: *mut c_void,
        in_: *const u8,
        inlen: usize,
    ) -> c_int;
    fn _sodium_turboshake256_ref_squeeze(
        state: *mut c_void,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256_blockbytes() -> usize {
    crypto_xof_turboshake256_BLOCKBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256_statebytes() -> usize {
    crypto_xof_turboshake256_STATEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256_domain_standard() -> u8 {
    crypto_xof_turboshake256_DOMAIN_STANDARD
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    // COMPILER_ASSERT(sizeof(crypto_xof_turboshake256_state) >= sizeof(turboshake256_state_internal));
    _sodium_turboshake256_ref(out, outlen, in_, inlen as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256_init(
    state: *mut crypto_xof_turboshake256_state,
) -> c_int {
    let st = state as *mut c_void;
    _sodium_turboshake256_ref_init(st)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256_init_with_domain(
    state: *mut crypto_xof_turboshake256_state,
    domain: u8,
) -> c_int {
    let st = state as *mut c_void;
    _sodium_turboshake256_ref_init_with_domain(st, domain)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256_update(
    state: *mut crypto_xof_turboshake256_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    let st = state as *mut c_void;
    _sodium_turboshake256_ref_update(st, in_, inlen as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_turboshake256_squeeze(
    state: *mut crypto_xof_turboshake256_state,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let st = state as *mut c_void;
    _sodium_turboshake256_ref_squeeze(st, out, outlen)
}
