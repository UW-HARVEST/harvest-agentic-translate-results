//! Translated from crypto_hash/sha3/hash_sha3.c
use crate::primitives::cutil::*;
use crate::primitives::keccak::{core_extract_bytes, core_init, core_permute_24, core_xor_bytes};

const SHA3_256_RATE: usize = 136;
const SHA3_512_RATE: usize = 72;
const SHA3_DOMAIN: u8 = 0x06;

const SHA3_PHASE_ABSORBING: u8 = 0;
const SHA3_PHASE_FINALIZED: u8 = 1;

// crypto_core_keccak1600_state = opaque[224]; sha3_state_internal layout:
//   state (224 bytes, align 16), offset (usize), rate (usize), outlen (usize), phase (u8)
// The public opaque state buffers are 256 bytes (crypto_hash_sha3256_state/512_state).
#[repr(C, align(16))]
struct Sha3StateInternal {
    state: [u8; 224],
    offset: usize,
    rate: usize,
    outlen: usize,
    phase: u8,
}

unsafe fn sha3_init(state: *mut Sha3StateInternal, rate: usize, outlen: usize) -> i32 {
    let s = &mut *state;
    core_init(s.state.as_mut_ptr());
    s.offset = 0;
    s.rate = rate;
    s.outlen = outlen;
    s.phase = SHA3_PHASE_ABSORBING;
    0
}

unsafe fn sha3_update(state: *mut Sha3StateInternal, input: *const u8, inlen: usize) -> i32 {
    let s = &mut *state;
    let mut consumed = 0usize;
    let mut ret = 0i32;

    if s.phase != SHA3_PHASE_ABSORBING {
        core_permute_24(s.state.as_mut_ptr());
        s.phase = SHA3_PHASE_ABSORBING;
        s.offset = 0;
        ret = -1;
    }

    if s.offset == s.rate && inlen > 0 {
        core_permute_24(s.state.as_mut_ptr());
        s.offset = 0;
    }
    if s.offset != 0 && inlen > 0 {
        let mut chunk_size = s.rate - s.offset;
        if chunk_size > inlen {
            chunk_size = inlen;
        }
        core_xor_bytes(s.state.as_mut_ptr(), input, s.offset, chunk_size);
        s.offset += chunk_size;
        consumed = chunk_size;
        if s.offset == s.rate && consumed < inlen {
            core_permute_24(s.state.as_mut_ptr());
            s.offset = 0;
        }
    }
    while inlen - consumed >= s.rate {
        core_xor_bytes(s.state.as_mut_ptr(), input.add(consumed), 0, s.rate);
        consumed += s.rate;
        s.offset = s.rate;
        if consumed < inlen {
            core_permute_24(s.state.as_mut_ptr());
            s.offset = 0;
        }
    }
    if consumed < inlen {
        let chunk_size = inlen - consumed;
        core_xor_bytes(s.state.as_mut_ptr(), input.add(consumed), 0, chunk_size);
        s.offset = chunk_size;
    }
    ret
}

unsafe fn sha3_final(state: *mut Sha3StateInternal, out: *mut u8) -> i32 {
    let s = &mut *state;
    let mut pad: u8;
    let mut ret = 0i32;

    if s.phase != SHA3_PHASE_ABSORBING {
        core_permute_24(s.state.as_mut_ptr());
        ret = -1;
    } else {
        if s.offset == s.rate {
            core_permute_24(s.state.as_mut_ptr());
            s.offset = 0;
        }
        if s.offset == s.rate - 1 {
            pad = SHA3_DOMAIN ^ 0x80;
            core_xor_bytes(s.state.as_mut_ptr(), &pad, s.offset, 1);
        } else {
            pad = SHA3_DOMAIN;
            core_xor_bytes(s.state.as_mut_ptr(), &pad, s.offset, 1);
            pad = 0x80;
            core_xor_bytes(s.state.as_mut_ptr(), &pad, s.rate - 1, 1);
        }
        core_permute_24(s.state.as_mut_ptr());
    }
    core_extract_bytes(s.state.as_ptr(), out, 0, s.outlen);
    s.offset = 0;
    s.phase = SHA3_PHASE_FINALIZED;
    ret
}

// public opaque state: crypto_hash_sha3256_state / crypto_hash_sha3512_state = opaque[256]
#[repr(C, align(16))]
pub struct crypto_hash_sha3256_state {
    pub opaque: [u8; 256],
}
#[repr(C, align(16))]
pub struct crypto_hash_sha3512_state {
    pub opaque: [u8; 256],
}

// ---- sha3-256 ----

#[unsafe(no_mangle)]
pub extern "C" fn crypto_hash_sha3256_bytes() -> usize {
    32
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_hash_sha3256_statebytes() -> usize {
    core::mem::size_of::<crypto_hash_sha3256_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha3256_init(state: *mut crypto_hash_sha3256_state) -> i32 {
    sha3_init(state as *mut Sha3StateInternal, SHA3_256_RATE, 32)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha3256_update(
    state: *mut crypto_hash_sha3256_state,
    input: *const u8,
    inlen: u64,
) -> i32 {
    sha3_update(state as *mut Sha3StateInternal, input, inlen as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha3256_final(
    state: *mut crypto_hash_sha3256_state,
    out: *mut u8,
) -> i32 {
    sha3_final(state as *mut Sha3StateInternal, out)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha3256(
    out: *mut u8,
    input: *const u8,
    inlen: u64,
) -> i32 {
    let mut state = crypto_hash_sha3256_state { opaque: [0; 256] };
    crypto_hash_sha3256_init(&mut state);
    crypto_hash_sha3256_update(&mut state, input, inlen);
    crypto_hash_sha3256_final(&mut state, out);
    sodium_memzero(
        &mut state as *mut _ as *mut core::ffi::c_void,
        core::mem::size_of::<crypto_hash_sha3256_state>(),
    );
    0
}

// ---- sha3-512 ----

#[unsafe(no_mangle)]
pub extern "C" fn crypto_hash_sha3512_bytes() -> usize {
    64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_hash_sha3512_statebytes() -> usize {
    core::mem::size_of::<crypto_hash_sha3512_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha3512_init(state: *mut crypto_hash_sha3512_state) -> i32 {
    sha3_init(state as *mut Sha3StateInternal, SHA3_512_RATE, 64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha3512_update(
    state: *mut crypto_hash_sha3512_state,
    input: *const u8,
    inlen: u64,
) -> i32 {
    sha3_update(state as *mut Sha3StateInternal, input, inlen as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha3512_final(
    state: *mut crypto_hash_sha3512_state,
    out: *mut u8,
) -> i32 {
    sha3_final(state as *mut Sha3StateInternal, out)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha3512(
    out: *mut u8,
    input: *const u8,
    inlen: u64,
) -> i32 {
    let mut state = crypto_hash_sha3512_state { opaque: [0; 256] };
    crypto_hash_sha3512_init(&mut state);
    crypto_hash_sha3512_update(&mut state, input, inlen);
    crypto_hash_sha3512_final(&mut state, out);
    sodium_memzero(
        &mut state as *mut _ as *mut core::ffi::c_void,
        core::mem::size_of::<crypto_hash_sha3512_state>(),
    );
    0
}
