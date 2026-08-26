//! Translation of `crypto_xof/shake256/xof_shake256.c` and
//! `crypto_xof/shake256/ref/shake256_ref.c`.

use core::ffi::{c_int, c_void};

use crate::crypto_core::keccak1600::{
    crypto_core_keccak1600_extract_bytes, crypto_core_keccak1600_init,
    crypto_core_keccak1600_permute_24, crypto_core_keccak1600_state,
    crypto_core_keccak1600_xor_bytes,
};

pub const crypto_xof_shake256_BLOCKBYTES: usize = 136;
pub const crypto_xof_shake256_STATEBYTES: usize = 256;
pub const crypto_xof_shake256_DOMAIN_STANDARD: u8 = 0x1f;

const SHAKE256_RATE: usize = 136;

const SHAKE256_PHASE_ABSORBING: u8 = 0;
const SHAKE256_PHASE_SQUEEZING: u8 = 1;

/// `crypto_xof_shake256_state` from `include/sodium/crypto_xof_shake256.h`.
#[repr(C, align(16))]
#[derive(Copy, Clone)]
pub struct crypto_xof_shake256_state {
    pub opaque: [u8; 256],
}

/// `shake256_state_internal` from `crypto_xof/shake256/ref/shake256_ref.h`.
#[repr(C)]
pub struct shake256_state_internal {
    pub state: crypto_core_keccak1600_state,
    pub offset: usize,
    pub phase: u8,
    pub domain: u8,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_shake256_ref_init_with_domain(
    state: *mut shake256_state_internal,
    domain: u8,
) -> c_int {
    unsafe {
        crypto_core_keccak1600_init(core::ptr::addr_of_mut!((*state).state));
        (*state).offset = 0;
        (*state).phase = SHAKE256_PHASE_ABSORBING;
        (*state).domain = domain;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_shake256_ref_init(state: *mut shake256_state_internal) -> c_int {
    unsafe { _sodium_shake256_ref_init_with_domain(state, crypto_xof_shake256_DOMAIN_STANDARD) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_shake256_ref_update(
    state: *mut shake256_state_internal,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    let mut consumed: usize = 0;
    let mut chunk_size: usize;
    let mut ret: c_int = 0;

    unsafe {
        if (*state).phase != SHAKE256_PHASE_ABSORBING {
            crypto_core_keccak1600_permute_24(core::ptr::addr_of_mut!((*state).state));
            (*state).phase = SHAKE256_PHASE_ABSORBING;
            (*state).offset = 0;
            ret = -1;
        }

        if (*state).offset == SHAKE256_RATE && inlen > 0 {
            crypto_core_keccak1600_permute_24(core::ptr::addr_of_mut!((*state).state));
            (*state).offset = 0;
        }
        if (*state).offset != 0 && inlen > 0 {
            chunk_size = SHAKE256_RATE - (*state).offset;
            if chunk_size > inlen {
                chunk_size = inlen;
            }
            crypto_core_keccak1600_xor_bytes(
                core::ptr::addr_of_mut!((*state).state),
                in_,
                (*state).offset,
                chunk_size,
            );
            (*state).offset += chunk_size;
            consumed = chunk_size;
            if (*state).offset == SHAKE256_RATE && consumed < inlen {
                crypto_core_keccak1600_permute_24(core::ptr::addr_of_mut!((*state).state));
                (*state).offset = 0;
            }
        }
        while inlen.wrapping_sub(consumed) >= SHAKE256_RATE {
            crypto_core_keccak1600_xor_bytes(
                core::ptr::addr_of_mut!((*state).state),
                in_.wrapping_add(consumed),
                0,
                SHAKE256_RATE,
            );
            consumed += SHAKE256_RATE;
            (*state).offset = SHAKE256_RATE;
            if consumed < inlen {
                crypto_core_keccak1600_permute_24(core::ptr::addr_of_mut!((*state).state));
                (*state).offset = 0;
            }
        }
        if consumed < inlen {
            chunk_size = inlen - consumed;
            crypto_core_keccak1600_xor_bytes(
                core::ptr::addr_of_mut!((*state).state),
                in_.wrapping_add(consumed),
                0,
                chunk_size,
            );
            (*state).offset = chunk_size;
        }
    }

    ret
}

unsafe fn shake256_finalize(state: *mut shake256_state_internal) {
    let pad: u8;

    unsafe {
        /* If the rate is exactly full, process that block before padding */
        if (*state).offset == SHAKE256_RATE {
            crypto_core_keccak1600_permute_24(core::ptr::addr_of_mut!((*state).state));
            (*state).offset = 0;
        }

        /* Apply padding: domain byte at current position, 0x80 at last byte */
        if (*state).offset == SHAKE256_RATE - 1 {
            /* Special case: padding fits in one byte */
            pad = (*state).domain ^ 0x80;
            crypto_core_keccak1600_xor_bytes(
                core::ptr::addr_of_mut!((*state).state),
                &pad,
                (*state).offset,
                1,
            );
        } else {
            /* Normal case: domain and 0x80 at different positions */
            crypto_core_keccak1600_xor_bytes(
                core::ptr::addr_of_mut!((*state).state),
                core::ptr::addr_of!((*state).domain),
                (*state).offset,
                1,
            );
            pad = 0x80;
            crypto_core_keccak1600_xor_bytes(
                core::ptr::addr_of_mut!((*state).state),
                &pad,
                SHAKE256_RATE - 1,
                1,
            );
        }

        /* Final permutation */
        crypto_core_keccak1600_permute_24(core::ptr::addr_of_mut!((*state).state));

        (*state).offset = 0;
        (*state).phase = SHAKE256_PHASE_SQUEEZING;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_shake256_ref_squeeze(
    state: *mut shake256_state_internal,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let mut extracted: usize = 0;
    let mut chunk_size: usize;

    unsafe {
        if (*state).phase == SHAKE256_PHASE_ABSORBING {
            shake256_finalize(state);
        }

        if (*state).offset == SHAKE256_RATE && outlen > 0 {
            crypto_core_keccak1600_permute_24(core::ptr::addr_of_mut!((*state).state));
            (*state).offset = 0;
        }
        if (*state).offset != 0 && outlen > 0 {
            chunk_size = SHAKE256_RATE - (*state).offset;
            if chunk_size > outlen {
                chunk_size = outlen;
            }
            crypto_core_keccak1600_extract_bytes(
                core::ptr::addr_of!((*state).state),
                out,
                (*state).offset,
                chunk_size,
            );
            (*state).offset += chunk_size;
            extracted = chunk_size;
            if (*state).offset == SHAKE256_RATE && extracted < outlen {
                crypto_core_keccak1600_permute_24(core::ptr::addr_of_mut!((*state).state));
                (*state).offset = 0;
            }
        }
        while outlen.wrapping_sub(extracted) >= SHAKE256_RATE {
            crypto_core_keccak1600_extract_bytes(
                core::ptr::addr_of!((*state).state),
                out.wrapping_add(extracted),
                0,
                SHAKE256_RATE,
            );
            extracted += SHAKE256_RATE;
            (*state).offset = SHAKE256_RATE;
            if extracted < outlen {
                crypto_core_keccak1600_permute_24(core::ptr::addr_of_mut!((*state).state));
                (*state).offset = 0;
            }
        }
        if extracted < outlen {
            chunk_size = outlen - extracted;
            crypto_core_keccak1600_extract_bytes(
                core::ptr::addr_of!((*state).state),
                out.wrapping_add(extracted),
                0,
                chunk_size,
            );
            (*state).offset = chunk_size;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_shake256_ref(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    let mut state = shake256_state_internal {
        state: crypto_core_keccak1600_state { opaque: [0u8; 224] },
        offset: 0,
        phase: 0,
        domain: 0,
    };

    unsafe {
        _sodium_shake256_ref_init(&mut state);
        _sodium_shake256_ref_update(&mut state, in_, inlen);
        _sodium_shake256_ref_squeeze(&mut state, out, outlen);
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake256_blockbytes() -> usize {
    crypto_xof_shake256_BLOCKBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake256_statebytes() -> usize {
    crypto_xof_shake256_STATEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake256_domain_standard() -> u8 {
    crypto_xof_shake256_DOMAIN_STANDARD
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake256(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    unsafe { _sodium_shake256_ref(out, outlen, in_, inlen as usize) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake256_init(
    state: *mut crypto_xof_shake256_state,
) -> c_int {
    let st: *mut shake256_state_internal = state as *mut c_void as *mut shake256_state_internal;

    unsafe { _sodium_shake256_ref_init(st) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake256_init_with_domain(
    state: *mut crypto_xof_shake256_state,
    domain: u8,
) -> c_int {
    let st: *mut shake256_state_internal = state as *mut c_void as *mut shake256_state_internal;

    unsafe { _sodium_shake256_ref_init_with_domain(st, domain) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake256_update(
    state: *mut crypto_xof_shake256_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    let st: *mut shake256_state_internal = state as *mut c_void as *mut shake256_state_internal;

    unsafe { _sodium_shake256_ref_update(st, in_, inlen as usize) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake256_squeeze(
    state: *mut crypto_xof_shake256_state,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let st: *mut shake256_state_internal = state as *mut c_void as *mut shake256_state_internal;

    unsafe { _sodium_shake256_ref_squeeze(st, out, outlen) }
}
