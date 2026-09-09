//! Translation of crypto_pwhash/scryptsalsa208sha256/pbkdf2-sha256.c
//! (+ pbkdf2-sha256.h)

use core::ffi::c_void;

use crate::common::store32_be;
use crate::crypto_auth::hmacsha256::{
    crypto_auth_hmacsha256_final, crypto_auth_hmacsha256_init, crypto_auth_hmacsha256_state,
    crypto_auth_hmacsha256_update,
};
use crate::sodium_core::sodium_misuse;
use crate::sodium_utils::sodium_memzero;

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
    let mut PShctx: crypto_auth_hmacsha256_state = core::mem::zeroed();
    let mut hctx: crypto_auth_hmacsha256_state = core::mem::zeroed();
    let mut i: usize;
    let mut ivec: [u8; 4] = [0; 4];
    let mut U: [u8; 32] = [0; 32];
    let mut T: [u8; 32] = [0; 32];
    let mut j: u64;
    let mut k: i32;
    let mut clen: usize;

    /* SIZE_MAX > 0x1fffffffe0ULL on LP64 x86-64 */
    if dkLen > 0x1fffffffe0u64 as usize {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_auth_hmacsha256_init(&mut PShctx, passwd, passwdlen);
    crypto_auth_hmacsha256_update(&mut PShctx, salt, saltlen as u64);

    i = 0;
    while i * 32 < dkLen {
        store32_be(ivec.as_mut_ptr(), (i + 1) as u32);
        core::ptr::copy_nonoverlapping(
            &PShctx as *const crypto_auth_hmacsha256_state,
            &mut hctx as *mut crypto_auth_hmacsha256_state,
            1,
        );
        crypto_auth_hmacsha256_update(&mut hctx, ivec.as_ptr(), 4);
        crypto_auth_hmacsha256_final(&mut hctx, U.as_mut_ptr());

        core::ptr::copy_nonoverlapping(U.as_ptr(), T.as_mut_ptr(), 32);
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
        core::ptr::copy_nonoverlapping(T.as_ptr(), buf.add(i * 32), clen);
        i += 1;
    }
    sodium_memzero(
        &mut PShctx as *mut crypto_auth_hmacsha256_state as *mut c_void,
        core::mem::size_of::<crypto_auth_hmacsha256_state>(),
    );
}
