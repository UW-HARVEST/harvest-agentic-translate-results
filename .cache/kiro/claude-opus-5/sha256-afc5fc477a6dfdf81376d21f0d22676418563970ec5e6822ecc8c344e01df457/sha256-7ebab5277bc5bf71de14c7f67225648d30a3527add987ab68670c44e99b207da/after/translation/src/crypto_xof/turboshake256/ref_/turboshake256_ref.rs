//! Translation of c_src/libsodium/crypto_xof/turboshake256/ref/turboshake256_ref.c

use core::ffi::{c_int, c_void};

// Local repr(C) copy of crypto_core_keccak1600_state (rule 4). CRYPTO_ALIGN(16)
// with #pragma pack(1); pack(1) has no effect on an all-`u8` struct and Rust
// forbids packed+align, so repr(C, align(16)) reproduces it (size 224, align 16).
#[repr(C, align(16))]
struct crypto_core_keccak1600_state {
    opaque: [u8; 224],
}

const TURBOSHAKE256_RATE: usize = 136;

// typedef enum { TURBOSHAKE256_PHASE_ABSORBING = 0, TURBOSHAKE256_PHASE_SQUEEZING = 1 }
const TURBOSHAKE256_PHASE_ABSORBING: u8 = 0;
const TURBOSHAKE256_PHASE_SQUEEZING: u8 = 1;

// #define crypto_xof_turboshake256_DOMAIN_STANDARD 0x1FU
const crypto_xof_turboshake256_DOMAIN_STANDARD: u8 = 0x1F;

#[repr(C)]
struct turboshake256_state_internal {
    state: crypto_core_keccak1600_state,
    offset: usize,
    phase: u8,
    domain: u8,
}

