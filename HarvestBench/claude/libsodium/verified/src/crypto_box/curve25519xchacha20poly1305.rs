//! Translation of
//! `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c`.

use core::ffi::{c_int, c_void};

use crate::common::SODIUM_SIZE_MAX;
use crate::randombytes::randombytes_buf;
use crate::sodium::core::sodium_misuse;
use crate::sodium::utils::sodium_memzero;

// ---------------------------------------------------------------------------
// Constants from include/sodium/crypto_box_curve25519xchacha20poly1305.h
// ---------------------------------------------------------------------------

/// `#define crypto_box_curve25519xchacha20poly1305_SEEDBYTES 32U`
pub const crypto_box_curve25519xchacha20poly1305_SEEDBYTES: usize = 32;
/// `#define crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES 32U`
pub const crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES: usize = 32;
/// `#define crypto_box_curve25519xchacha20poly1305_SECRETKEYBYTES 32U`
pub const crypto_box_curve25519xchacha20poly1305_SECRETKEYBYTES: usize = 32;
/// `#define crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES 32U`
pub const crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES: usize = 32;
/// `#define crypto_box_curve25519xchacha20poly1305_NONCEBYTES 24U`
pub const crypto_box_curve25519xchacha20poly1305_NONCEBYTES: usize = 24;
/// `#define crypto_box_curve25519xchacha20poly1305_MACBYTES 16U`
pub const crypto_box_curve25519xchacha20poly1305_MACBYTES: usize = 16;
/// `crypto_stream_xchacha20_MESSAGEBYTES_MAX - MACBYTES`, i.e.
/// `SODIUM_SIZE_MAX - 16`.
pub const crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX: usize =
    (SODIUM_SIZE_MAX - crypto_box_curve25519xchacha20poly1305_MACBYTES as u64) as usize;
/// `PUBLICKEYBYTES + MACBYTES`
pub const crypto_box_curve25519xchacha20poly1305_SEALBYTES: usize =
    crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES
        + crypto_box_curve25519xchacha20poly1305_MACBYTES;

// ---------------------------------------------------------------------------
// Cross-file C entry points
// ---------------------------------------------------------------------------

unsafe extern "C" {
    /// `crypto_hash/sha512/cp/hash_sha512_cp.c`
    fn crypto_hash_sha512(out: *mut u8, in_: *const u8, inlen: u64) -> c_int;
    /// `crypto_scalarmult/curve25519/scalarmult_curve25519.c`
    fn crypto_scalarmult_curve25519(q: *mut u8, n: *const u8, p: *const u8) -> c_int;
    fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> c_int;
    /// `crypto_core/hchacha20/core_hchacha20.c`
    fn crypto_core_hchacha20(
        out: *mut u8,
        in_: *const u8,
        k: *const u8,
        c: *const u8,
    ) -> c_int;
    /// `crypto_secretbox/crypto_secretbox_xchacha20poly1305.c`
    fn crypto_secretbox_xchacha20poly1305_detached(
        c: *mut u8,
        mac: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_secretbox_xchacha20poly1305_open_detached(
        m: *mut u8,
        c: *const u8,
        mac: *const u8,
        clen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
}

// ---------------------------------------------------------------------------
// Key generation
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    unsafe {
        let mut hash: [u8; 64] = [0; 64];

        crypto_hash_sha512(hash.as_mut_ptr(), seed, 32);
        core::ptr::copy_nonoverlapping(hash.as_ptr(), sk, 32);
        sodium_memzero(hash.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&hash));

        crypto_scalarmult_curve25519_base(pk, sk)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_keypair(
    pk: *mut u8,
    sk: *mut u8,
) -> c_int {
    unsafe {
        randombytes_buf(sk as *mut c_void, 32);

        crypto_scalarmult_curve25519_base(pk, sk)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_beforenm(
    k: *mut u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    unsafe {
        static zero: [u8; 16] = [0; 16];
        let mut s: [u8; 32] = [0; 32];

        if crypto_scalarmult_curve25519(s.as_mut_ptr(), sk, pk) != 0 {
            return -1;
        }
        crypto_core_hchacha20(k, zero.as_ptr(), s.as_ptr(), core::ptr::null());
        sodium_memzero(s.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&s));

        0
    }
}

// ---------------------------------------------------------------------------
// Detached / easy interfaces
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_detached_afternm(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    unsafe { crypto_secretbox_xchacha20poly1305_detached(c, mac, m, mlen, n, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_detached(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    unsafe {
        let mut k: [u8; crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES] =
            [0; crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES];
        let ret: c_int;

        // COMPILER_ASSERT(BEFORENMBYTES >= crypto_secretbox_xchacha20poly1305_KEYBYTES)
        const _: () = assert!(crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES >= 32);
        if crypto_box_curve25519xchacha20poly1305_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
            return -1;
        }
        ret = crypto_box_curve25519xchacha20poly1305_detached_afternm(
            c,
            mac,
            m,
            mlen,
            n,
            k.as_ptr(),
        );
        sodium_memzero(k.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&k));

        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_easy_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    unsafe {
        if mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX as u64 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
        crypto_box_curve25519xchacha20poly1305_detached_afternm(
            c.add(crypto_box_curve25519xchacha20poly1305_MACBYTES),
            c,
            m,
            mlen,
            n,
            k,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_easy(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    unsafe {
        if mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX as u64 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
        crypto_box_curve25519xchacha20poly1305_detached(
            c.add(crypto_box_curve25519xchacha20poly1305_MACBYTES),
            c,
            m,
            mlen,
            n,
            pk,
            sk,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_detached_afternm(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    unsafe { crypto_secretbox_xchacha20poly1305_open_detached(m, c, mac, clen, n, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_detached(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    unsafe {
        let mut k: [u8; crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES] =
            [0; crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES];
        let ret: c_int;

        if crypto_box_curve25519xchacha20poly1305_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
            return -1;
        }
        ret = crypto_box_curve25519xchacha20poly1305_open_detached_afternm(
            m,
            c,
            mac,
            clen,
            n,
            k.as_ptr(),
        );
        sodium_memzero(k.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&k));

        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_easy_afternm(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    unsafe {
        if clen < crypto_box_curve25519xchacha20poly1305_MACBYTES as u64 {
            return -1;
        }
        crypto_box_curve25519xchacha20poly1305_open_detached_afternm(
            m,
            c.add(crypto_box_curve25519xchacha20poly1305_MACBYTES),
            c,
            clen - crypto_box_curve25519xchacha20poly1305_MACBYTES as u64,
            n,
            k,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    unsafe {
        if clen < crypto_box_curve25519xchacha20poly1305_MACBYTES as u64 {
            return -1;
        }
        crypto_box_curve25519xchacha20poly1305_open_detached(
            m,
            c.add(crypto_box_curve25519xchacha20poly1305_MACBYTES),
            c,
            clen - crypto_box_curve25519xchacha20poly1305_MACBYTES as u64,
            n,
            pk,
            sk,
        )
    }
}

// ---------------------------------------------------------------------------
// Size accessors
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seedbytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_publickeybytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_secretkeybytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_beforenmbytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_noncebytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_macbytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_MACBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_messagebytes_max() -> usize {
    crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX
}
