//! Translation of `crypto_hash/sha3/hash_sha3.c`.
//!
//! Implements the FIPS 202 SHA3-256 and SHA3-512 hash functions on top of
//! `crypto_core_keccak1600`. Header: `include/sodium/crypto_hash_sha3.h`.

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

    fn sodium_memzero(pnt: *mut core::ffi::c_void, len: usize);
}

const SHA3_PHASE_ABSORBING: u8 = 0;
const SHA3_PHASE_FINALIZED: u8 = 1;

/// `crypto_hash_sha3256_state` from `crypto_hash_sha3.h`.
#[repr(C, align(16))]
pub struct crypto_hash_sha3256_state {
    pub opaque: [u8; 256],
}

/// `crypto_hash_sha3512_state` from `crypto_hash_sha3.h`.
#[repr(C, align(16))]
pub struct crypto_hash_sha3512_state {
    pub opaque: [u8; 256],
}

#[repr(C)]
struct sha3_state_internal {
    state: crypto_core_keccak1600_state,
    offset: usize,
    rate: usize,
    outlen: usize,
    phase: u8,
}

unsafe fn sha3_init(state: *mut sha3_state_internal, rate: usize, outlen: usize) -> c_int {
    crypto_core_keccak1600_init(&mut (*state).state);
    (*state).offset = 0;
    (*state).rate = rate;
    (*state).outlen = outlen;
    (*state).phase = SHA3_PHASE_ABSORBING;

    0
}

unsafe fn sha3_update(state: *mut sha3_state_internal, in_: *const u8, inlen: usize) -> c_int {
    let mut consumed: usize = 0;
    let mut chunk_size: usize;
    let mut ret: c_int = 0;

    if (*state).phase != SHA3_PHASE_ABSORBING {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        (*state).phase = SHA3_PHASE_ABSORBING;
        (*state).offset = 0;
        ret = -1;
    }

    if (*state).offset == (*state).rate && inlen > 0 {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && inlen > 0 {
        chunk_size = (*state).rate - (*state).offset;
        if chunk_size > inlen {
            chunk_size = inlen;
        }
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_, (*state).offset, chunk_size);
        (*state).offset += chunk_size;
        consumed = chunk_size;
        if (*state).offset == (*state).rate && consumed < inlen {
            crypto_core_keccak1600_permute_24(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    while inlen - consumed >= (*state).rate {
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_.add(consumed), 0, (*state).rate);
        consumed += (*state).rate;
        (*state).offset = (*state).rate;
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

unsafe fn sha3_final(state: *mut sha3_state_internal, out: *mut u8) -> c_int {
    let mut pad: u8;
    let mut ret: c_int = 0;

    if (*state).phase != SHA3_PHASE_ABSORBING {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        ret = -1;
    } else {
        if (*state).offset == (*state).rate {
            crypto_core_keccak1600_permute_24(&mut (*state).state);
            (*state).offset = 0;
        }

        if (*state).offset == (*state).rate - 1 {
            pad = 0x06u8 ^ 0x80u8;
            crypto_core_keccak1600_xor_bytes(&mut (*state).state, &pad, (*state).offset, 1);
        } else {
            pad = 0x06;
            crypto_core_keccak1600_xor_bytes(&mut (*state).state, &pad, (*state).offset, 1);
            pad = 0x80;
            crypto_core_keccak1600_xor_bytes(&mut (*state).state, &pad, (*state).rate - 1, 1);
        }

        crypto_core_keccak1600_permute_24(&mut (*state).state);
    }

    crypto_core_keccak1600_extract_bytes(&(*state).state, out, 0, (*state).outlen);
    (*state).offset = 0;
    (*state).phase = SHA3_PHASE_FINALIZED;

    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha3256_bytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha3256_statebytes() -> usize {
    core::mem::size_of::<crypto_hash_sha3256_state>()
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha3256_init(state: *mut crypto_hash_sha3256_state) -> c_int {
    sha3_init(state as *mut sha3_state_internal, 136, 32)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha3256_update(
    state: *mut crypto_hash_sha3256_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    sha3_update(state as *mut sha3_state_internal, in_, inlen as usize)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha3256_final(
    state: *mut crypto_hash_sha3256_state,
    out: *mut u8,
) -> c_int {
    sha3_final(state as *mut sha3_state_internal, out)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha3256(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    let mut state: crypto_hash_sha3256_state = crypto_hash_sha3256_state { opaque: [0u8; 256] };

    crypto_hash_sha3256_init(&mut state);
    crypto_hash_sha3256_update(&mut state, in_, inlen);
    crypto_hash_sha3256_final(&mut state, out);
    sodium_memzero(
        &mut state as *mut crypto_hash_sha3256_state as *mut core::ffi::c_void,
        core::mem::size_of::<crypto_hash_sha3256_state>(),
    );

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha3512_bytes() -> usize {
    64
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha3512_statebytes() -> usize {
    core::mem::size_of::<crypto_hash_sha3512_state>()
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha3512_init(state: *mut crypto_hash_sha3512_state) -> c_int {
    sha3_init(state as *mut sha3_state_internal, 72, 64)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha3512_update(
    state: *mut crypto_hash_sha3512_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    sha3_update(state as *mut sha3_state_internal, in_, inlen as usize)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha3512_final(
    state: *mut crypto_hash_sha3512_state,
    out: *mut u8,
) -> c_int {
    sha3_final(state as *mut sha3_state_internal, out)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha3512(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    let mut state: crypto_hash_sha3512_state = crypto_hash_sha3512_state { opaque: [0u8; 256] };

    crypto_hash_sha3512_init(&mut state);
    crypto_hash_sha3512_update(&mut state, in_, inlen);
    crypto_hash_sha3512_final(&mut state, out);
    sodium_memzero(
        &mut state as *mut crypto_hash_sha3512_state as *mut core::ffi::c_void,
        core::mem::size_of::<crypto_hash_sha3512_state>(),
    );

    0
}
