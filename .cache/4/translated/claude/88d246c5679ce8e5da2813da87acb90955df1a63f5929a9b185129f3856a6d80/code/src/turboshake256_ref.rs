//! Translation of `c_src/libsodium/crypto_xof/turboshake256/ref/turboshake256_ref.c`.
//!
//! `private/quirks.h` renames every external symbol of this file:
//!   `turboshake256_ref`                  -> `_sodium_turboshake256_ref`
//!   `turboshake256_ref_init`             -> `_sodium_turboshake256_ref_init`
//!   `turboshake256_ref_init_with_domain` -> `_sodium_turboshake256_ref_init_with_domain`
//!   `turboshake256_ref_update`           -> `_sodium_turboshake256_ref_update`
//!   `turboshake256_ref_squeeze`          -> `_sodium_turboshake256_ref_squeeze`
//!
//! `turboshake256_finalize()` is `static` and therefore stays private.
//!
//! The `crypto_core_keccak1600_*` entry points live in
//! `c_src/libsodium/crypto_core/keccak1600/keccak1600.c` (public API, not
//! renamed by quirks.h), so they are declared in a local `extern "C"` block.

use core::ffi::c_int;
use core::mem::MaybeUninit;

/* #define TURBOSHAKE256_RATE 136 */
const TURBOSHAKE256_RATE: usize = 136;

/* typedef enum { TURBOSHAKE256_PHASE_ABSORBING = 0, TURBOSHAKE256_PHASE_SQUEEZING = 1 } turboshake256_phase; */
const TURBOSHAKE256_PHASE_ABSORBING: u8 = 0;
const TURBOSHAKE256_PHASE_SQUEEZING: u8 = 1;

/* #define crypto_xof_turboshake256_DOMAIN_STANDARD 0x1FU */
const crypto_xof_turboshake256_DOMAIN_STANDARD: u8 = 0x1f;

/// ```c
/// typedef struct CRYPTO_ALIGN(16) crypto_core_keccak1600_state {
///     unsigned char opaque[224];
/// } crypto_core_keccak1600_state;
/// ```
/// (`#pragma pack(1)` is irrelevant here: the single member is a `char` array;
/// `CRYPTO_ALIGN(16)` still gives the struct 16-byte alignment. Verified with
/// gcc: `sizeof == 224`, `_Alignof == 16`.)
#[repr(C, align(16))]
pub struct crypto_core_keccak1600_state {
    pub opaque: [u8; 224],
}

/// ```c
/// typedef struct turboshake256_state_internal_ {
///     crypto_core_keccak1600_state state;
///     size_t                       offset;
///     uint8_t                      phase;
///     unsigned char                domain;
/// } turboshake256_state_internal;
/// ```
/// gcc layout: size 240, align 16, `offset` @224, `phase` @232, `domain` @233.
#[repr(C)]
pub struct turboshake256_state_internal {
    pub state: crypto_core_keccak1600_state,
    pub offset: usize,
    pub phase: u8,
    pub domain: u8,
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
    fn crypto_core_keccak1600_permute_12(state: *mut crypto_core_keccak1600_state);
}

/// `int turboshake256_ref_init_with_domain(turboshake256_state_internal *state, unsigned char domain)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_turboshake256_ref_init_with_domain(
    state: *mut turboshake256_state_internal,
    domain: u8,
) -> c_int {
    crypto_core_keccak1600_init(&raw mut (*state).state);
    (*state).offset = 0;
    (*state).phase = TURBOSHAKE256_PHASE_ABSORBING;
    (*state).domain = domain;

    0
}

/// `int turboshake256_ref_init(turboshake256_state_internal *state)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_turboshake256_ref_init(state: *mut turboshake256_state_internal) -> c_int {
    _sodium_turboshake256_ref_init_with_domain(state, crypto_xof_turboshake256_DOMAIN_STANDARD)
}

