//! Translation of `crypto_auth/hmacsha512256/auth_hmacsha512256.c`.

#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::{c_int, c_void};

use crate::common::memcpy;
use crate::crypto_verify::crypto_verify_32;
use crate::randombytes::randombytes_buf;
use crate::sodium::utils::{sodium_memcmp, sodium_memzero};

use super::hmacsha512::crypto_auth_hmacsha512_state;

/// `#define crypto_auth_hmacsha512256_BYTES 32U`
pub const crypto_auth_hmacsha512256_BYTES: usize = 32;

/// `#define crypto_auth_hmacsha512256_KEYBYTES 32U`
pub const crypto_auth_hmacsha512256_KEYBYTES: usize = 32;

/// `typedef crypto_auth_hmacsha512_state crypto_auth_hmacsha512256_state;`
pub type crypto_auth_hmacsha512256_state = crypto_auth_hmacsha512_state;

unsafe extern "C" {
    fn crypto_auth_hmacsha512_init(
        state: *mut crypto_auth_hmacsha512_state,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    fn crypto_auth_hmacsha512_update(
        state: *mut crypto_auth_hmacsha512_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_auth_hmacsha512_final(
        state: *mut crypto_auth_hmacsha512_state,
        out: *mut u8,
    ) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_bytes() -> usize {
    crypto_auth_hmacsha512256_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_keybytes() -> usize {
    crypto_auth_hmacsha512256_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_statebytes() -> usize {
    core::mem::size_of::<crypto_auth_hmacsha512256_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_auth_hmacsha512256_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_init(
    state: *mut crypto_auth_hmacsha512256_state,
    key: *const u8,
    keylen: usize,
) -> c_int {
    crypto_auth_hmacsha512_init(state as *mut crypto_auth_hmacsha512_state, key, keylen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_update(
    state: *mut crypto_auth_hmacsha512256_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    crypto_auth_hmacsha512_update(state as *mut crypto_auth_hmacsha512_state, in_, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_final(
    state: *mut crypto_auth_hmacsha512256_state,
    out: *mut u8,
) -> c_int {
    let mut out0: [u8; 64] = [0; 64];

    crypto_auth_hmacsha512_final(
        state as *mut crypto_auth_hmacsha512_state,
        out0.as_mut_ptr(),
    );
    memcpy(out, out0.as_ptr(), 32);
    sodium_memzero(
        out0.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&out0),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut state: crypto_auth_hmacsha512256_state = core::mem::zeroed();

    crypto_auth_hmacsha512256_init(&mut state, k, crypto_auth_hmacsha512256_KEYBYTES);
    crypto_auth_hmacsha512256_update(&mut state, in_, inlen);
    crypto_auth_hmacsha512256_final(&mut state, out);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_verify(
    h: *const u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut correct: [u8; 32] = [0; 32];

    crypto_auth_hmacsha512256(correct.as_mut_ptr(), in_, inlen, k);

    crypto_verify_32(h, correct.as_ptr())
        | ((h == correct.as_ptr()) as c_int).wrapping_neg()
        | sodium_memcmp(
            correct.as_ptr() as *const c_void,
            h as *const c_void,
            32,
        )
}
