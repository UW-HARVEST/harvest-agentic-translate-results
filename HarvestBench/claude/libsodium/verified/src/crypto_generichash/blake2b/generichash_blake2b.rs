//! Translation of `crypto_generichash/blake2b/ref/generichash_blake2b.c`
//! plus the `crypto_generichash_blake2b_state` declaration from
//! `include/sodium/crypto_generichash_blake2b.h`.
//!
//! The reference library is built WITHOUT `-DNDEBUG`, so every `assert()` in
//! the C source is live. All of them except the one in
//! `crypto_generichash_blake2b_final` are unreachable, because the preceding
//! `outlen`/`keylen` range checks already returned `-1`; those are kept as
//! comments. The reachable one is reproduced with a `panic!` (the crate is
//! built with `panic = "abort"`, so it dies with SIGABRT just like `assert()`,
//! *without* running the `sodium_misuse()` handler).

use core::ffi::{c_int, c_void};

use super::blake2b_ref::{blake2b_state, BLAKE2B_KEYBYTES, BLAKE2B_OUTBYTES};

/// ```c
/// #pragma pack(push, 1)
/// typedef struct CRYPTO_ALIGN(64) crypto_generichash_blake2b_state {
///     unsigned char opaque[384];
/// } crypto_generichash_blake2b_state;
/// #pragma pack(pop)
/// ```
/// `sizeof == 384`, `_Alignof == 64`.
#[repr(C, align(64))]
pub struct crypto_generichash_blake2b_state {
    pub opaque: [u8; 384],
}

// Defined in blake2b-ref.c.
unsafe extern "C" {
    fn _sodium_blake2b(
        out: *mut u8,
        in_: *const c_void,
        key: *const c_void,
        outlen: u8,
        inlen: u64,
        keylen: u8,
    ) -> c_int;
    fn _sodium_blake2b_salt_personal(
        out: *mut u8,
        in_: *const c_void,
        key: *const c_void,
        outlen: u8,
        inlen: u64,
        keylen: u8,
        salt: *const c_void,
        personal: *const c_void,
    ) -> c_int;
    fn _sodium_blake2b_init(S: *mut blake2b_state, outlen: u8) -> c_int;
    fn _sodium_blake2b_init_salt_personal(
        S: *mut blake2b_state,
        outlen: u8,
        salt: *const c_void,
        personal: *const c_void,
    ) -> c_int;
    fn _sodium_blake2b_init_key(
        S: *mut blake2b_state,
        outlen: u8,
        key: *const c_void,
        keylen: u8,
    ) -> c_int;
    fn _sodium_blake2b_init_key_salt_personal(
        S: *mut blake2b_state,
        outlen: u8,
        key: *const c_void,
        keylen: u8,
        salt: *const c_void,
        personal: *const c_void,
    ) -> c_int;
    fn _sodium_blake2b_update(S: *mut blake2b_state, in_: *const u8, inlen: u64) -> c_int;
    fn _sodium_blake2b_final(S: *mut blake2b_state, out: *mut u8, outlen: u8) -> c_int;
    fn _sodium_blake2b_pick_best_implementation() -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
    key: *const u8,
    keylen: usize,
) -> c_int {
    /* `inlen > UINT64_MAX` can never hold for a 64-bit `unsigned long long`. */
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || keylen > BLAKE2B_KEYBYTES || inlen > u64::MAX {
        return -1;
    }
    /* assert(outlen <= UINT8_MAX); */
    /* assert(keylen <= UINT8_MAX); */

    unsafe {
        _sodium_blake2b(
            out,
            in_ as *const c_void,
            key as *const c_void,
            outlen as u8,
            inlen,
            keylen as u8,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_salt_personal(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
    key: *const u8,
    keylen: usize,
    salt: *const u8,
    personal: *const u8,
) -> c_int {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || keylen > BLAKE2B_KEYBYTES || inlen > u64::MAX {
        return -1;
    }
    /* assert(outlen <= UINT8_MAX); */
    /* assert(keylen <= UINT8_MAX); */

    unsafe {
        _sodium_blake2b_salt_personal(
            out,
            in_ as *const c_void,
            key as *const c_void,
            outlen as u8,
            inlen,
            keylen as u8,
            salt as *const c_void,
            personal as *const c_void,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_init(
    state: *mut crypto_generichash_blake2b_state,
    key: *const u8,
    keylen: usize,
    outlen: usize,
) -> c_int {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || keylen > BLAKE2B_KEYBYTES {
        return -1;
    }
    /* assert(outlen <= UINT8_MAX); */
    /* assert(keylen <= UINT8_MAX); */
    /* COMPILER_ASSERT(sizeof(blake2b_state) <= sizeof *state); */
    const _: () = assert!(
        core::mem::size_of::<blake2b_state>()
            <= core::mem::size_of::<crypto_generichash_blake2b_state>()
    );
    if key.is_null() || keylen == 0 {
        if unsafe { _sodium_blake2b_init(state as *mut blake2b_state, outlen as u8) } != 0 {
            return -1; /* LCOV_EXCL_LINE */
        }
    } else if unsafe {
        _sodium_blake2b_init_key(
            state as *mut blake2b_state,
            outlen as u8,
            key as *const c_void,
            keylen as u8,
        )
    } != 0
    {
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_init_salt_personal(
    state: *mut crypto_generichash_blake2b_state,
    key: *const u8,
    keylen: usize,
    outlen: usize,
    salt: *const u8,
    personal: *const u8,
) -> c_int {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || keylen > BLAKE2B_KEYBYTES {
        return -1;
    }
    /* assert(outlen <= UINT8_MAX); */
    /* assert(keylen <= UINT8_MAX); */
    if key.is_null() || keylen == 0 {
        if unsafe {
            _sodium_blake2b_init_salt_personal(
                state as *mut blake2b_state,
                outlen as u8,
                salt as *const c_void,
                personal as *const c_void,
            )
        } != 0
        {
            return -1; /* LCOV_EXCL_LINE */
        }
    } else if unsafe {
        _sodium_blake2b_init_key_salt_personal(
            state as *mut blake2b_state,
            outlen as u8,
            key as *const c_void,
            keylen as u8,
            salt as *const c_void,
            personal as *const c_void,
        )
    } != 0
    {
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_update(
    state: *mut crypto_generichash_blake2b_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    unsafe { _sodium_blake2b_update(state as *mut blake2b_state, in_, inlen) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_final(
    state: *mut crypto_generichash_blake2b_state,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    /* `assert(outlen <= UINT8_MAX);` — the reference `.so` is built WITHOUT
     * `NDEBUG`, so this assertion is live and `outlen >= 256` dies with a raw
     * `abort()` (SIGABRT) that does NOT go through the `sodium_misuse()`
     * handler. `panic!` + `panic = "abort"` reproduces exactly that. */
    if outlen > u8::MAX as usize {
        assert_failed_outlen();
    }
    unsafe { _sodium_blake2b_final(state as *mut blake2b_state, out, outlen as u8) }
}

/// `assert(outlen <= UINT8_MAX)` failing in `crypto_generichash_blake2b_final`.
#[cold]
#[inline(never)]
fn assert_failed_outlen() -> ! {
    panic!(
        "crypto_generichash_blake2b_final: Assertion `outlen <= UINT8_MAX' failed."
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_generichash_blake2b_pick_best_implementation() -> c_int {
    unsafe { _sodium_blake2b_pick_best_implementation() }
}
