pub mod r#ref;

// Translation of `crypto_core/keccak1600/keccak1600.c`.
//
// `__ARM_FEATURE_SHA3` is undefined in the reference build, so the `#else`
// branch is taken and every entry point forwards to the `ref` backend:
//
//     # define keccak1600_init          keccak1600_ref_init
//     # define keccak1600_xor_bytes     keccak1600_ref_xor_bytes
//     # define keccak1600_extract_bytes keccak1600_ref_extract_bytes
//     # define keccak1600_permute_24    keccak1600_ref_permute_24
//     # define keccak1600_permute_12    keccak1600_ref_permute_12

use core::ffi::c_void;

use self::r#ref::{
    _sodium_keccak1600_ref_extract_bytes as keccak1600_extract_bytes,
    _sodium_keccak1600_ref_init as keccak1600_init,
    _sodium_keccak1600_ref_permute_12 as keccak1600_permute_12,
    _sodium_keccak1600_ref_permute_24 as keccak1600_permute_24,
    _sodium_keccak1600_ref_xor_bytes as keccak1600_xor_bytes,
};

/// `crypto_core_keccak1600_state` from `include/sodium/crypto_core_keccak1600.h`.
///
/// The header wraps the definition in `#pragma pack(push, 1)` but the type
/// itself carries `CRYPTO_ALIGN(16)`, which wins: `sizeof` is 224 and
/// `_Alignof` is 16.
#[repr(C, align(16))]
#[derive(Copy, Clone)]
pub struct crypto_core_keccak1600_state {
    pub opaque: [u8; 224],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_statebytes() -> usize {
    core::mem::size_of::<crypto_core_keccak1600_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_init(state: *mut crypto_core_keccak1600_state) {
    unsafe { keccak1600_init(core::ptr::addr_of_mut!((*state).opaque) as *mut c_void) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_xor_bytes(
    state: *mut crypto_core_keccak1600_state,
    bytes: *const u8,
    offset: usize,
    length: usize,
) {
    unsafe {
        keccak1600_xor_bytes(
            core::ptr::addr_of_mut!((*state).opaque) as *mut c_void,
            bytes,
            offset,
            length,
        )
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_extract_bytes(
    state: *const crypto_core_keccak1600_state,
    bytes: *mut u8,
    offset: usize,
    length: usize,
) {
    unsafe {
        keccak1600_extract_bytes(
            core::ptr::addr_of!((*state).opaque) as *const c_void,
            bytes,
            offset,
            length,
        )
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_permute_24(
    state: *mut crypto_core_keccak1600_state,
) {
    unsafe { keccak1600_permute_24(core::ptr::addr_of_mut!((*state).opaque) as *mut c_void) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_keccak1600_permute_12(
    state: *mut crypto_core_keccak1600_state,
) {
    unsafe { keccak1600_permute_12(core::ptr::addr_of_mut!((*state).opaque) as *mut c_void) };
}