extern "C" {
    fn crypto_core_keccak1600_init(state: *mut c_void);
    fn crypto_core_keccak1600_xor_bytes(
        state: *mut c_void,
        bytes: *const u8,
        offset: usize,
        length: usize,
    );
    fn crypto_core_keccak1600_extract_bytes(
        state: *const c_void,
        bytes: *mut u8,
        offset: usize,
        length: usize,
    );
    fn crypto_core_keccak1600_permute_12(state: *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_turboshake256_ref_init_with_domain(
    state: *mut turboshake256_state_internal,
    domain: u8,
) -> c_int {
    crypto_core_keccak1600_init(core::ptr::addr_of_mut!((*state).state) as *mut c_void);
    (*state).offset = 0;
    (*state).phase = TURBOSHAKE256_PHASE_ABSORBING;
    (*state).domain = domain;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_turboshake256_ref_init(
    state: *mut turboshake256_state_internal,
) -> c_int {
    _sodium_turboshake256_ref_init_with_domain(state, crypto_xof_turboshake256_DOMAIN_STANDARD)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_turboshake256_ref_update(
    state: *mut turboshake256_state_internal,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    let mut consumed: usize = 0;
    let mut chunk_size: usize;
    let mut ret: c_int = 0;

    if (*state).phase != TURBOSHAKE256_PHASE_ABSORBING {
        crypto_core_keccak1600_permute_12(core::ptr::addr_of_mut!((*state).state) as *mut c_void);
        (*state).phase = TURBOSHAKE256_PHASE_ABSORBING;
        (*state).offset = 0;
        ret = -1;
    }

    if (*state).offset == TURBOSHAKE256_RATE && inlen > 0 {
        crypto_core_keccak1600_permute_12(core::ptr::addr_of_mut!((*state).state) as *mut c_void);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && inlen > 0 {
        chunk_size = TURBOSHAKE256_RATE - (*state).offset;
        if chunk_size > inlen {
            chunk_size = inlen;
        }
        crypto_core_keccak1600_xor_bytes(
            core::ptr::addr_of_mut!((*state).state) as *mut c_void,
            in_,
            (*state).offset,
            chunk_size,
        );
        (*state).offset += chunk_size;
        consumed = chunk_size;
        if (*state).offset == TURBOSHAKE256_RATE && consumed < inlen {
            crypto_core_keccak1600_permute_12(
                core::ptr::addr_of_mut!((*state).state) as *mut c_void
            );
            (*state).offset = 0;
        }
    }
    while inlen - consumed >= TURBOSHAKE256_RATE {
        crypto_core_keccak1600_xor_bytes(
            core::ptr::addr_of_mut!((*state).state) as *mut c_void,
            in_.add(consumed),
            0,
            TURBOSHAKE256_RATE,
        );
        consumed += TURBOSHAKE256_RATE;
        (*state).offset = TURBOSHAKE256_RATE;
        if consumed < inlen {
            crypto_core_keccak1600_permute_12(
                core::ptr::addr_of_mut!((*state).state) as *mut c_void
            );
            (*state).offset = 0;
        }
    }
    if consumed < inlen {
        chunk_size = inlen - consumed;
        crypto_core_keccak1600_xor_bytes(
            core::ptr::addr_of_mut!((*state).state) as *mut c_void,
            in_.add(consumed),
            0,
            chunk_size,
        );
        (*state).offset = chunk_size;
    }

    ret
}

unsafe fn turboshake256_finalize(state: *mut turboshake256_state_internal) {
    let mut pad: u8;

    // If the rate is exactly full, process that block before padding
    if (*state).offset == TURBOSHAKE256_RATE {
        crypto_core_keccak1600_permute_12(core::ptr::addr_of_mut!((*state).state) as *mut c_void);
        (*state).offset = 0;
    }

    // Apply padding: domain byte at current position, 0x80 at last byte
    if (*state).offset == TURBOSHAKE256_RATE - 1 {
        // Special case: padding fits in one byte
        pad = (*state).domain ^ 0x80;
        crypto_core_keccak1600_xor_bytes(
            core::ptr::addr_of_mut!((*state).state) as *mut c_void,
            &pad,
            (*state).offset,
            1,
        );
    } else {
        // Normal case: domain and 0x80 at different positions
        crypto_core_keccak1600_xor_bytes(
            core::ptr::addr_of_mut!((*state).state) as *mut c_void,
            core::ptr::addr_of!((*state).domain),
            (*state).offset,
            1,
        );
        pad = 0x80;
        crypto_core_keccak1600_xor_bytes(
            core::ptr::addr_of_mut!((*state).state) as *mut c_void,
            &pad,
            TURBOSHAKE256_RATE - 1,
            1,
        );
    }

    // Final permutation (12 rounds for TurboSHAKE)
    crypto_core_keccak1600_permute_12(core::ptr::addr_of_mut!((*state).state) as *mut c_void);

    (*state).offset = 0;
    (*state).phase = TURBOSHAKE256_PHASE_SQUEEZING;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_turboshake256_ref_squeeze(
    state: *mut turboshake256_state_internal,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let mut extracted: usize = 0;
    let mut chunk_size: usize;

    if (*state).phase == TURBOSHAKE256_PHASE_ABSORBING {
        turboshake256_finalize(state);
    }

    if (*state).offset == TURBOSHAKE256_RATE && outlen > 0 {
        crypto_core_keccak1600_permute_12(core::ptr::addr_of_mut!((*state).state) as *mut c_void);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && outlen > 0 {
        chunk_size = TURBOSHAKE256_RATE - (*state).offset;
        if chunk_size > outlen {
            chunk_size = outlen;
        }
        crypto_core_keccak1600_extract_bytes(
            core::ptr::addr_of!((*state).state) as *const c_void,
            out,
            (*state).offset,
            chunk_size,
        );
        (*state).offset += chunk_size;
        extracted = chunk_size;
        if (*state).offset == TURBOSHAKE256_RATE && extracted < outlen {
            crypto_core_keccak1600_permute_12(
                core::ptr::addr_of_mut!((*state).state) as *mut c_void
            );
            (*state).offset = 0;
        }
    }
    while outlen - extracted >= TURBOSHAKE256_RATE {
        crypto_core_keccak1600_extract_bytes(
            core::ptr::addr_of!((*state).state) as *const c_void,
            out.add(extracted),
            0,
            TURBOSHAKE256_RATE,
        );
        extracted += TURBOSHAKE256_RATE;
        (*state).offset = TURBOSHAKE256_RATE;
        if extracted < outlen {
            crypto_core_keccak1600_permute_12(
                core::ptr::addr_of_mut!((*state).state) as *mut c_void
            );
            (*state).offset = 0;
        }
    }
    if extracted < outlen {
        chunk_size = outlen - extracted;
        crypto_core_keccak1600_extract_bytes(
            core::ptr::addr_of!((*state).state) as *const c_void,
            out.add(extracted),
            0,
            chunk_size,
        );
        (*state).offset = chunk_size;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_turboshake256_ref(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    let mut state = core::mem::MaybeUninit::<turboshake256_state_internal>::uninit();
    let state = state.as_mut_ptr();

    _sodium_turboshake256_ref_init(state);
    _sodium_turboshake256_ref_update(state, in_, inlen);
    _sodium_turboshake256_ref_squeeze(state, out, outlen);

    0
}
