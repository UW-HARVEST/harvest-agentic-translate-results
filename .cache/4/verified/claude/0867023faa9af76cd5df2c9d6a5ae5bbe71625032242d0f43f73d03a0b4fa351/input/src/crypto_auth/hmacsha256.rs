//! Translation of `crypto_auth/hmacsha256/auth_hmacsha256.c`.

#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::{c_int, c_void};
use core::ptr::addr_of_mut;

use crate::common::memset;
use crate::crypto_verify::crypto_verify_32;
use crate::randombytes::randombytes_buf;
use crate::sodium::core::sodium_misuse;
use crate::sodium::utils::{sodium_memcmp, sodium_memzero};

/// `#define crypto_auth_hmacsha256_BYTES 32U`
pub const crypto_auth_hmacsha256_BYTES: usize = 32;

/// `#define crypto_auth_hmacsha256_KEYBYTES 32U`
pub const crypto_auth_hmacsha256_KEYBYTES: usize = 32;

/// ```c
/// typedef struct crypto_hash_sha256_state {
///     uint32_t state[8];
///     uint64_t count;
///     uint8_t  buf[64];
/// } crypto_hash_sha256_state;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct crypto_hash_sha256_state {
    pub state: [u32; 8],
    pub count: u64,
    pub buf: [u8; 64],
}

/// ```c
/// typedef struct crypto_auth_hmacsha256_state {
///     crypto_hash_sha256_state ictx;
///     crypto_hash_sha256_state octx;
/// } crypto_auth_hmacsha256_state;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct crypto_auth_hmacsha256_state {
    pub ictx: crypto_hash_sha256_state,
    pub octx: crypto_hash_sha256_state,
}

unsafe extern "C" {
    fn crypto_hash_sha256_init(state: *mut crypto_hash_sha256_state) -> c_int;
    fn crypto_hash_sha256_update(
        state: *mut crypto_hash_sha256_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha256_final(state: *mut crypto_hash_sha256_state, out: *mut u8) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_bytes() -> usize {
    crypto_auth_hmacsha256_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_keybytes() -> usize {
    crypto_auth_hmacsha256_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_statebytes() -> usize {
    core::mem::size_of::<crypto_auth_hmacsha256_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_auth_hmacsha256_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_init(
    state: *mut crypto_auth_hmacsha256_state,
    mut key: *const u8,
    mut keylen: usize,
) -> c_int {
    let mut pad: [u8; 64] = [0; 64];
    let mut khash: [u8; 32] = [0; 32];
    let mut i: usize;

    if keylen > 64 {
        crypto_hash_sha256_init(addr_of_mut!((*state).ictx));
        crypto_hash_sha256_update(addr_of_mut!((*state).ictx), key, keylen as u64);
        crypto_hash_sha256_final(addr_of_mut!((*state).ictx), khash.as_mut_ptr());
        key = khash.as_ptr();
        keylen = 32;
    } else if key.is_null() {
        if keylen > 0 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    }
    crypto_hash_sha256_init(addr_of_mut!((*state).ictx));
    memset(pad.as_mut_ptr(), 0x36, 64);
    i = 0;
    while i < keylen {
        pad[i] ^= *key.add(i);
        i += 1;
    }
    crypto_hash_sha256_update(addr_of_mut!((*state).ictx), pad.as_ptr(), 64);

    crypto_hash_sha256_init(addr_of_mut!((*state).octx));
    memset(pad.as_mut_ptr(), 0x5c, 64);
    i = 0;
    while i < keylen {
        pad[i] ^= *key.add(i);
        i += 1;
    }
    crypto_hash_sha256_update(addr_of_mut!((*state).octx), pad.as_ptr(), 64);

    sodium_memzero(pad.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&pad));
    sodium_memzero(
        khash.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&khash),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_update(
    state: *mut crypto_auth_hmacsha256_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    crypto_hash_sha256_update(addr_of_mut!((*state).ictx), in_, inlen);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_final(
    state: *mut crypto_auth_hmacsha256_state,
    out: *mut u8,
) -> c_int {
    let mut ihash: [u8; 32] = [0; 32];

    crypto_hash_sha256_final(addr_of_mut!((*state).ictx), ihash.as_mut_ptr());
    crypto_hash_sha256_update(addr_of_mut!((*state).octx), ihash.as_ptr(), 32);
    crypto_hash_sha256_final(addr_of_mut!((*state).octx), out);

    sodium_memzero(
        ihash.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&ihash),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut state: crypto_auth_hmacsha256_state = core::mem::zeroed();

    crypto_auth_hmacsha256_init(&mut state, k, crypto_auth_hmacsha256_KEYBYTES);
    crypto_auth_hmacsha256_update(&mut state, in_, inlen);
    crypto_auth_hmacsha256_final(&mut state, out);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_verify(
    h: *const u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut correct: [u8; 32] = [0; 32];

    crypto_auth_hmacsha256(correct.as_mut_ptr(), in_, inlen, k);

    crypto_verify_32(h, correct.as_ptr())
        | ((h == correct.as_ptr()) as c_int).wrapping_neg()
        | sodium_memcmp(
            correct.as_ptr() as *const c_void,
            h as *const c_void,
            32,
        )
}
