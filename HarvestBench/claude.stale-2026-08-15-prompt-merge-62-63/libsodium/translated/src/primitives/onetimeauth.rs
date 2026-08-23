//! Translated from crypto_onetimeauth/crypto_onetimeauth.c
use crate::primitives::cutil::*;
use crate::primitives::poly1305::crypto_onetimeauth_poly1305_state;
use core::ffi::{c_char, c_void};

extern "C" {
    fn crypto_onetimeauth_poly1305(out: *mut u8, input: *const u8, inlen: u64, k: *const u8)
        -> i32;
    fn crypto_onetimeauth_poly1305_verify(
        h: *const u8,
        input: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> i32;
    fn crypto_onetimeauth_poly1305_init(
        state: *mut crypto_onetimeauth_poly1305_state,
        key: *const u8,
    ) -> i32;
    fn crypto_onetimeauth_poly1305_update(
        state: *mut crypto_onetimeauth_poly1305_state,
        input: *const u8,
        inlen: u64,
    ) -> i32;
    fn crypto_onetimeauth_poly1305_final(
        state: *mut crypto_onetimeauth_poly1305_state,
        out: *mut u8,
    ) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_onetimeauth_statebytes() -> usize {
    // crypto_onetimeauth_state == crypto_onetimeauth_poly1305_state (opaque[256])
    core::mem::size_of::<crypto_onetimeauth_poly1305_state>()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_onetimeauth_bytes() -> usize {
    16
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_onetimeauth_keybytes() -> usize {
    32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth(
    out: *mut u8,
    input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    crypto_onetimeauth_poly1305(out, input, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_verify(
    h: *const u8,
    input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    crypto_onetimeauth_poly1305_verify(h, input, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_init(
    state: *mut crypto_onetimeauth_poly1305_state,
    key: *const u8,
) -> i32 {
    crypto_onetimeauth_poly1305_init(state, key)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_update(
    state: *mut crypto_onetimeauth_poly1305_state,
    input: *const u8,
    inlen: u64,
) -> i32 {
    crypto_onetimeauth_poly1305_update(state, input, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_final(
    state: *mut crypto_onetimeauth_poly1305_state,
    out: *mut u8,
) -> i32 {
    crypto_onetimeauth_poly1305_final(state, out)
}

static ONETIMEAUTH_PRIMITIVE: &[u8] = b"poly1305\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_onetimeauth_primitive() -> *const c_char {
    ONETIMEAUTH_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}
