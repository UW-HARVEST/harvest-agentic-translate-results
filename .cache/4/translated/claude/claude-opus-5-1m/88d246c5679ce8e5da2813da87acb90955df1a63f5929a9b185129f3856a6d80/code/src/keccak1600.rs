//! Translation of `crypto_core/keccak1600/keccak1600.c`.
//!
//! `__ARM_FEATURE_SHA3` is not defined for the reference build, so the
//! `keccak1600_*` dispatch macros expand to the `keccak1600_ref_*` variants,
//! which `private/quirks.h` in turn renames to `_sodium_keccak1600_ref_*`.

use core::ffi::c_void;

/// `crypto_core_keccak1600_state` from `include/sodium/crypto_core_keccak1600.h`
/// (`unsigned char opaque[224]`, `CRYPTO_ALIGN(16)`): size 224, alignment 16.
#[repr(C, align(16))]
pub struct crypto_core_keccak1600_state {
    pub opaque: [u8; 224],
}

extern "C" {
    fn _sodium_keccak1600_ref_init(state: *mut c_void);
    fn _sodium_keccak1600_ref_xor_bytes(
        state: *mut c_void,
        bytes: *const u8,
        offset: usize,
        length: usize,
    );
    fn _sodium_keccak1600_ref_extract_bytes(
        state: *const c_void,
        bytes: *mut u8,
        offset: usize,
        length: usize,
    );
    fn _sodium_keccak1600_ref_permute_24(state: *mut c_void);
    fn _sodium_keccak1600_ref_permute_12(state: *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_statebytes() -> usize {
    core::mem::size_of::<crypto_core_keccak1600_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_init(state: *mut crypto_core_keccak1600_state) {
    _sodium_keccak1600_ref_init((*state).opaque.as_mut_ptr() as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_xor_bytes(
    state: *mut crypto_core_keccak1600_state,
    bytes: *const u8,
    offset: usize,
    length: usize,
) {
    _sodium_keccak1600_ref_xor_bytes(
        (*state).opaque.as_mut_ptr() as *mut c_void,
        bytes,
        offset,
        length,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_extract_bytes(
    state: *const crypto_core_keccak1600_state,
    bytes: *mut u8,
    offset: usize,
    length: usize,
) {
    _sodium_keccak1600_ref_extract_bytes(
        (*state).opaque.as_ptr() as *const c_void,
        bytes,
        offset,
        length,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_permute_24(
    state: *mut crypto_core_keccak1600_state,
) {
    _sodium_keccak1600_ref_permute_24((*state).opaque.as_mut_ptr() as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_permute_12(
    state: *mut crypto_core_keccak1600_state,
) {
    _sodium_keccak1600_ref_permute_12((*state).opaque.as_mut_ptr() as *mut c_void);
}
