use crate::crypto_core::keccak1600::{
    crypto_core_keccak1600_extract_bytes, crypto_core_keccak1600_init,
    crypto_core_keccak1600_permute_24, crypto_core_keccak1600_state,
    crypto_core_keccak1600_xor_bytes,
};

/* ---- constants from crypto_xof_shake256.h / shake256_ref.h ---- */

const crypto_xof_shake256_BLOCKBYTES: usize = 136;
const crypto_xof_shake256_STATEBYTES: usize = 256;
const crypto_xof_shake256_DOMAIN_STANDARD: u8 = 0x1F;

const SHAKE256_RATE: usize = 136;

const SHAKE256_PHASE_ABSORBING: u8 = 0;
const SHAKE256_PHASE_SQUEEZING: u8 = 1;

/*
 * typedef struct shake256_state_internal_ {
 *     crypto_core_keccak1600_state state;
 *     size_t                       offset;
 *     uint8_t                      phase;
 *     unsigned char                domain;
 * } shake256_state_internal;
 */
#[repr(C)]
pub struct shake256_state_internal {
    pub state: crypto_core_keccak1600_state,
    pub offset: usize,
    pub phase: u8,
    pub domain: u8,
}

/* ---- public API constant accessors (from xof_shake256.c) ---- */

#[unsafe(no_mangle)]
pub extern "C" fn crypto_xof_shake256_blockbytes() -> usize {
    crypto_xof_shake256_BLOCKBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_xof_shake256_statebytes() -> usize {
    crypto_xof_shake256_STATEBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_xof_shake256_domain_standard() -> u8 {
    crypto_xof_shake256_DOMAIN_STANDARD
}

/* ---- reference implementation (_sodium_-prefixed exported symbols) ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_shake256_ref_init_with_domain(
    state: *mut shake256_state_internal,
    domain: u8,
) -> i32 {
    crypto_core_keccak1600_init(&mut (*state).state);
    (*state).offset = 0;
    (*state).phase = SHAKE256_PHASE_ABSORBING;
    (*state).domain = domain;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_shake256_ref_init(state: *mut shake256_state_internal) -> i32 {
    _sodium_shake256_ref_init_with_domain(state, crypto_xof_shake256_DOMAIN_STANDARD)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_shake256_ref_update(
    state: *mut shake256_state_internal,
    in_: *const u8,
    inlen: usize,
) -> i32 {
    let mut consumed: usize = 0;
    let mut chunk_size: usize;
    let mut ret: i32 = 0;

    if (*state).phase != SHAKE256_PHASE_ABSORBING {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        (*state).phase = SHAKE256_PHASE_ABSORBING;
        (*state).offset = 0;
        ret = -1;
    }

    if (*state).offset == SHAKE256_RATE && inlen > 0 {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && inlen > 0 {
        chunk_size = SHAKE256_RATE - (*state).offset;
        if chunk_size > inlen {
            chunk_size = inlen;
        }
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_, (*state).offset, chunk_size);
        (*state).offset += chunk_size;
        consumed = chunk_size;
        if (*state).offset == SHAKE256_RATE && consumed < inlen {
            crypto_core_keccak1600_permute_24(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    while inlen - consumed >= SHAKE256_RATE {
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, in_.add(consumed), 0, SHAKE256_RATE);
        consumed += SHAKE256_RATE;
        (*state).offset = SHAKE256_RATE;
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
    let pad: u8;

    /* If the rate is exactly full, process that block before padding */
    if (*state).offset == SHAKE256_RATE {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        (*state).offset = 0;
    }

    /* Apply padding: domain byte at current position, 0x80 at last byte */
    if (*state).offset == SHAKE256_RATE - 1 {
        /* Special case: padding fits in one byte */
        pad = (*state).domain ^ 0x80;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, &pad, (*state).offset, 1);
    } else {
        /* Normal case: domain and 0x80 at different positions */
        crypto_core_keccak1600_xor_bytes(
            &mut (*state).state,
            &(*state).domain,
            (*state).offset,
            1,
        );
        pad = 0x80;
        crypto_core_keccak1600_xor_bytes(&mut (*state).state, &pad, SHAKE256_RATE - 1, 1);
    }

    /* Final permutation */
    crypto_core_keccak1600_permute_24(&mut (*state).state);

    (*state).offset = 0;
    (*state).phase = SHAKE256_PHASE_SQUEEZING;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_shake256_ref_squeeze(
    state: *mut shake256_state_internal,
    out: *mut u8,
    outlen: usize,
) -> i32 {
    let mut extracted: usize = 0;
    let mut chunk_size: usize;

    if (*state).phase == SHAKE256_PHASE_ABSORBING {
        shake256_finalize(state);
    }

    if (*state).offset == SHAKE256_RATE && outlen > 0 {
        crypto_core_keccak1600_permute_24(&mut (*state).state);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && outlen > 0 {
        chunk_size = SHAKE256_RATE - (*state).offset;
        if chunk_size > outlen {
            chunk_size = outlen;
        }
        crypto_core_keccak1600_extract_bytes(&(*state).state, out, (*state).offset, chunk_size);
        (*state).offset += chunk_size;
        extracted = chunk_size;
        if (*state).offset == SHAKE256_RATE && extracted < outlen {
            crypto_core_keccak1600_permute_24(&mut (*state).state);
            (*state).offset = 0;
        }
    }
    while outlen - extracted >= SHAKE256_RATE {
        crypto_core_keccak1600_extract_bytes(
            &(*state).state,
            out.add(extracted),
            0,
            SHAKE256_RATE,
        );
        extracted += SHAKE256_RATE;
        (*state).offset = SHAKE256_RATE;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_shake256_ref(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: usize,
) -> i32 {
    let mut state: shake256_state_internal = core::mem::zeroed();

    _sodium_shake256_ref_init(&mut state);
    _sodium_shake256_ref_update(&mut state, in_, inlen);
    _sodium_shake256_ref_squeeze(&mut state, out, outlen);

    0
}

/* ---- public streaming/one-shot API (from xof_shake256.c) ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake256(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
) -> i32 {
    _sodium_shake256_ref(out, outlen, in_, inlen as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake256_init(
    state: *mut crypto_xof_shake256_state,
) -> i32 {
    let st = state as *mut shake256_state_internal;

    _sodium_shake256_ref_init(st)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake256_init_with_domain(
    state: *mut crypto_xof_shake256_state,
    domain: u8,
) -> i32 {
    let st = state as *mut shake256_state_internal;

    _sodium_shake256_ref_init_with_domain(st, domain)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake256_update(
    state: *mut crypto_xof_shake256_state,
    in_: *const u8,
    inlen: u64,
) -> i32 {
    let st = state as *mut shake256_state_internal;

    _sodium_shake256_ref_update(st, in_, inlen as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_xof_shake256_squeeze(
    state: *mut crypto_xof_shake256_state,
    out: *mut u8,
    outlen: usize,
) -> i32 {
    let st = state as *mut shake256_state_internal;

    _sodium_shake256_ref_squeeze(st, out, outlen)
}

/*
 * typedef struct CRYPTO_ALIGN(16) crypto_xof_shake256_state {
 *     unsigned char opaque[256];
 * } crypto_xof_shake256_state;
 */
#[repr(C, align(16))]
pub struct crypto_xof_shake256_state {
    pub opaque: [u8; 256],
}
