//! Translated from crypto_generichash/blake2b/{generichash_blake2.c, ref/generichash_blake2b.c}
//! and crypto_generichash/crypto_generichash.c
use crate::primitives::blake2b::*;
use crate::primitives::cutil::*;
use core::ffi::{c_char, c_void};

// crypto_generichash_blake2b_state = opaque[384], align 64
#[repr(C, align(64))]
pub struct crypto_generichash_blake2b_state {
    pub opaque: [u8; 384],
}

const BLAKE2B_OUTBYTES_L: usize = 64;
const BLAKE2B_KEYBYTES_L: usize = 64;

// ---- generichash_blake2.c: constant accessors ----
#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_bytes_min() -> usize {
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_bytes_max() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_bytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_keybytes_min() -> usize {
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_keybytes_max() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_saltbytes() -> usize {
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_personalbytes() -> usize {
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_statebytes() -> usize {
    (core::mem::size_of::<crypto_generichash_blake2b_state>() + 63) & !63
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}

// ---- ref/generichash_blake2b.c ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b(
    out: *mut u8,
    outlen: usize,
    input: *const u8,
    inlen: u64,
    key: *const u8,
    keylen: usize,
) -> i32 {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES_L || keylen > BLAKE2B_KEYBYTES_L {
        return -1;
    }
    _sodium_blake2b(
        out,
        input as *const c_void,
        key as *const c_void,
        outlen as u8,
        inlen,
        keylen as u8,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_salt_personal(
    out: *mut u8,
    outlen: usize,
    input: *const u8,
    inlen: u64,
    key: *const u8,
    keylen: usize,
    salt: *const u8,
    personal: *const u8,
) -> i32 {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES_L || keylen > BLAKE2B_KEYBYTES_L {
        return -1;
    }
    _sodium_blake2b_salt_personal(
        out,
        input as *const c_void,
        key as *const c_void,
        outlen as u8,
        inlen,
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
) -> i32 {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES_L || keylen > BLAKE2B_KEYBYTES_L {
        return -1;
    }
    let bs = state as *mut blake2b_state;
    if key.is_null() || keylen == 0 {
        if _sodium_blake2b_init(bs, outlen as u8) != 0 {
            return -1;
        }
    } else if _sodium_blake2b_init_key(bs, outlen as u8, key as *const c_void, keylen as u8) != 0 {
        return -1;
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
) -> i32 {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES_L || keylen > BLAKE2B_KEYBYTES_L {
        return -1;
    }
    let bs = state as *mut blake2b_state;
    if key.is_null() || keylen == 0 {
        if _sodium_blake2b_init_salt_personal(
            bs,
            outlen as u8,
            salt as *const c_void,
            personal as *const c_void,
        ) != 0
        {
            return -1;
        }
    } else if _sodium_blake2b_init_key_salt_personal(
        bs,
        outlen as u8,
        key as *const c_void,
        keylen as u8,
        salt as *const c_void,
        personal as *const c_void,
    ) != 0
    {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_update(
    state: *mut crypto_generichash_blake2b_state,
    input: *const u8,
    inlen: u64,
) -> i32 {
    _sodium_blake2b_update(state as *mut blake2b_state, input, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_final(
    state: *mut crypto_generichash_blake2b_state,
    out: *mut u8,
    outlen: usize,
) -> i32 {
    _sodium_blake2b_final(state as *mut blake2b_state, out, outlen as u8)
}

#[unsafe(no_mangle)]
pub extern "C" fn _crypto_generichash_blake2b_pick_best_implementation() -> i32 {
    _sodium_blake2b_pick_best_implementation()
}

// ---- crypto_generichash.c (dispatch) ----

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_bytes_min() -> usize {
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_bytes_max() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_bytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_keybytes_min() -> usize {
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_keybytes_max() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_keybytes() -> usize {
    32
}

static GENERICHASH_PRIMITIVE: &[u8] = b"blake2b\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_primitive() -> *const c_char {
    GENERICHASH_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_statebytes() -> usize {
    (core::mem::size_of::<crypto_generichash_blake2b_state>() + 63) & !63
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash(
    out: *mut u8,
    outlen: usize,
    input: *const u8,
    inlen: u64,
    key: *const u8,
    keylen: usize,
) -> i32 {
    crypto_generichash_blake2b(out, outlen, input, inlen, key, keylen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}
