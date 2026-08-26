//! Translation of `crypto_auth/hmacsha512256/auth_hmacsha512256.c`.
//!
//! Nothing in this file is affected by `private/quirks.h`, so every exported
//! function keeps its plain C name.  `crypto_auth_hmacsha512256_state` is a
//! typedef of `crypto_auth_hmacsha512_state`, so the same layout is used here.

use crate::common::memcpy;
use core::ffi::{c_int, c_ulonglong, c_void};
use core::ptr::addr_of_mut;

/* crypto_hash_sha512.h */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_hash_sha512_state {
    pub state: [u64; 8],
    pub count: [u64; 2],
    pub buf: [u8; 128],
}

/* crypto_auth_hmacsha512.h */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_auth_hmacsha512_state {
    pub ictx: crypto_hash_sha512_state,
    pub octx: crypto_hash_sha512_state,
}

/* typedef crypto_auth_hmacsha512_state crypto_auth_hmacsha512256_state; */
pub type crypto_auth_hmacsha512256_state = crypto_auth_hmacsha512_state;

pub const crypto_auth_hmacsha512256_BYTES: usize = 32;
pub const crypto_auth_hmacsha512256_KEYBYTES: usize = 32;

extern "C" {
    fn crypto_auth_hmacsha512_init(
        state: *mut crypto_auth_hmacsha512_state,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    fn crypto_auth_hmacsha512_update(
        state: *mut crypto_auth_hmacsha512_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_auth_hmacsha512_final(
        state: *mut crypto_auth_hmacsha512_state,
        out: *mut u8,
    ) -> c_int;

    fn crypto_verify_32(x: *const u8, y: *const u8) -> c_int;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1_: *const c_void, b2_: *const c_void, len: usize) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

/* size_t crypto_auth_hmacsha512256_bytes(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_bytes() -> usize {
    crypto_auth_hmacsha512256_BYTES
}

/* size_t crypto_auth_hmacsha512256_keybytes(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_keybytes() -> usize {
    crypto_auth_hmacsha512256_KEYBYTES
}

/* size_t crypto_auth_hmacsha512256_statebytes(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_statebytes() -> usize {
    core::mem::size_of::<crypto_auth_hmacsha512256_state>()
}

/* void crypto_auth_hmacsha512256_keygen(unsigned char k[32]) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_auth_hmacsha512256_KEYBYTES);
}

/* int crypto_auth_hmacsha512256_init(crypto_auth_hmacsha512256_state *state,
                                      const unsigned char *key, size_t keylen) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_init(
    state: *mut crypto_auth_hmacsha512256_state,
    key: *const u8,
    keylen: usize,
) -> c_int {
    crypto_auth_hmacsha512_init(state as *mut crypto_auth_hmacsha512_state, key, keylen)
}

/* int crypto_auth_hmacsha512256_update(crypto_auth_hmacsha512256_state *state,
                                        const unsigned char *in,
                                        unsigned long long inlen) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_update(
    state: *mut crypto_auth_hmacsha512256_state,
    in_: *const u8,
    inlen: c_ulonglong,
) -> c_int {
    crypto_auth_hmacsha512_update(state as *mut crypto_auth_hmacsha512_state, in_, inlen)
}

/* int crypto_auth_hmacsha512256_final(crypto_auth_hmacsha512256_state *state,
                                       unsigned char *out) */
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

/* int crypto_auth_hmacsha512256(unsigned char *out, const unsigned char *in,
                                 unsigned long long inlen, const unsigned char *k) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256(
    out: *mut u8,
    in_: *const u8,
    inlen: c_ulonglong,
    k: *const u8,
) -> c_int {
    let mut state_storage = core::mem::MaybeUninit::<crypto_auth_hmacsha512256_state>::uninit();
    let state: *mut crypto_auth_hmacsha512256_state = state_storage.as_mut_ptr();

    crypto_auth_hmacsha512256_init(state, k, crypto_auth_hmacsha512256_KEYBYTES);
    crypto_auth_hmacsha512256_update(state, in_, inlen);
    crypto_auth_hmacsha512256_final(state, out);

    0
}

/* int crypto_auth_hmacsha512256_verify(const unsigned char *h,
                                        const unsigned char *in,
                                        unsigned long long inlen,
                                        const unsigned char *k) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_verify(
    h: *const u8,
    in_: *const u8,
    inlen: c_ulonglong,
    k: *const u8,
) -> c_int {
    let mut correct: [u8; 32] = [0; 32];

    crypto_auth_hmacsha512256(correct.as_mut_ptr(), in_, inlen, k);

    let a = crypto_verify_32(h, correct.as_ptr());
    let b = -((h == correct.as_ptr() as *const u8) as c_int);
    let c = sodium_memcmp(
        correct.as_ptr() as *const c_void,
        h as *const c_void,
        32,
    );

    a | b | c
}
