//! Translated from crypto_auth/{crypto_auth.c, hmacsha256/auth_hmacsha256.c,
//! hmacsha512/auth_hmacsha512.c, hmacsha512256/auth_hmacsha512256.c}
use crate::primitives::sha256::crypto_hash_sha256_state;
use crate::primitives::sha512::crypto_hash_sha512_state;
use core::ffi::{c_char, c_void};

extern "C" {
    fn crypto_hash_sha256_init(state: *mut crypto_hash_sha256_state) -> i32;
    fn crypto_hash_sha256_update(state: *mut crypto_hash_sha256_state, input: *const u8, inlen: u64) -> i32;
    fn crypto_hash_sha256_final(state: *mut crypto_hash_sha256_state, out: *mut u8) -> i32;
    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> i32;
    fn crypto_hash_sha512_update(state: *mut crypto_hash_sha512_state, input: *const u8, inlen: u64) -> i32;
    fn crypto_hash_sha512_final(state: *mut crypto_hash_sha512_state, out: *mut u8) -> i32;
    fn crypto_verify_32(x: *const u8, y: *const u8) -> i32;
    fn crypto_verify_64(x: *const u8, y: *const u8) -> i32;
    fn sodium_memcmp(b1: *const c_void, b2: *const c_void, len: usize) -> i32;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_misuse() -> !;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

// ==== hmacsha256 ====

#[repr(C)]
pub struct crypto_auth_hmacsha256_state {
    pub ictx: crypto_hash_sha256_state,
    pub octx: crypto_hash_sha256_state,
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_hmacsha256_bytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_hmacsha256_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_hmacsha256_statebytes() -> usize {
    core::mem::size_of::<crypto_auth_hmacsha256_state>()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_init(
    state: *mut crypto_auth_hmacsha256_state,
    mut key: *const u8,
    mut keylen: usize,
) -> i32 {
    let mut pad = [0u8; 64];
    let mut khash = [0u8; 32];

    if keylen > 64 {
        crypto_hash_sha256_init(&mut (*state).ictx);
        crypto_hash_sha256_update(&mut (*state).ictx, key, keylen as u64);
        crypto_hash_sha256_final(&mut (*state).ictx, khash.as_mut_ptr());
        key = khash.as_ptr();
        keylen = 32;
    } else if key.is_null() {
        if keylen > 0 {
            sodium_misuse();
        }
    }
    crypto_hash_sha256_init(&mut (*state).ictx);
    pad = [0x36u8; 64];
    for i in 0..keylen {
        pad[i] ^= *key.add(i);
    }
    crypto_hash_sha256_update(&mut (*state).ictx, pad.as_ptr(), 64);

    crypto_hash_sha256_init(&mut (*state).octx);
    pad = [0x5cu8; 64];
    for i in 0..keylen {
        pad[i] ^= *key.add(i);
    }
    crypto_hash_sha256_update(&mut (*state).octx, pad.as_ptr(), 64);

    sodium_memzero(pad.as_mut_ptr() as *mut c_void, 64);
    sodium_memzero(khash.as_mut_ptr() as *mut c_void, 32);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_update(
    state: *mut crypto_auth_hmacsha256_state,
    input: *const u8,
    inlen: u64,
) -> i32 {
    crypto_hash_sha256_update(&mut (*state).ictx, input, inlen);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_final(
    state: *mut crypto_auth_hmacsha256_state,
    out: *mut u8,
) -> i32 {
    let mut ihash = [0u8; 32];
    crypto_hash_sha256_final(&mut (*state).ictx, ihash.as_mut_ptr());
    crypto_hash_sha256_update(&mut (*state).octx, ihash.as_ptr(), 32);
    crypto_hash_sha256_final(&mut (*state).octx, out);
    sodium_memzero(ihash.as_mut_ptr() as *mut c_void, 32);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256(
    out: *mut u8,
    input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    let mut state = core::mem::MaybeUninit::<crypto_auth_hmacsha256_state>::uninit();
    crypto_auth_hmacsha256_init(state.as_mut_ptr(), k, 32);
    crypto_auth_hmacsha256_update(state.as_mut_ptr(), input, inlen);
    crypto_auth_hmacsha256_final(state.as_mut_ptr(), out);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha256_verify(
    h: *const u8,
    input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    let mut correct = [0u8; 32];
    crypto_auth_hmacsha256(correct.as_mut_ptr(), input, inlen, k);
    crypto_verify_32(h, correct.as_ptr())
        | (-((h == correct.as_ptr()) as i32))
        | sodium_memcmp(correct.as_ptr() as *const c_void, h as *const c_void, 32)
}

// ==== hmacsha512 ====

#[repr(C)]
pub struct crypto_auth_hmacsha512_state {
    pub ictx: crypto_hash_sha512_state,
    pub octx: crypto_hash_sha512_state,
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_hmacsha512_bytes() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_hmacsha512_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_hmacsha512_statebytes() -> usize {
    core::mem::size_of::<crypto_auth_hmacsha512_state>()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_init(
    state: *mut crypto_auth_hmacsha512_state,
    mut key: *const u8,
    mut keylen: usize,
) -> i32 {
    let mut pad = [0u8; 128];
    let mut khash = [0u8; 64];

    if keylen > 128 {
        crypto_hash_sha512_init(&mut (*state).ictx);
        crypto_hash_sha512_update(&mut (*state).ictx, key, keylen as u64);
        crypto_hash_sha512_final(&mut (*state).ictx, khash.as_mut_ptr());
        key = khash.as_ptr();
        keylen = 64;
    } else if key.is_null() {
        if keylen > 0 {
            sodium_misuse();
        }
    }
    crypto_hash_sha512_init(&mut (*state).ictx);
    pad = [0x36u8; 128];
    for i in 0..keylen {
        pad[i] ^= *key.add(i);
    }
    crypto_hash_sha512_update(&mut (*state).ictx, pad.as_ptr(), 128);

    crypto_hash_sha512_init(&mut (*state).octx);
    pad = [0x5cu8; 128];
    for i in 0..keylen {
        pad[i] ^= *key.add(i);
    }
    crypto_hash_sha512_update(&mut (*state).octx, pad.as_ptr(), 128);

    sodium_memzero(pad.as_mut_ptr() as *mut c_void, 128);
    sodium_memzero(khash.as_mut_ptr() as *mut c_void, 64);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_update(
    state: *mut crypto_auth_hmacsha512_state,
    input: *const u8,
    inlen: u64,
) -> i32 {
    crypto_hash_sha512_update(&mut (*state).ictx, input, inlen);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_final(
    state: *mut crypto_auth_hmacsha512_state,
    out: *mut u8,
) -> i32 {
    let mut ihash = [0u8; 64];
    crypto_hash_sha512_final(&mut (*state).ictx, ihash.as_mut_ptr());
    crypto_hash_sha512_update(&mut (*state).octx, ihash.as_ptr(), 64);
    crypto_hash_sha512_final(&mut (*state).octx, out);
    sodium_memzero(ihash.as_mut_ptr() as *mut c_void, 64);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512(
    out: *mut u8,
    input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    let mut state = core::mem::MaybeUninit::<crypto_auth_hmacsha512_state>::uninit();
    crypto_auth_hmacsha512_init(state.as_mut_ptr(), k, 32);
    crypto_auth_hmacsha512_update(state.as_mut_ptr(), input, inlen);
    crypto_auth_hmacsha512_final(state.as_mut_ptr(), out);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_verify(
    h: *const u8,
    input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    let mut correct = [0u8; 64];
    crypto_auth_hmacsha512(correct.as_mut_ptr(), input, inlen, k);
    crypto_verify_64(h, correct.as_ptr())
        | (-((h == correct.as_ptr()) as i32))
        | sodium_memcmp(correct.as_ptr() as *const c_void, h as *const c_void, 64)
}

// ==== hmacsha512256 (state == hmacsha512_state) ====

#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_hmacsha512256_bytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_hmacsha512256_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_hmacsha512256_statebytes() -> usize {
    core::mem::size_of::<crypto_auth_hmacsha512_state>()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_init(
    state: *mut crypto_auth_hmacsha512_state,
    key: *const u8,
    keylen: usize,
) -> i32 {
    crypto_auth_hmacsha512_init(state, key, keylen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_update(
    state: *mut crypto_auth_hmacsha512_state,
    input: *const u8,
    inlen: u64,
) -> i32 {
    crypto_auth_hmacsha512_update(state, input, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_final(
    state: *mut crypto_auth_hmacsha512_state,
    out: *mut u8,
) -> i32 {
    let mut out0 = [0u8; 64];
    crypto_auth_hmacsha512_final(state, out0.as_mut_ptr());
    core::ptr::copy_nonoverlapping(out0.as_ptr(), out, 32);
    sodium_memzero(out0.as_mut_ptr() as *mut c_void, 64);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256(
    out: *mut u8,
    input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    let mut state = core::mem::MaybeUninit::<crypto_auth_hmacsha512_state>::uninit();
    crypto_auth_hmacsha512256_init(state.as_mut_ptr(), k, 32);
    crypto_auth_hmacsha512256_update(state.as_mut_ptr(), input, inlen);
    crypto_auth_hmacsha512256_final(state.as_mut_ptr(), out);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_verify(
    h: *const u8,
    input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    let mut correct = [0u8; 32];
    crypto_auth_hmacsha512256(correct.as_mut_ptr(), input, inlen, k);
    crypto_verify_32(h, correct.as_ptr())
        | (-((h == correct.as_ptr()) as i32))
        | sodium_memcmp(correct.as_ptr() as *const c_void, h as *const c_void, 32)
}

// ==== crypto_auth.c dispatch ====

#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_bytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_keybytes() -> usize {
    32
}

static AUTH_PRIMITIVE: &[u8] = b"hmacsha512256\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_primitive() -> *const c_char {
    AUTH_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth(out: *mut u8, input: *const u8, inlen: u64, k: *const u8) -> i32 {
    crypto_auth_hmacsha512256(out, input, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_verify(
    h: *const u8,
    input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    crypto_auth_hmacsha512256_verify(h, input, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}
