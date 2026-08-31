//! Translation of c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pbkdf2-sha256.c

use core::ffi::{c_int, c_void};

use crate::common::store32_be;

// Mirror of crypto_hash_sha256_state (include/sodium/crypto_hash_sha256.h).
#[repr(C)]
struct crypto_hash_sha256_state {
    state: [u32; 8],
    count: u64,
    buf: [u8; 64],
}

// Mirror of crypto_auth_hmacsha256_state
// (include/sodium/crypto_auth_hmacsha256.h).
#[repr(C)]
struct crypto_auth_hmacsha256_state {
    ictx: crypto_hash_sha256_state,
    octx: crypto_hash_sha256_state,
}

extern "C" {
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;

    // Exported symbols from the sibling crypto_auth_hmacsha256 module.
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
}

/// escrypt_PBKDF2_SHA256(passwd, passwdlen, salt, saltlen, c, buf, dkLen):
/// Compute PBKDF2(passwd, salt, c, dkLen) using HMAC-SHA256 as the PRF, and
/// write the output to buf.  The value dkLen must be at most 32 * (2^32 - 1).
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
    // PShctx and hctx are zeroed here for a defined initial value; the C leaves
    // them uninitialized but fully overwrites before use.
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
    let mut hctx = crypto_auth_hmacsha256_state {
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
    let mut i: usize;
    let mut ivec: [u8; 4] = [0; 4];
    let mut U: [u8; 32] = [0; 32];
    let mut T: [u8; 32] = [0; 32];
    let mut j: u64;
    let mut k: c_int;
    let mut clen: usize;

    // SIZE_MAX > 0x1fffffffe0ULL on x86_64: the dkLen bound check is compiled.
    if dkLen as u64 > 0x1fffffffe0u64 {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_auth_hmacsha256_init(&mut PShctx, passwd, passwdlen);
    crypto_auth_hmacsha256_update(&mut PShctx, salt, saltlen as u64);

    i = 0;
    while i * 32 < dkLen {
        store32_be(ivec.as_mut_ptr(), (i + 1) as u32);
        memcpy(
            &mut hctx as *mut _ as *mut c_void,
            &PShctx as *const _ as *const c_void,
            core::mem::size_of::<crypto_auth_hmacsha256_state>(),
        );
        crypto_auth_hmacsha256_update(&mut hctx, ivec.as_ptr(), 4);
        crypto_auth_hmacsha256_final(&mut hctx, U.as_mut_ptr());

        memcpy(
            T.as_mut_ptr() as *mut c_void,
            U.as_ptr() as *const c_void,
            32,
        );
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
            j += 1;
        }
        /* LCOV_EXCL_STOP */

        clen = dkLen - i * 32;
        if clen > 32 {
            clen = 32;
        }
        memcpy(
            buf.add(i * 32) as *mut c_void,
            T.as_ptr() as *const c_void,
            clen,
        );
        i += 1;
    }
    sodium_memzero(
        &mut PShctx as *mut _ as *mut c_void,
        core::mem::size_of::<crypto_auth_hmacsha256_state>(),
    );
}
