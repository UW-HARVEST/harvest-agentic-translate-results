//! Translation of `crypto_auth/hmacsha512/auth_hmacsha512.c`.
//!
//! Nothing in this file is affected by `private/quirks.h`, so every exported
//! function keeps its plain C name.

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

pub const crypto_auth_hmacsha512_BYTES: usize = 64;
pub const crypto_auth_hmacsha512_KEYBYTES: usize = 32;

extern "C" {
    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int;
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_hash_sha512_final(state: *mut crypto_hash_sha512_state, out: *mut u8) -> c_int;

    fn crypto_verify_64(x: *const u8, y: *const u8) -> c_int;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1_: *const c_void, b2_: *const c_void, len: usize) -> c_int;
    fn sodium_misuse() -> !;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

/* size_t crypto_auth_hmacsha512_bytes(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_bytes() -> usize {
    crypto_auth_hmacsha512_BYTES
}

/* size_t crypto_auth_hmacsha512_keybytes(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_keybytes() -> usize {
    crypto_auth_hmacsha512_KEYBYTES
}

/* size_t crypto_auth_hmacsha512_statebytes(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_statebytes() -> usize {
    core::mem::size_of::<crypto_auth_hmacsha512_state>()
}

/* void crypto_auth_hmacsha512_keygen(unsigned char k[32]) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_auth_hmacsha512_KEYBYTES);
}

/* int crypto_auth_hmacsha512_init(crypto_auth_hmacsha512_state *state,
                                   const unsigned char *key, size_t keylen) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_init(
    state: *mut crypto_auth_hmacsha512_state,
    key: *const u8,
    keylen: usize,
) -> c_int {
    let mut pad: [u8; 128] = [0; 128];
    let mut khash: [u8; 64] = [0; 64];
    let mut i: usize;

    let mut key = key;
    let mut keylen = keylen;

    if keylen > 128 {
        crypto_hash_sha512_init(addr_of_mut!((*state).ictx));
        crypto_hash_sha512_update(addr_of_mut!((*state).ictx), key, keylen as c_ulonglong);
        crypto_hash_sha512_final(addr_of_mut!((*state).ictx), khash.as_mut_ptr());
        key = khash.as_ptr();
        keylen = 64;
    } else if key.is_null() {
        if keylen > 0 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    }
    crypto_hash_sha512_init(addr_of_mut!((*state).ictx));
    core::ptr::write_bytes(pad.as_mut_ptr(), 0x36, 128);
    i = 0;
    while i < keylen {
        pad[i] ^= *key.add(i);
        i += 1;
    }
    crypto_hash_sha512_update(
        addr_of_mut!((*state).ictx),
        pad.as_ptr(),
        128 as c_ulonglong,
    );

    crypto_hash_sha512_init(addr_of_mut!((*state).octx));
    core::ptr::write_bytes(pad.as_mut_ptr(), 0x5c, 128);
    i = 0;
    while i < keylen {
        pad[i] ^= *key.add(i);
        i += 1;
    }
    crypto_hash_sha512_update(
        addr_of_mut!((*state).octx),
        pad.as_ptr(),
        128 as c_ulonglong,
    );

    sodium_memzero(pad.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&pad));
    sodium_memzero(
        khash.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&khash),
    );

    0
}

/* int crypto_auth_hmacsha512_update(crypto_auth_hmacsha512_state *state,
                                     const unsigned char *in,
                                     unsigned long long inlen) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_update(
    state: *mut crypto_auth_hmacsha512_state,
    in_: *const u8,
    inlen: c_ulonglong,
) -> c_int {
    crypto_hash_sha512_update(addr_of_mut!((*state).ictx), in_, inlen);

    0
}

/* int crypto_auth_hmacsha512_final(crypto_auth_hmacsha512_state *state,
                                    unsigned char *out) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_final(
    state: *mut crypto_auth_hmacsha512_state,
    out: *mut u8,
) -> c_int {
    let mut ihash: [u8; 64] = [0; 64];

    crypto_hash_sha512_final(addr_of_mut!((*state).ictx), ihash.as_mut_ptr());
    crypto_hash_sha512_update(
        addr_of_mut!((*state).octx),
        ihash.as_ptr(),
        64 as c_ulonglong,
    );
    crypto_hash_sha512_final(addr_of_mut!((*state).octx), out);

    sodium_memzero(
        ihash.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&ihash),
    );

    0
}

/* int crypto_auth_hmacsha512(unsigned char *out, const unsigned char *in,
                              unsigned long long inlen, const unsigned char *k) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512(
    out: *mut u8,
    in_: *const u8,
    inlen: c_ulonglong,
    k: *const u8,
) -> c_int {
    let mut state_storage = core::mem::MaybeUninit::<crypto_auth_hmacsha512_state>::uninit();
    let state: *mut crypto_auth_hmacsha512_state = state_storage.as_mut_ptr();

    crypto_auth_hmacsha512_init(state, k, crypto_auth_hmacsha512_KEYBYTES);
    crypto_auth_hmacsha512_update(state, in_, inlen);
    crypto_auth_hmacsha512_final(state, out);

    0
}

/* int crypto_auth_hmacsha512_verify(const unsigned char *h,
                                     const unsigned char *in,
                                     unsigned long long inlen,
                                     const unsigned char *k) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_hmacsha512_verify(
    h: *const u8,
    in_: *const u8,
    inlen: c_ulonglong,
    k: *const u8,
) -> c_int {
    let mut correct: [u8; 64] = [0; 64];

    crypto_auth_hmacsha512(correct.as_mut_ptr(), in_, inlen, k);

    let a = crypto_verify_64(h, correct.as_ptr());
    let b = -((h == correct.as_ptr() as *const u8) as c_int);
    let c = sodium_memcmp(
        correct.as_ptr() as *const c_void,
        h as *const c_void,
        64,
    );

    a | b | c
}