/// `int turboshake256_ref_update(turboshake256_state_internal *state, const unsigned char *in, size_t inlen)`
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
        crypto_core_keccak1600_permute_12(&raw mut (*state).state);
        (*state).phase = TURBOSHAKE256_PHASE_ABSORBING;
        (*state).offset = 0;
        ret = -1;
    }

    if (*state).offset == TURBOSHAKE256_RATE && inlen > 0 {
        crypto_core_keccak1600_permute_12(&raw mut (*state).state);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && inlen > 0 {
        chunk_size = TURBOSHAKE256_RATE - (*state).offset;
        if chunk_size > inlen {
            chunk_size = inlen;
        }
        crypto_core_keccak1600_xor_bytes(
            &raw mut (*state).state,
            in_,
            (*state).offset,
            chunk_size,
        );
        (*state).offset = (*state).offset.wrapping_add(chunk_size);
        consumed = chunk_size;
        if (*state).offset == TURBOSHAKE256_RATE && consumed < inlen {
            crypto_core_keccak1600_permute_12(&raw mut (*state).state);
            (*state).offset = 0;
        }
    }
    while inlen.wrapping_sub(consumed) >= TURBOSHAKE256_RATE {
        crypto_core_keccak1600_xor_bytes(
            &raw mut (*state).state,
            in_.add(consumed),
            0,
            TURBOSHAKE256_RATE,
        );
        consumed = consumed.wrapping_add(TURBOSHAKE256_RATE);
        (*state).offset = TURBOSHAKE256_RATE;
        if consumed < inlen {
            crypto_core_keccak1600_permute_12(&raw mut (*state).state);
            (*state).offset = 0;
        }
    }
    if consumed < inlen {
        chunk_size = inlen.wrapping_sub(consumed);
        crypto_core_keccak1600_xor_bytes(
            &raw mut (*state).state,
            in_.add(consumed),
            0,
            chunk_size,
        );
        (*state).offset = chunk_size;
    }

    ret
}

/// `static void turboshake256_finalize(turboshake256_state_internal *state)`
unsafe fn turboshake256_finalize(state: *mut turboshake256_state_internal) {
    let pad: u8;

    /* If the rate is exactly full, process that block before padding */
    if (*state).offset == TURBOSHAKE256_RATE {
        crypto_core_keccak1600_permute_12(&raw mut (*state).state);
        (*state).offset = 0;
    }

    /* Apply padding: domain byte at current position, 0x80 at last byte */
    if (*state).offset == TURBOSHAKE256_RATE - 1 {
        /* Special case: padding fits in one byte */
        pad = (*state).domain ^ 0x80;
        crypto_core_keccak1600_xor_bytes(&raw mut (*state).state, &raw const pad, (*state).offset, 1);
    } else {
        /* Normal case: domain and 0x80 at different positions */
        crypto_core_keccak1600_xor_bytes(
            &raw mut (*state).state,
            &raw const (*state).domain,
            (*state).offset,
            1,
        );
        pad = 0x80;
        crypto_core_keccak1600_xor_bytes(
            &raw mut (*state).state,
            &raw const pad,
            TURBOSHAKE256_RATE - 1,
            1,
        );
    }

    /* Final permutation (12 rounds for TurboSHAKE) */
    crypto_core_keccak1600_permute_12(&raw mut (*state).state);

    (*state).offset = 0;
    (*state).phase = TURBOSHAKE256_PHASE_SQUEEZING;
}

/// `int turboshake256_ref_squeeze(turboshake256_state_internal *state, unsigned char *out, size_t outlen)`
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
        crypto_core_keccak1600_permute_12(&raw mut (*state).state);
        (*state).offset = 0;
    }
    if (*state).offset != 0 && outlen > 0 {
        chunk_size = TURBOSHAKE256_RATE - (*state).offset;
        if chunk_size > outlen {
            chunk_size = outlen;
        }
        crypto_core_keccak1600_extract_bytes(
            &raw const (*state).state,
            out,
            (*state).offset,
            chunk_size,
        );
        (*state).offset = (*state).offset.wrapping_add(chunk_size);
        extracted = chunk_size;
        if (*state).offset == TURBOSHAKE256_RATE && extracted < outlen {
            crypto_core_keccak1600_permute_12(&raw mut (*state).state);
            (*state).offset = 0;
        }
    }
    while outlen.wrapping_sub(extracted) >= TURBOSHAKE256_RATE {
        crypto_core_keccak1600_extract_bytes(
            &raw const (*state).state,
            out.add(extracted),
            0,
            TURBOSHAKE256_RATE,
        );
        extracted = extracted.wrapping_add(TURBOSHAKE256_RATE);
        (*state).offset = TURBOSHAKE256_RATE;
        if extracted < outlen {
            crypto_core_keccak1600_permute_12(&raw mut (*state).state);
            (*state).offset = 0;
        }
    }
    if extracted < outlen {
        chunk_size = outlen.wrapping_sub(extracted);
        crypto_core_keccak1600_extract_bytes(
            &raw const (*state).state,
            out.add(extracted),
            0,
            chunk_size,
        );
        (*state).offset = chunk_size;
    }

    0
}

/// `int turboshake256_ref(unsigned char *out, size_t outlen, const unsigned char *in, size_t inlen)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_turboshake256_ref(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: usize,
) -> c_int {
    /* turboshake256_state_internal state; -- uninitialised, fully set up by init() */
    let mut state_storage: MaybeUninit<turboshake256_state_internal> = MaybeUninit::uninit();
    let state: *mut turboshake256_state_internal = state_storage.as_mut_ptr();

    _sodium_turboshake256_ref_init(state);
    _sodium_turboshake256_ref_update(state, in_, inlen);
    _sodium_turboshake256_ref_squeeze(state, out, outlen);

    0
}
