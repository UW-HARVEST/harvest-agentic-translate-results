//! Translation of `crypto_generichash/blake2b/ref/generichash_blake2b.c`.
//!
//! Exports:
//!   * `_crypto_generichash_blake2b_pick_best_implementation`
//!   * `crypto_generichash_blake2b`
//!   * `crypto_generichash_blake2b_final`
//!   * `crypto_generichash_blake2b_init`
//!   * `crypto_generichash_blake2b_init_salt_personal`
//!   * `crypto_generichash_blake2b_salt_personal`
//!   * `crypto_generichash_blake2b_update`

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/* enum blake2b_constant (blake2.h) */
const BLAKE2B_OUTBYTES: usize = 64;
const BLAKE2B_KEYBYTES: usize = 64;

/* `crypto_generichash_blake2b_state` from crypto_generichash_blake2b.h:
 * `typedef struct CRYPTO_ALIGN(64) { unsigned char opaque[384]; }` declared
 * inside `#pragma pack(push, 1)`.  sizeof == 384, _Alignof == 64. */
#[repr(C, align(64))]
pub struct crypto_generichash_blake2b_state {
    pub opaque: [u8; 384],
}

/* COMPILER_ASSERT(sizeof(blake2b_state) <= sizeof *state); sizeof(blake2b_state) == 361 */
const _: () = assert!(361 <= core::mem::size_of::<crypto_generichash_blake2b_state>());

extern "C" {
    /* blake2b-ref.c (names after private/quirks.h renaming) */
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
    fn _sodium_blake2b_init(S: *mut c_void, outlen: u8) -> c_int;
    fn _sodium_blake2b_init_salt_personal(
        S: *mut c_void,
        outlen: u8,
        salt: *const c_void,
        personal: *const c_void,
    ) -> c_int;
    fn _sodium_blake2b_init_key(
        S: *mut c_void,
        outlen: u8,
        key: *const c_void,
        keylen: u8,
    ) -> c_int;
    fn _sodium_blake2b_init_key_salt_personal(
        S: *mut c_void,
        outlen: u8,
        key: *const c_void,
        keylen: u8,
        salt: *const c_void,
        personal: *const c_void,
    ) -> c_int;
    fn _sodium_blake2b_update(S: *mut c_void, in_: *const u8, inlen: u64) -> c_int;
    fn _sodium_blake2b_final(S: *mut c_void, out: *mut u8, outlen: u8) -> c_int;
    fn _sodium_blake2b_pick_best_implementation() -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: c_ulonglong,
    key: *const u8,
    keylen: usize,
) -> c_int {
    if outlen == 0
        || outlen > BLAKE2B_OUTBYTES
        || keylen > BLAKE2B_KEYBYTES
        || inlen > u64::MAX as c_ulonglong
    {
        return -1;
    }
    assert!(outlen <= u8::MAX as usize);
    assert!(keylen <= u8::MAX as usize);

    _sodium_blake2b(
        out,
        in_ as *const c_void,
        key as *const c_void,
        outlen as u8,
        inlen as u64,
        keylen as u8,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_salt_personal(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: c_ulonglong,
    key: *const u8,
    keylen: usize,
    salt: *const u8,
    personal: *const u8,
) -> c_int {
    if outlen == 0
        || outlen > BLAKE2B_OUTBYTES
        || keylen > BLAKE2B_KEYBYTES
        || inlen > u64::MAX as c_ulonglong
    {
        return -1;
    }
    assert!(outlen <= u8::MAX as usize);
    assert!(keylen <= u8::MAX as usize);

    _sodium_blake2b_salt_personal(
        out,
        in_ as *const c_void,
        key as *const c_void,
        outlen as u8,
        inlen as u64,
        keylen as u8,
        salt as *const c_void,
        personal as *const c_void,
    )
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
    assert!(outlen <= u8::MAX as usize);
    assert!(keylen <= u8::MAX as usize);
    if key.is_null() || keylen == 0 {
        if _sodium_blake2b_init(state as *mut c_void, outlen as u8) != 0 {
            return -1; /* LCOV_EXCL_LINE */
        }
    } else if _sodium_blake2b_init_key(
        state as *mut c_void,
        outlen as u8,
        key as *const c_void,
        keylen as u8,
    ) != 0
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
    assert!(outlen <= u8::MAX as usize);
    assert!(keylen <= u8::MAX as usize);
    if key.is_null() || keylen == 0 {
        if _sodium_blake2b_init_salt_personal(
            state as *mut c_void,
            outlen as u8,
            salt as *const c_void,
            personal as *const c_void,
        ) != 0
        {
            return -1; /* LCOV_EXCL_LINE */
        }
    } else if _sodium_blake2b_init_key_salt_personal(
        state as *mut c_void,
        outlen as u8,
        key as *const c_void,
        keylen as u8,
        salt as *const c_void,
        personal as *const c_void,
    ) != 0
    {
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_update(
    state: *mut crypto_generichash_blake2b_state,
    in_: *const u8,
    inlen: c_ulonglong,
) -> c_int {
    _sodium_blake2b_update(state as *mut c_void, in_, inlen as u64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_final(
    state: *mut crypto_generichash_blake2b_state,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    assert!(outlen <= u8::MAX as usize);
    _sodium_blake2b_final(state as *mut c_void, out, outlen as u8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_generichash_blake2b_pick_best_implementation() -> c_int {
    _sodium_blake2b_pick_best_implementation()
}
