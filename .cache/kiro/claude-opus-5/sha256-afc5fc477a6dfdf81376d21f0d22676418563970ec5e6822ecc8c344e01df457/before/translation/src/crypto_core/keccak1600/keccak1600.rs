//! Translation of c_src/libsodium/crypto_core/keccak1600/keccak1600.c

use core::ffi::c_void;

// Local repr(C) copy of the public API struct (rule 4), matching
// include/sodium/crypto_core_keccak1600.h.
// The header wraps the struct in `#pragma pack(push, 1)` and marks it
// CRYPTO_ALIGN(16). For an all-`u8` struct `pack(1)` has no effect on the
// layout (size 224, align 16), and Rust forbids combining `packed` with
// `align`, so `#[repr(C, align(16))]` reproduces the layout exactly.
#[repr(C, align(16))]
pub struct crypto_core_keccak1600_state {
    pub opaque: [u8; 224],
}

// __ARM_FEATURE_SHA3 undefined: dispatch to the `ref` implementation.
// The renamed (quirks.h) linker symbols of the ref functions.
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
    _sodium_keccak1600_ref_init(core::ptr::addr_of_mut!((*state).opaque) as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_xor_bytes(
    state: *mut crypto_core_keccak1600_state,
    bytes: *const u8,
    offset: usize,
    length: usize,
) {
    _sodium_keccak1600_ref_xor_bytes(
        core::ptr::addr_of_mut!((*state).opaque) as *mut c_void,
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
        core::ptr::addr_of!((*state).opaque) as *const c_void,
        bytes,
        offset,
        length,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_permute_24(
    state: *mut crypto_core_keccak1600_state,
) {
    _sodium_keccak1600_ref_permute_24(core::ptr::addr_of_mut!((*state).opaque) as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_permute_12(
    state: *mut crypto_core_keccak1600_state,
) {
    _sodium_keccak1600_ref_permute_12(core::ptr::addr_of_mut!((*state).opaque) as *mut c_void);
}
