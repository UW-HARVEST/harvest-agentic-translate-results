//! Dispatch wrappers from crypto_generichash/crypto_generichash.c.
//! Only the init/update/final symbols are owned by this package (P3); the rest
//! of crypto_generichash lives in the primitives package.
use crate::primitives::generichash::crypto_generichash_blake2b_state;

extern "C" {
    fn crypto_generichash_blake2b_init(
        state: *mut crypto_generichash_blake2b_state,
        key: *const u8,
        keylen: usize,
        outlen: usize,
    ) -> i32;
    fn crypto_generichash_blake2b_update(
        state: *mut crypto_generichash_blake2b_state,
        input: *const u8,
        inlen: u64,
    ) -> i32;
    fn crypto_generichash_blake2b_final(
        state: *mut crypto_generichash_blake2b_state,
        out: *mut u8,
        outlen: usize,
    ) -> i32;
}

// crypto_generichash_state == crypto_generichash_blake2b_state (opaque[384], align 64)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_init(
    state: *mut crypto_generichash_blake2b_state,
    key: *const u8,
    keylen: usize,
    outlen: usize,
) -> i32 {
    crypto_generichash_blake2b_init(state, key, keylen, outlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_update(
    state: *mut crypto_generichash_blake2b_state,
    input: *const u8,
    inlen: u64,
) -> i32 {
    crypto_generichash_blake2b_update(state, input, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_final(
    state: *mut crypto_generichash_blake2b_state,
    out: *mut u8,
    outlen: usize,
) -> i32 {
    crypto_generichash_blake2b_final(state, out, outlen)
}
