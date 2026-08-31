//! Translation of c_src/libsodium/crypto_xof/shake128/ref/shake128_ref.c

use core::ffi::{c_int, c_void};

// Local repr(C) copy of crypto_core_keccak1600_state (rule 4). CRYPTO_ALIGN(16)
// with #pragma pack(1); pack(1) has no effect on an all-`u8` struct and Rust
// forbids packed+align, so repr(C, align(16)) reproduces it (size 224, align 16).
#[repr(C, align(16))]
struct crypto_core_keccak1600_state {
    opaque: [u8; 224],
}

const SHAKE128_RATE: usize = 168;

// typedef enum { SHAKE128_PHASE_ABSORBING = 0, SHAKE128_PHASE_SQUEEZING = 1 }
const SHAKE128_PHASE_ABSORBING: u8 = 0;
const SHAKE128_PHASE_SQUEEZING: u8 = 1;

// #define crypto_xof_shake128_DOMAIN_STANDARD 0x1FU
const crypto_xof_shake128_DOMAIN_STANDARD: u8 = 0x1F;

#[repr(C)]
struct shake128_state_internal {
    state: crypto_core_keccak1600_state,
    offset: usize,
    phase: u8,
    domain: u8,
}

// Cross-file calls to the keccak1600 core (rule 3).
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
    fn crypto_core_keccak1600_permute_24(state: *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_shake128_ref_init_with_domain(
    state: *mut shake128_state_internal,
    domain: u8,
) -> c_int {
    crypto_core_keccak1600_init(core::ptr::addr_of_mut!((*state).state) as *mut c_void);
    (*state).offset = 0;
    (*state).phase = SHAKE128_PHASE_ABSORBING;
    (*state).domain = domain;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_shake128_ref_init(
    state: *mut shake128_state_internal,
) -> c_int {
    _sodium_shake128_ref_init_with_domain(state, crypto_xof_shake128_DOMAIN_STANDARD)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_shake128_ref_update(
    state: *mut shake128_state_internal,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    let mut consumed: usize = 0;
    let mut chunk_size: usize;
    let mut ret: c_int = 0;

    if (*state).phase != SHAKE128_PHASE_ABSORBING {
        crypto_core_keccak1600_permute_24(core::ptr::addr_of_mut!((*state).state) as *mut c_void);
        (*state).phase = SHAKE128_PHASE_ABSORBING;
        (*state).offset = 0;
        ret = -1;
    }

    if (*state).offset == SHAKE128_RATE && inlen > 0 {
        crypto_core_keccak1600_permute_24(core::ptr::addr_of_mut!((*state).state) as *mut c_void);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && inlen > 0 {
        chunk_size = SHAKE128_RATE - (*state).offset;
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
        if (*state).offset == SHAKE128_RATE && consumed < inlen {
            crypto_core_keccak1600_permute_24(
                core::ptr::addr_of_mut!((*state).state) as *mut c_void
            );
            (*state).offset = 0;
        }
    }
    while inlen - consumed >= SHAKE128_RATE {
        crypto_core_keccak1600_xor_bytes(
            core::ptr::addr_of_mut!((*state).state) as *mut c_void,
            in_.add(consumed),
            0,
            SHAKE128_RATE,
        );
        consumed += SHAKE128_RATE;
        (*state).offset = SHAKE128_RATE;
        if consumed < inlen {
            crypto_core_keccak1600_permute_24(
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

unsafe fn shake128_finalize(state: *mut shake128_state_internal) {
    let mut pad: u8;

    // If the rate is exactly full, process that block before padding
    if (*state).offset == SHAKE128_RATE {
        crypto_core_keccak1600_permute_24(core::ptr::addr_of_mut!((*state).state) as *mut c_void);
        (*state).offset = 0;
    }

    // Apply padding: domain byte at current position, 0x80 at last byte
    if (*state).offset == SHAKE128_RATE - 1 {
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
            SHAKE128_RATE - 1,
            1,
        );
    }

    // Final permutation
    crypto_core_keccak1600_permute_24(core::ptr::addr_of_mut!((*state).state) as *mut c_void);

    (*state).offset = 0;
    (*state).phase = SHAKE128_PHASE_SQUEEZING;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_shake128_ref_squeeze(
    state: *mut shake128_state_internal,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    let mut extracted: usize = 0;
    let mut chunk_size: usize;

    if (*state).phase == SHAKE128_PHASE_ABSORBING {
        shake128_finalize(state);
    }

    if (*state).offset == SHAKE128_RATE && outlen > 0 {
        crypto_core_keccak1600_permute_24(core::ptr::addr_of_mut!((*state).state) as *mut c_void);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && outlen > 0 {
        chunk_size = SHAKE128_RATE - (*state).offset;
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
        if (*state).offset == SHAKE128_RATE && extracted < outlen {
            crypto_core_keccak1600_permute_24(
                core::ptr::addr_of_mut!((*state).state) as *mut c_void
            );
            (*state).offset = 0;
        }
    }
    while outlen - extracted >= SHAKE128_RATE {
        crypto_core_keccak1600_extract_bytes(
            core::ptr::addr_of!((*state).state) as *const c_void,
            out.add(extracted),
            0,
            SHAKE128_RATE,
        );
        extracted += SHAKE128_RATE;
        (*state).offset = SHAKE128_RATE;
        if extracted < outlen {
            crypto_core_keccak1600_permute_24(
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
pub unsafe extern "C" fn _sodium_shake128_ref(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    let mut state = core::mem::MaybeUninit::<shake128_state_internal>::uninit();
    let state = state.as_mut_ptr();

    _sodium_shake128_ref_init(state);
    _sodium_shake128_ref_update(state, in_, inlen);
    _sodium_shake128_ref_squeeze(state, out, outlen);

    0
}
