//! Translation of crypto_auth/hmacsha512/auth_hmacsha512.c
//! and include/sodium/crypto_auth_hmacsha512.h

use core::ffi::{c_int, c_void};

use crate::crypto_hash::sha512::{
    crypto_hash_sha512_final, crypto_hash_sha512_init, crypto_hash_sha512_state,
    crypto_hash_sha512_update,
};
use crate::crypto_verify::crypto_verify_64;
use crate::randombytes::randombytes_buf;
use crate::sodium_core::sodium_misuse;
use crate::sodium_utils::{sodium_memcmp, sodium_memzero};

pub const crypto_auth_hmacsha512_BYTES: usize = 64;
pub const crypto_auth_hmacsha512_KEYBYTES: usize = 32;

#[repr(C)]
pub struct crypto_auth_hmacsha512_state {
    pub ictx: crypto_hash_sha512_state,
    pub octx: crypto_hash_sha512_state,
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_hmacsha512_bytes() -> usize {
    crypto_auth_hmacsha512_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_hmacsha512_keybytes() -> usize {
    crypto_auth_hmacsha512_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_hmacsha512_statebytes() -> usize {
    core::mem::size_of::<crypto_auth_hmacsha512_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_auth_hmacsha512_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_init(
    state: *mut crypto_auth_hmacsha512_state,
    mut key: *const u8,
    mut keylen: usize,
) -> c_int {
    let mut pad: [u8; 128] = [0; 128];
    let mut khash: [u8; 64] = [0; 64];
    let mut i: usize;

    if keylen > 128 {
        crypto_hash_sha512_init(&mut (*state).ictx);
        crypto_hash_sha512_update(&mut (*state).ictx, key, keylen as u64);
        crypto_hash_sha512_final(&mut (*state).ictx, khash.as_mut_ptr());
        key = khash.as_ptr();
        keylen = 64;
    } else if key.is_null() {
        if keylen > 0 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    }
    crypto_hash_sha512_init(&mut (*state).ictx);
    core::ptr::write_bytes(pad.as_mut_ptr(), 0x36, 128);
    i = 0;
    while i < keylen {
        pad[i] ^= *key.add(i);
        i += 1;
    }
    crypto_hash_sha512_update(&mut (*state).ictx, pad.as_ptr(), 128);

    crypto_hash_sha512_init(&mut (*state).octx);
    core::ptr::write_bytes(pad.as_mut_ptr(), 0x5c, 128);
    i = 0;
    while i < keylen {
        pad[i] ^= *key.add(i);
        i += 1;
    }
    crypto_hash_sha512_update(&mut (*state).octx, pad.as_ptr(), 128);

    sodium_memzero(pad.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&pad));
    sodium_memzero(khash.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&khash));

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_update(
    state: *mut crypto_auth_hmacsha512_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    crypto_hash_sha512_update(&mut (*state).ictx, in_, inlen);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_final(
    state: *mut crypto_auth_hmacsha512_state,
    out: *mut u8,
) -> c_int {
    let mut ihash: [u8; 64] = [0; 64];

    crypto_hash_sha512_final(&mut (*state).ictx, ihash.as_mut_ptr());
    crypto_hash_sha512_update(&mut (*state).octx, ihash.as_ptr(), 64);
    crypto_hash_sha512_final(&mut (*state).octx, out);

    sodium_memzero(ihash.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&ihash));

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut state: crypto_auth_hmacsha512_state = core::mem::zeroed();

    crypto_auth_hmacsha512_init(&mut state, k, crypto_auth_hmacsha512_KEYBYTES);
    crypto_auth_hmacsha512_update(&mut state, in_, inlen);
    crypto_auth_hmacsha512_final(&mut state, out);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_verify(
    h: *const u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut correct: [u8; 64] = [0; 64];

    crypto_auth_hmacsha512(correct.as_mut_ptr(), in_, inlen, k);

    crypto_verify_64(h, correct.as_ptr())
        | (-((h == correct.as_ptr()) as c_int))
        | sodium_memcmp(correct.as_ptr() as *const c_void, h as *const c_void, 64)
}
