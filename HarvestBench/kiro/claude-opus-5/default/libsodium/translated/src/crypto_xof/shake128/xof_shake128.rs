//! Translation of c_src/libsodium/crypto_xof/shake128/xof_shake128.c

use core::ffi::{c_int, c_void};

// Local repr(C) copy of the public API struct (rule 4), matching
// include/sodium/crypto_xof_shake128.h. CRYPTO_ALIGN(16), no #pragma pack.
#[repr(C, align(16))]
pub struct crypto_xof_shake128_state {
    pub opaque: [u8; 256],
}

// #define crypto_xof_shake128_BLOCKBYTES 168U
const crypto_xof_shake128_BLOCKBYTES: usize = 168;
// #define crypto_xof_shake128_STATEBYTES 256U
const crypto_xof_shake128_STATEBYTES: usize = 256;
// #define crypto_xof_shake128_DOMAIN_STANDARD 0x1FU
const crypto_xof_shake128_DOMAIN_STANDARD: u8 = 0x1F;

// shake128_ref* live in another C file (rule 3), renamed by quirks.h.
extern "C" {
    fn _sodium_shake128_ref(
        out: *mut u8,
        outlen: usize,
        in_: *const u8,
        inlen: usize,
    ) -> c_int;
    fn _sodium_shake128_ref_init(state: *mut c_void) -> c_int;
    fn _sodium_shake128_ref_init_with_domain(state: *mut c_void, domain: u8) -> c_int;
    fn _sodium_shake128_ref_update(state: *mut c_void, in_: *const u8, inlen: usize) -> c_int;
    fn _sodium_shake128_ref_squeeze(state: *mut c_void, out: *mut u8, outlen: usize) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake128_blockbytes() -> usize {
    crypto_xof_shake128_BLOCKBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake128_statebytes() -> usize {
    crypto_xof_shake128_STATEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake128_domain_standard() -> u8 {
    crypto_xof_shake128_DOMAIN_STANDARD
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake128(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    // COMPILER_ASSERT(sizeof(crypto_xof_shake128_state) >= sizeof(shake128_state_internal));
    _sodium_shake128_ref(out, outlen, in_, inlen as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake128_init(
    state: *mut crypto_xof_shake128_state,
) -> c_int {
    let st = state as *mut c_void;
    // COMPILER_ASSERT(sizeof(crypto_xof_shake128_state) >= sizeof(shake128_state_internal));
    _sodium_shake128_ref_init(st)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake128_init_with_domain(
    state: *mut crypto_xof_shake128_state,
    domain: u8,
) -> c_int {
    let st = state as *mut c_void;
    // COMPILER_ASSERT(sizeof(crypto_xof_shake128_state) >= sizeof(shake128_state_internal));
    _sodium_shake128_ref_init_with_domain(st, domain)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake128_update(
    state: *mut crypto_xof_shake128_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    let st = state as *mut c_void;
    _sodium_shake128_ref_update(st, in_, inlen as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake128_squeeze(
    state: *mut crypto_xof_shake128_state,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let st = state as *mut c_void;
    _sodium_shake128_ref_squeeze(st, out, outlen)
}
