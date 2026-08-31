//! Translated from:
//! * `crypto_auth/hmacsha256/auth_hmacsha256.c`
//! * `crypto_auth/hmacsha512/auth_hmacsha512.c`
//! * `crypto_auth/hmacsha512256/auth_hmacsha512256.c`
//! * `crypto_auth/crypto_auth.c`

use core::ffi::{c_char, c_int};

use crate::types::{crypto_hash_sha256_state, crypto_hash_sha512_state};

extern "C" {
    fn crypto_hash_sha256_init(state: *mut crypto_hash_sha256_state) -> c_int;
    fn crypto_hash_sha256_update(
        state: *mut crypto_hash_sha256_state,
        inp: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha256_final(state: *mut crypto_hash_sha256_state, out: *mut u8) -> c_int;

    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int;
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        inp: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha512_final(state: *mut crypto_hash_sha512_state, out: *mut u8) -> c_int;

    fn crypto_verify_32(x: *const u8, y: *const u8) -> c_int;
    fn crypto_verify_64(x: *const u8, y: *const u8) -> c_int;

    fn sodium_memcmp(b1: *const core::ffi::c_void, b2: *const core::ffi::c_void, len: usize)
        -> c_int;
    fn sodium_memzero(pnt: *mut core::ffi::c_void, len: usize);

    fn randombytes_buf(buf: *mut core::ffi::c_void, size: usize);

    fn sodium_misuse() -> !;
}

/// `crypto_auth_hmacsha256_state` — from `crypto_auth_hmacsha256.h`
#[repr(C)]
pub struct crypto_auth_hmacsha256_state {
    pub ictx: crypto_hash_sha256_state,
    pub octx: crypto_hash_sha256_state,
}

/// `crypto_auth_hmacsha512_state` — from `crypto_auth_hmacsha512.h`
#[repr(C)]
pub struct crypto_auth_hmacsha512_state {
    pub ictx: crypto_hash_sha512_state,
    pub octx: crypto_hash_sha512_state,
}

/// `crypto_auth_hmacsha512256_state` — alias of `crypto_auth_hmacsha512_state`,
/// from `crypto_auth_hmacsha512256.h`
#[repr(C)]
pub struct crypto_auth_hmacsha512256_state {
    pub ictx: crypto_hash_sha512_state,
    pub octx: crypto_hash_sha512_state,
}

pub const crypto_auth_hmacsha256_BYTES: usize = 32;
pub const crypto_auth_hmacsha256_KEYBYTES: usize = 32;

pub const crypto_auth_hmacsha512_BYTES: usize = 64;
pub const crypto_auth_hmacsha512_KEYBYTES: usize = 32;

pub const crypto_auth_hmacsha512256_BYTES: usize = 32;
pub const crypto_auth_hmacsha512256_KEYBYTES: usize = 32;

pub const crypto_auth_BYTES: usize = crypto_auth_hmacsha512256_BYTES;
pub const crypto_auth_KEYBYTES: usize = crypto_auth_hmacsha512256_KEYBYTES;
pub const crypto_auth_PRIMITIVE: &[u8] = b"hmacsha512256\0";

// ---------------------------------------------------------------------------
// hmac-sha256
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha256_bytes() -> usize {
    crypto_auth_hmacsha256_BYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha256_keybytes() -> usize {
    crypto_auth_hmacsha256_KEYBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha256_statebytes() -> usize {
    core::mem::size_of::<crypto_auth_hmacsha256_state>()
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha256_keygen(k: *mut u8) {
    randombytes_buf(k as *mut core::ffi::c_void, crypto_auth_hmacsha256_KEYBYTES);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha256_init(
    state: *mut crypto_auth_hmacsha256_state,
    key: *const u8,
    keylen: usize,
) -> c_int {
    let mut pad = [0u8; 64];
    let mut khash = [0u8; 32];
    let mut key = key;
    let mut keylen = keylen;

    if keylen > 64 {
        crypto_hash_sha256_init(&mut (*state).ictx);
        crypto_hash_sha256_update(&mut (*state).ictx, key, keylen as u64);
        crypto_hash_sha256_final(&mut (*state).ictx, khash.as_mut_ptr());
        key = khash.as_ptr();
        keylen = 32;
    } else if key.is_null() {
        if keylen > 0 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    }
    crypto_hash_sha256_init(&mut (*state).ictx);
    for b in pad.iter_mut() {
        *b = 0x36;
    }
    for i in 0..keylen {
        pad[i] ^= *key.add(i);
    }
    crypto_hash_sha256_update(&mut (*state).ictx, pad.as_ptr(), 64);

    crypto_hash_sha256_init(&mut (*state).octx);
    for b in pad.iter_mut() {
        *b = 0x5c;
    }
    for i in 0..keylen {
        pad[i] ^= *key.add(i);
    }
    crypto_hash_sha256_update(&mut (*state).octx, pad.as_ptr(), 64);

    sodium_memzero(pad.as_mut_ptr() as *mut core::ffi::c_void, pad.len());
    sodium_memzero(khash.as_mut_ptr() as *mut core::ffi::c_void, khash.len());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha256_update(
    state: *mut crypto_auth_hmacsha256_state,
    inp: *const u8,
    inlen: u64,
) -> c_int {
    crypto_hash_sha256_update(&mut (*state).ictx, inp, inlen);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha256_final(
    state: *mut crypto_auth_hmacsha256_state,
    out: *mut u8,
) -> c_int {
    let mut ihash = [0u8; 32];

    crypto_hash_sha256_final(&mut (*state).ictx, ihash.as_mut_ptr());
    crypto_hash_sha256_update(&mut (*state).octx, ihash.as_ptr(), 32);
    crypto_hash_sha256_final(&mut (*state).octx, out);

    sodium_memzero(ihash.as_mut_ptr() as *mut core::ffi::c_void, ihash.len());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha256(
    out: *mut u8,
    inp: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut state = core::mem::MaybeUninit::<crypto_auth_hmacsha256_state>::uninit();
    let state = state.as_mut_ptr();

    crypto_auth_hmacsha256_init(state, k, crypto_auth_hmacsha256_KEYBYTES);
    crypto_auth_hmacsha256_update(state, inp, inlen);
    crypto_auth_hmacsha256_final(state, out);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha256_verify(
    h: *const u8,
    inp: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut correct = [0u8; 32];

    crypto_auth_hmacsha256(correct.as_mut_ptr(), inp, inlen, k);

    crypto_verify_32(h, correct.as_ptr())
        | (0i32.wrapping_sub((h == correct.as_ptr()) as i32))
        | sodium_memcmp(
            correct.as_ptr() as *const core::ffi::c_void,
            h as *const core::ffi::c_void,
            32,
        )
}

// ---------------------------------------------------------------------------
// hmac-sha512
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512_bytes() -> usize {
    crypto_auth_hmacsha512_BYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512_keybytes() -> usize {
    crypto_auth_hmacsha512_KEYBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512_statebytes() -> usize {
    core::mem::size_of::<crypto_auth_hmacsha512_state>()
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512_keygen(k: *mut u8) {
    randombytes_buf(k as *mut core::ffi::c_void, crypto_auth_hmacsha512_KEYBYTES);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512_init(
    state: *mut crypto_auth_hmacsha512_state,
    key: *const u8,
    keylen: usize,
) -> c_int {
    let mut pad = [0u8; 128];
    let mut khash = [0u8; 64];
    let mut key = key;
    let mut keylen = keylen;

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
    for b in pad.iter_mut() {
        *b = 0x36;
    }
    for i in 0..keylen {
        pad[i] ^= *key.add(i);
    }
    crypto_hash_sha512_update(&mut (*state).ictx, pad.as_ptr(), 128);

    crypto_hash_sha512_init(&mut (*state).octx);
    for b in pad.iter_mut() {
        *b = 0x5c;
    }
    for i in 0..keylen {
        pad[i] ^= *key.add(i);
    }
    crypto_hash_sha512_update(&mut (*state).octx, pad.as_ptr(), 128);

    sodium_memzero(pad.as_mut_ptr() as *mut core::ffi::c_void, pad.len());
    sodium_memzero(khash.as_mut_ptr() as *mut core::ffi::c_void, khash.len());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512_update(
    state: *mut crypto_auth_hmacsha512_state,
    inp: *const u8,
    inlen: u64,
) -> c_int {
    crypto_hash_sha512_update(&mut (*state).ictx, inp, inlen);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512_final(
    state: *mut crypto_auth_hmacsha512_state,
    out: *mut u8,
) -> c_int {
    let mut ihash = [0u8; 64];

    crypto_hash_sha512_final(&mut (*state).ictx, ihash.as_mut_ptr());
    crypto_hash_sha512_update(&mut (*state).octx, ihash.as_ptr(), 64);
    crypto_hash_sha512_final(&mut (*state).octx, out);

    sodium_memzero(ihash.as_mut_ptr() as *mut core::ffi::c_void, ihash.len());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512(
    out: *mut u8,
    inp: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut state = core::mem::MaybeUninit::<crypto_auth_hmacsha512_state>::uninit();
    let state = state.as_mut_ptr();

    crypto_auth_hmacsha512_init(state, k, crypto_auth_hmacsha512_KEYBYTES);
    crypto_auth_hmacsha512_update(state, inp, inlen);
    crypto_auth_hmacsha512_final(state, out);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512_verify(
    h: *const u8,
    inp: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut correct = [0u8; 64];

    crypto_auth_hmacsha512(correct.as_mut_ptr(), inp, inlen, k);

    crypto_verify_64(h, correct.as_ptr())
        | (0i32.wrapping_sub((h == correct.as_ptr()) as i32))
        | sodium_memcmp(
            correct.as_ptr() as *const core::ffi::c_void,
            h as *const core::ffi::c_void,
            64,
        )
}

// ---------------------------------------------------------------------------
// hmac-sha512256
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_bytes() -> usize {
    crypto_auth_hmacsha512256_BYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_keybytes() -> usize {
    crypto_auth_hmacsha512256_KEYBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_statebytes() -> usize {
    core::mem::size_of::<crypto_auth_hmacsha512256_state>()
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_keygen(k: *mut u8) {
    randombytes_buf(
        k as *mut core::ffi::c_void,
        crypto_auth_hmacsha512256_KEYBYTES,
    );
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_init(
    state: *mut crypto_auth_hmacsha512256_state,
    key: *const u8,
    keylen: usize,
) -> c_int {
    crypto_auth_hmacsha512_init(state as *mut crypto_auth_hmacsha512_state, key, keylen)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_update(
    state: *mut crypto_auth_hmacsha512256_state,
    inp: *const u8,
    inlen: u64,
) -> c_int {
    crypto_auth_hmacsha512_update(state as *mut crypto_auth_hmacsha512_state, inp, inlen)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_final(
    state: *mut crypto_auth_hmacsha512256_state,
    out: *mut u8,
) -> c_int {
    let mut out0 = [0u8; 64];

    crypto_auth_hmacsha512_final(state as *mut crypto_auth_hmacsha512_state, out0.as_mut_ptr());
    core::ptr::copy_nonoverlapping(out0.as_ptr(), out, 32);
    sodium_memzero(out0.as_mut_ptr() as *mut core::ffi::c_void, out0.len());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512256(
    out: *mut u8,
    inp: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut state = core::mem::MaybeUninit::<crypto_auth_hmacsha512256_state>::uninit();
    let state = state.as_mut_ptr();

    crypto_auth_hmacsha512256_init(state, k, crypto_auth_hmacsha512256_KEYBYTES);
    crypto_auth_hmacsha512256_update(state, inp, inlen);
    crypto_auth_hmacsha512256_final(state, out);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_hmacsha512256_verify(
    h: *const u8,
    inp: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut correct = [0u8; 32];

    crypto_auth_hmacsha512256(correct.as_mut_ptr(), inp, inlen, k);

    crypto_verify_32(h, correct.as_ptr())
        | (0i32.wrapping_sub((h == correct.as_ptr()) as i32))
        | sodium_memcmp(
            correct.as_ptr() as *const core::ffi::c_void,
            h as *const core::ffi::c_void,
            32,
        )
}

// ---------------------------------------------------------------------------
// crypto_auth.c — top-level dispatch (hmacsha512256)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_bytes() -> usize {
    crypto_auth_BYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_keybytes() -> usize {
    crypto_auth_KEYBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_primitive() -> *const c_char {
    crypto_auth_PRIMITIVE.as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth(
    out: *mut u8,
    inp: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    crypto_auth_hmacsha512256(out, inp, inlen, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_verify(
    h: *const u8,
    inp: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    crypto_auth_hmacsha512256_verify(h, inp, inlen, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_auth_keygen(k: *mut u8) {
    randombytes_buf(k as *mut core::ffi::c_void, crypto_auth_KEYBYTES);
}
