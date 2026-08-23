//! Translation of `crypto_pwhash/scryptsalsa208sha256/pbkdf2-sha256.c`.
//!
//! `private/quirks.h` renames `escrypt_PBKDF2_SHA256` to
//! `_sodium_escrypt_PBKDF2_SHA256`.

use crate::common::{memcpy, store32_be};
use core::ffi::{c_int, c_void};

/* crypto_hash_sha256.h */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_hash_sha256_state {
    pub state: [u32; 8],
    pub count: u64,
    pub buf: [u8; 64],
}

/* crypto_auth_hmacsha256.h */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_auth_hmacsha256_state {
    pub ictx: crypto_hash_sha256_state,
    pub octx: crypto_hash_sha256_state,
}

extern "C" {
    fn crypto_auth_hmacsha256_init(
        state: *mut crypto_auth_hmacsha256_state,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    fn crypto_auth_hmacsha256_update(
        state: *mut crypto_auth_hmacsha256_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_auth_hmacsha256_final(
        state: *mut crypto_auth_hmacsha256_state,
        out: *mut u8,
    ) -> c_int;

    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_misuse() -> !;
}

/**
 * escrypt_PBKDF2_SHA256(passwd, passwdlen, salt, saltlen, c, buf, dkLen):
 * Compute PBKDF2(passwd, salt, c, dkLen) using HMAC-SHA256 as the PRF, and
 * write the output to buf.  The value dkLen must be at most 32 * (2^32 - 1).
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_PBKDF2_SHA256(
    passwd: *const u8,
    passwdlen: usize,
    salt: *const u8,
    saltlen: usize,
    c: u64,
    buf: *mut u8,
    dkLen: usize,
) {
    let mut PShctx = crypto_auth_hmacsha256_state {
        ictx: crypto_hash_sha256_state {
            state: [0; 8],
            count: 0,
            buf: [0; 64],
        },
        octx: crypto_hash_sha256_state {
            state: [0; 8],
            count: 0,
            buf: [0; 64],
        },
    };
    let mut hctx = PShctx;
    let mut i: usize;
    let mut ivec: [u8; 4] = [0; 4];
    let mut U: [u8; 32] = [0; 32];
    let mut T: [u8; 32] = [0; 32];
    let mut j: u64;
    let mut k: c_int;
    let mut clen: usize;

    /* #if SIZE_MAX > 0x1fffffffe0ULL  -- true on x86-64 */
    if dkLen as u64 > 0x1fffffffe0u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_auth_hmacsha256_init(&mut PShctx, passwd, passwdlen);
    crypto_auth_hmacsha256_update(&mut PShctx, salt, saltlen as u64);

    i = 0;
    while i.wrapping_mul(32) < dkLen {
        store32_be(ivec.as_mut_ptr(), i.wrapping_add(1) as u32);
        memcpy(
            (&mut hctx) as *mut crypto_auth_hmacsha256_state as *mut u8,
            (&PShctx) as *const crypto_auth_hmacsha256_state as *const u8,
            core::mem::size_of::<crypto_auth_hmacsha256_state>(),
        );
        crypto_auth_hmacsha256_update(&mut hctx, ivec.as_ptr(), 4);
        crypto_auth_hmacsha256_final(&mut hctx, U.as_mut_ptr());

        memcpy(T.as_mut_ptr(), U.as_ptr(), 32);
        /* LCOV_EXCL_START */
        j = 2;
        while j <= c {
            crypto_auth_hmacsha256_init(&mut hctx, passwd, passwdlen);
            crypto_auth_hmacsha256_update(&mut hctx, U.as_ptr(), 32);
            crypto_auth_hmacsha256_final(&mut hctx, U.as_mut_ptr());

            k = 0;
            while k < 32 {
                T[k as usize] ^= U[k as usize];
                k += 1;
            }
            j = j.wrapping_add(1);
        }
        /* LCOV_EXCL_STOP */

        clen = dkLen.wrapping_sub(i.wrapping_mul(32));
        if clen > 32 {
            clen = 32;
        }
        memcpy(buf.add(i.wrapping_mul(32)), T.as_ptr(), clen);

        i = i.wrapping_add(1);
    }
    sodium_memzero(
        (&mut PShctx) as *mut crypto_auth_hmacsha256_state as *mut c_void,
        core::mem::size_of::<crypto_auth_hmacsha256_state>(),
    );
}
