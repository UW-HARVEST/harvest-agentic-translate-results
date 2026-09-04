//! Translation of the four libsodium XOF (extendable output function)
//! wrappers built on `crypto_core_keccak1600`:
//!
//!   * `crypto_xof/shake128/xof_shake128.c` + `.../ref/shake128_ref.c`
//!   * `crypto_xof/shake256/xof_shake256.c` + `.../ref/shake256_ref.c`
//!   * `crypto_xof/turboshake128/xof_turboshake128.c` + `.../ref/turboshake128_ref.c`
//!   * `crypto_xof/turboshake256/xof_turboshake256.c` + `.../ref/turboshake256_ref.c`
//!
//! All four share the identical absorb/permute/squeeze logic from libsodium;
//! they differ only in the block rate (168 vs 136 bytes) and which
//! `crypto_core_keccak1600_permute_*` variant is used (24 vs 12 rounds).
//! Headers: `crypto_xof_shake128.h`, `crypto_xof_shake256.h`,
//! `crypto_xof_turboshake128.h`, `crypto_xof_turboshake256.h`, and the
//! matching `*_ref.h` files.
#![allow(private_interfaces)]

use core::ffi::c_int;

/// `crypto_core_keccak1600_state` from `crypto_core_keccak1600.h`, duplicated
/// here (per translation conventions) since cross-module calls only share
/// the final linker names, not Rust types.
#[repr(C, align(16))]
struct crypto_core_keccak1600_state {
    opaque: [u8; 224],
}

extern "C" {
    fn crypto_core_keccak1600_init(state: *mut crypto_core_keccak1600_state);
    fn crypto_core_keccak1600_xor_bytes(
        state: *mut crypto_core_keccak1600_state,
        bytes: *const u8,
        offset: usize,
        length: usize,
    );
    fn crypto_core_keccak1600_extract_bytes(
        state: *const crypto_core_keccak1600_state,
        bytes: *mut u8,
        offset: usize,
        length: usize,
    );
    fn crypto_core_keccak1600_permute_24(state: *mut crypto_core_keccak1600_state);
    fn crypto_core_keccak1600_permute_12(state: *mut crypto_core_keccak1600_state);
}

const PHASE_ABSORBING: u8 = 0;
const PHASE_SQUEEZING: u8 = 1;

// ===================== shake128 (rate = 168, crypto_core_keccak1600_permute_24) =====================

#[repr(C, align(16))]
pub struct crypto_xof_shake128_state {
    pub opaque: [u8; 256],
}

#[repr(C)]
struct shake128_state_internal {
    state: crypto_core_keccak1600_state,
    offset: usize,
    phase: u8,
    domain: u8,
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_shake128_ref_init_with_domain(
    state: *mut shake128_state_internal,
    domain: u8,
) -> c_int {
    crypto_core_keccak1600_init(&mut (*state).state);
    (*state).offset = 0;
    (*state).phase = PHASE_ABSORBING;
    (*state).domain = domain;

    0
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_shake128_ref_init(state: *mut shake128_state_internal) -> c_int {
    _sodium_shake128_ref_init_with_domain(state, 0x1Fu8)
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_shake128_ref_update(
    state: *mut shake128_state_internal,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    let mut consumed: usize = 0;
    let mut chunk_size: usize;
    let mut ret: c_int = 0;

    if (*state).phase != PHASE_ABSORBING {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        (*state).phase = PHASE_ABSORBING;
        (*state).offset = 0;
        ret = -1;
    }

    if (*state).offset == 168 && inlen > 0 {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && inlen > 0 {
        chunk_size = 168 - (*state).offset;
        if chunk_size > inlen {
            chunk_size = inlen;
        }
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_, (*state).offset, chunk_size);
        (*state).offset += chunk_size;
        consumed = chunk_size;
        if (*state).offset == 168 && consumed < inlen {
            crypto_core_keccak1600_permute_24(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    while inlen - consumed >= 168 {
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_.add(consumed), 0, 168);
        consumed += 168;
        (*state).offset = 168;
        if consumed < inlen {
            crypto_core_keccak1600_permute_24(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    if consumed < inlen {
        chunk_size = inlen - consumed;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_.add(consumed), 0, chunk_size);
        (*state).offset = chunk_size;
    }

    ret
}

unsafe fn shake128_finalize(state: *mut shake128_state_internal) {
    let mut pad: u8;

    if (*state).offset == 168 {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        (*state).offset = 0;
    }

    if (*state).offset == 168 - 1 {
        pad = (*state).domain ^ 0x80;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, &pad, (*state).offset, 1);
    } else {
        let domain = (*state).domain;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, &domain, (*state).offset, 1);
        pad = 0x80;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, &pad, 168 - 1, 1);
    }

    crypto_core_keccak1600_permute_24(&mut (*state).state);

    (*state).offset = 0;
    (*state).phase = PHASE_SQUEEZING;
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_shake128_ref_squeeze(
    state: *mut shake128_state_internal,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let mut extracted: usize = 0;
    let mut chunk_size: usize;

    if (*state).phase == PHASE_ABSORBING {
        shake128_finalize(state);
    }

    if (*state).offset == 168 && outlen > 0 {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && outlen > 0 {
        chunk_size = 168 - (*state).offset;
        if chunk_size > outlen {
            chunk_size = outlen;
        }
        crypto_core_keccak1600_extract_bytes(&(*state).state, out, (*state).offset, chunk_size);
        (*state).offset += chunk_size;
        extracted = chunk_size;
        if (*state).offset == 168 && extracted < outlen {
            crypto_core_keccak1600_permute_24(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    while outlen - extracted >= 168 {
        crypto_core_keccak1600_extract_bytes(&(*state).state, out.add(extracted), 0, 168);
        extracted += 168;
        (*state).offset = 168;
        if extracted < outlen {
            crypto_core_keccak1600_permute_24(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    if extracted < outlen {
        chunk_size = outlen - extracted;
        crypto_core_keccak1600_extract_bytes(&(*state).state, out.add(extracted), 0, chunk_size);
        (*state).offset = chunk_size;
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_shake128_ref(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    let mut state: shake128_state_internal = core::mem::zeroed();

    _sodium_shake128_ref_init(&mut state);
    _sodium_shake128_ref_update(&mut state, in_, inlen);
    _sodium_shake128_ref_squeeze(&mut state, out, outlen);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake128_blockbytes() -> usize {
    168
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake128_statebytes() -> usize {
    256
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake128_domain_standard() -> u8 {
    0x1Fu8
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake128(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    _sodium_shake128_ref(out, outlen, in_, inlen as usize)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake128_init(state: *mut crypto_xof_shake128_state) -> c_int {
    let st = state as *mut shake128_state_internal;
    _sodium_shake128_ref_init(st)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake128_init_with_domain(
    state: *mut crypto_xof_shake128_state,
    domain: u8,
) -> c_int {
    let st = state as *mut shake128_state_internal;
    _sodium_shake128_ref_init_with_domain(st, domain)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake128_update(
    state: *mut crypto_xof_shake128_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    let st = state as *mut shake128_state_internal;
    _sodium_shake128_ref_update(st, in_, inlen as usize)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake128_squeeze(
    state: *mut crypto_xof_shake128_state,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let st = state as *mut shake128_state_internal;
    _sodium_shake128_ref_squeeze(st, out, outlen)
}

// ===================== shake256 (rate = 136, crypto_core_keccak1600_permute_24) =====================

#[repr(C, align(16))]
pub struct crypto_xof_shake256_state {
    pub opaque: [u8; 256],
}

#[repr(C)]
struct shake256_state_internal {
    state: crypto_core_keccak1600_state,
    offset: usize,
    phase: u8,
    domain: u8,
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_shake256_ref_init_with_domain(
    state: *mut shake256_state_internal,
    domain: u8,
) -> c_int {
    crypto_core_keccak1600_init(&mut (*state).state);
    (*state).offset = 0;
    (*state).phase = PHASE_ABSORBING;
    (*state).domain = domain;

    0
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_shake256_ref_init(state: *mut shake256_state_internal) -> c_int {
    _sodium_shake256_ref_init_with_domain(state, 0x1Fu8)
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_shake256_ref_update(
    state: *mut shake256_state_internal,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    let mut consumed: usize = 0;
    let mut chunk_size: usize;
    let mut ret: c_int = 0;

    if (*state).phase != PHASE_ABSORBING {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        (*state).phase = PHASE_ABSORBING;
        (*state).offset = 0;
        ret = -1;
    }

    if (*state).offset == 136 && inlen > 0 {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && inlen > 0 {
        chunk_size = 136 - (*state).offset;
        if chunk_size > inlen {
            chunk_size = inlen;
        }
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_, (*state).offset, chunk_size);
        (*state).offset += chunk_size;
        consumed = chunk_size;
        if (*state).offset == 136 && consumed < inlen {
            crypto_core_keccak1600_permute_24(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    while inlen - consumed >= 136 {
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_.add(consumed), 0, 136);
        consumed += 136;
        (*state).offset = 136;
        if consumed < inlen {
            crypto_core_keccak1600_permute_24(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    if consumed < inlen {
        chunk_size = inlen - consumed;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_.add(consumed), 0, chunk_size);
        (*state).offset = chunk_size;
    }

    ret
}

unsafe fn shake256_finalize(state: *mut shake256_state_internal) {
    let mut pad: u8;

    if (*state).offset == 136 {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        (*state).offset = 0;
    }

    if (*state).offset == 136 - 1 {
        pad = (*state).domain ^ 0x80;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, &pad, (*state).offset, 1);
    } else {
        let domain = (*state).domain;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, &domain, (*state).offset, 1);
        pad = 0x80;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, &pad, 136 - 1, 1);
    }

    crypto_core_keccak1600_permute_24(&mut (*state).state);

    (*state).offset = 0;
    (*state).phase = PHASE_SQUEEZING;
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_shake256_ref_squeeze(
    state: *mut shake256_state_internal,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let mut extracted: usize = 0;
    let mut chunk_size: usize;

    if (*state).phase == PHASE_ABSORBING {
        shake256_finalize(state);
    }

    if (*state).offset == 136 && outlen > 0 {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && outlen > 0 {
        chunk_size = 136 - (*state).offset;
        if chunk_size > outlen {
            chunk_size = outlen;
        }
        crypto_core_keccak1600_extract_bytes(&(*state).state, out, (*state).offset, chunk_size);
        (*state).offset += chunk_size;
        extracted = chunk_size;
        if (*state).offset == 136 && extracted < outlen {
            crypto_core_keccak1600_permute_24(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    while outlen - extracted >= 136 {
        crypto_core_keccak1600_extract_bytes(&(*state).state, out.add(extracted), 0, 136);
        extracted += 136;
        (*state).offset = 136;
        if extracted < outlen {
            crypto_core_keccak1600_permute_24(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    if extracted < outlen {
        chunk_size = outlen - extracted;
        crypto_core_keccak1600_extract_bytes(&(*state).state, out.add(extracted), 0, chunk_size);
        (*state).offset = chunk_size;
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_shake256_ref(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    let mut state: shake256_state_internal = core::mem::zeroed();

    _sodium_shake256_ref_init(&mut state);
    _sodium_shake256_ref_update(&mut state, in_, inlen);
    _sodium_shake256_ref_squeeze(&mut state, out, outlen);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake256_blockbytes() -> usize {
    136
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake256_statebytes() -> usize {
    256
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake256_domain_standard() -> u8 {
    0x1Fu8
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake256(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    _sodium_shake256_ref(out, outlen, in_, inlen as usize)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake256_init(state: *mut crypto_xof_shake256_state) -> c_int {
    let st = state as *mut shake256_state_internal;
    _sodium_shake256_ref_init(st)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake256_init_with_domain(
    state: *mut crypto_xof_shake256_state,
    domain: u8,
) -> c_int {
    let st = state as *mut shake256_state_internal;
    _sodium_shake256_ref_init_with_domain(st, domain)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake256_update(
    state: *mut crypto_xof_shake256_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    let st = state as *mut shake256_state_internal;
    _sodium_shake256_ref_update(st, in_, inlen as usize)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_shake256_squeeze(
    state: *mut crypto_xof_shake256_state,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let st = state as *mut shake256_state_internal;
    _sodium_shake256_ref_squeeze(st, out, outlen)
}

// ===================== turboshake128 (rate = 168, crypto_core_keccak1600_permute_12) =====================

#[repr(C, align(16))]
pub struct crypto_xof_turboshake128_state {
    pub opaque: [u8; 256],
}

#[repr(C)]
struct turboshake128_state_internal {
    state: crypto_core_keccak1600_state,
    offset: usize,
    phase: u8,
    domain: u8,
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_turboshake128_ref_init_with_domain(
    state: *mut turboshake128_state_internal,
    domain: u8,
) -> c_int {
    crypto_core_keccak1600_init(&mut (*state).state);
    (*state).offset = 0;
    (*state).phase = PHASE_ABSORBING;
    (*state).domain = domain;

    0
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_turboshake128_ref_init(state: *mut turboshake128_state_internal) -> c_int {
    _sodium_turboshake128_ref_init_with_domain(state, 0x1Fu8)
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_turboshake128_ref_update(
    state: *mut turboshake128_state_internal,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    let mut consumed: usize = 0;
    let mut chunk_size: usize;
    let mut ret: c_int = 0;

    if (*state).phase != PHASE_ABSORBING {
        crypto_core_keccak1600_permute_12(&mut (*state).state);
        (*state).phase = PHASE_ABSORBING;
        (*state).offset = 0;
        ret = -1;
    }

    if (*state).offset == 168 && inlen > 0 {
        crypto_core_keccak1600_permute_12(&mut (*state).state);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && inlen > 0 {
        chunk_size = 168 - (*state).offset;
        if chunk_size > inlen {
            chunk_size = inlen;
        }
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_, (*state).offset, chunk_size);
        (*state).offset += chunk_size;
        consumed = chunk_size;
        if (*state).offset == 168 && consumed < inlen {
            crypto_core_keccak1600_permute_12(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    while inlen - consumed >= 168 {
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_.add(consumed), 0, 168);
        consumed += 168;
        (*state).offset = 168;
        if consumed < inlen {
            crypto_core_keccak1600_permute_12(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    if consumed < inlen {
        chunk_size = inlen - consumed;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_.add(consumed), 0, chunk_size);
        (*state).offset = chunk_size;
    }

    ret
}

unsafe fn turboshake128_finalize(state: *mut turboshake128_state_internal) {
    let mut pad: u8;

    if (*state).offset == 168 {
        crypto_core_keccak1600_permute_12(&mut (*state).state);
        (*state).offset = 0;
    }

    if (*state).offset == 168 - 1 {
        pad = (*state).domain ^ 0x80;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, &pad, (*state).offset, 1);
    } else {
        let domain = (*state).domain;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, &domain, (*state).offset, 1);
        pad = 0x80;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, &pad, 168 - 1, 1);
    }

    crypto_core_keccak1600_permute_12(&mut (*state).state);

    (*state).offset = 0;
    (*state).phase = PHASE_SQUEEZING;
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_turboshake128_ref_squeeze(
    state: *mut turboshake128_state_internal,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let mut extracted: usize = 0;
    let mut chunk_size: usize;

    if (*state).phase == PHASE_ABSORBING {
        turboshake128_finalize(state);
    }

    if (*state).offset == 168 && outlen > 0 {
        crypto_core_keccak1600_permute_12(&mut (*state).state);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && outlen > 0 {
        chunk_size = 168 - (*state).offset;
        if chunk_size > outlen {
            chunk_size = outlen;
        }
        crypto_core_keccak1600_extract_bytes(&(*state).state, out, (*state).offset, chunk_size);
        (*state).offset += chunk_size;
        extracted = chunk_size;
        if (*state).offset == 168 && extracted < outlen {
            crypto_core_keccak1600_permute_12(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    while outlen - extracted >= 168 {
        crypto_core_keccak1600_extract_bytes(&(*state).state, out.add(extracted), 0, 168);
        extracted += 168;
        (*state).offset = 168;
        if extracted < outlen {
            crypto_core_keccak1600_permute_12(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    if extracted < outlen {
        chunk_size = outlen - extracted;
        crypto_core_keccak1600_extract_bytes(&(*state).state, out.add(extracted), 0, chunk_size);
        (*state).offset = chunk_size;
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_turboshake128_ref(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    let mut state: turboshake128_state_internal = core::mem::zeroed();

    _sodium_turboshake128_ref_init(&mut state);
    _sodium_turboshake128_ref_update(&mut state, in_, inlen);
    _sodium_turboshake128_ref_squeeze(&mut state, out, outlen);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake128_blockbytes() -> usize {
    168
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake128_statebytes() -> usize {
    256
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake128_domain_standard() -> u8 {
    0x1Fu8
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake128(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    _sodium_turboshake128_ref(out, outlen, in_, inlen as usize)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake128_init(state: *mut crypto_xof_turboshake128_state) -> c_int {
    let st = state as *mut turboshake128_state_internal;
    _sodium_turboshake128_ref_init(st)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake128_init_with_domain(
    state: *mut crypto_xof_turboshake128_state,
    domain: u8,
) -> c_int {
    let st = state as *mut turboshake128_state_internal;
    _sodium_turboshake128_ref_init_with_domain(st, domain)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake128_update(
    state: *mut crypto_xof_turboshake128_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    let st = state as *mut turboshake128_state_internal;
    _sodium_turboshake128_ref_update(st, in_, inlen as usize)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake128_squeeze(
    state: *mut crypto_xof_turboshake128_state,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let st = state as *mut turboshake128_state_internal;
    _sodium_turboshake128_ref_squeeze(st, out, outlen)
}

// ===================== turboshake256 (rate = 136, crypto_core_keccak1600_permute_12) =====================

#[repr(C, align(16))]
pub struct crypto_xof_turboshake256_state {
    pub opaque: [u8; 256],
}

#[repr(C)]
struct turboshake256_state_internal {
    state: crypto_core_keccak1600_state,
    offset: usize,
    phase: u8,
    domain: u8,
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_turboshake256_ref_init_with_domain(
    state: *mut turboshake256_state_internal,
    domain: u8,
) -> c_int {
    crypto_core_keccak1600_init(&mut (*state).state);
    (*state).offset = 0;
    (*state).phase = PHASE_ABSORBING;
    (*state).domain = domain;

    0
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_turboshake256_ref_init(state: *mut turboshake256_state_internal) -> c_int {
    _sodium_turboshake256_ref_init_with_domain(state, 0x1Fu8)
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_turboshake256_ref_update(
    state: *mut turboshake256_state_internal,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    let mut consumed: usize = 0;
    let mut chunk_size: usize;
    let mut ret: c_int = 0;

    if (*state).phase != PHASE_ABSORBING {
        crypto_core_keccak1600_permute_12(&mut (*state).state);
        (*state).phase = PHASE_ABSORBING;
        (*state).offset = 0;
        ret = -1;
    }

    if (*state).offset == 136 && inlen > 0 {
        crypto_core_keccak1600_permute_12(&mut (*state).state);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && inlen > 0 {
        chunk_size = 136 - (*state).offset;
        if chunk_size > inlen {
            chunk_size = inlen;
        }
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_, (*state).offset, chunk_size);
        (*state).offset += chunk_size;
        consumed = chunk_size;
        if (*state).offset == 136 && consumed < inlen {
            crypto_core_keccak1600_permute_12(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    while inlen - consumed >= 136 {
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_.add(consumed), 0, 136);
        consumed += 136;
        (*state).offset = 136;
        if consumed < inlen {
            crypto_core_keccak1600_permute_12(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    if consumed < inlen {
        chunk_size = inlen - consumed;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_.add(consumed), 0, chunk_size);
        (*state).offset = chunk_size;
    }

    ret
}

unsafe fn turboshake256_finalize(state: *mut turboshake256_state_internal) {
    let mut pad: u8;

    if (*state).offset == 136 {
        crypto_core_keccak1600_permute_12(&mut (*state).state);
        (*state).offset = 0;
    }

    if (*state).offset == 136 - 1 {
        pad = (*state).domain ^ 0x80;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, &pad, (*state).offset, 1);
    } else {
        let domain = (*state).domain;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, &domain, (*state).offset, 1);
        pad = 0x80;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, &pad, 136 - 1, 1);
    }

    crypto_core_keccak1600_permute_12(&mut (*state).state);

    (*state).offset = 0;
    (*state).phase = PHASE_SQUEEZING;
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_turboshake256_ref_squeeze(
    state: *mut turboshake256_state_internal,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let mut extracted: usize = 0;
    let mut chunk_size: usize;

    if (*state).phase == PHASE_ABSORBING {
        turboshake256_finalize(state);
    }

    if (*state).offset == 136 && outlen > 0 {
        crypto_core_keccak1600_permute_12(&mut (*state).state);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && outlen > 0 {
        chunk_size = 136 - (*state).offset;
        if chunk_size > outlen {
            chunk_size = outlen;
        }
        crypto_core_keccak1600_extract_bytes(&(*state).state, out, (*state).offset, chunk_size);
        (*state).offset += chunk_size;
        extracted = chunk_size;
        if (*state).offset == 136 && extracted < outlen {
            crypto_core_keccak1600_permute_12(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    while outlen - extracted >= 136 {
        crypto_core_keccak1600_extract_bytes(&(*state).state, out.add(extracted), 0, 136);
        extracted += 136;
        (*state).offset = 136;
        if extracted < outlen {
            crypto_core_keccak1600_permute_12(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    if extracted < outlen {
        chunk_size = outlen - extracted;
        crypto_core_keccak1600_extract_bytes(&(*state).state, out.add(extracted), 0, chunk_size);
        (*state).offset = chunk_size;
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_turboshake256_ref(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    let mut state: turboshake256_state_internal = core::mem::zeroed();

    _sodium_turboshake256_ref_init(&mut state);
    _sodium_turboshake256_ref_update(&mut state, in_, inlen);
    _sodium_turboshake256_ref_squeeze(&mut state, out, outlen);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake256_blockbytes() -> usize {
    136
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake256_statebytes() -> usize {
    256
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake256_domain_standard() -> u8 {
    0x1Fu8
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake256(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    _sodium_turboshake256_ref(out, outlen, in_, inlen as usize)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake256_init(state: *mut crypto_xof_turboshake256_state) -> c_int {
    let st = state as *mut turboshake256_state_internal;
    _sodium_turboshake256_ref_init(st)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake256_init_with_domain(
    state: *mut crypto_xof_turboshake256_state,
    domain: u8,
) -> c_int {
    let st = state as *mut turboshake256_state_internal;
    _sodium_turboshake256_ref_init_with_domain(st, domain)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake256_update(
    state: *mut crypto_xof_turboshake256_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    let st = state as *mut turboshake256_state_internal;
    _sodium_turboshake256_ref_update(st, in_, inlen as usize)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_xof_turboshake256_squeeze(
    state: *mut crypto_xof_turboshake256_state,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let st = state as *mut turboshake256_state_internal;
    _sodium_turboshake256_ref_squeeze(st, out, outlen)
}
