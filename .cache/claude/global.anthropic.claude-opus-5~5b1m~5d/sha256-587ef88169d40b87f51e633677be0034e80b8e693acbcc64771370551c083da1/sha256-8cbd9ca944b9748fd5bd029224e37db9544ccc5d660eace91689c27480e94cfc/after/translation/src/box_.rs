//! Translation of:
//! * `crypto_box/crypto_box.c`
//! * `crypto_box/crypto_box_easy.c`
//! * `crypto_box/crypto_box_seal.c`
//! * `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c`
//! * `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c`
//! * `crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c`
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_void};

use crate::common::SODIUM_SIZE_MAX;
use crate::types::crypto_generichash_blake2b_state;

/// `typedef crypto_generichash_blake2b_state crypto_generichash_state;`
type crypto_generichash_state = crypto_generichash_blake2b_state;

extern "C" {
    // ---- crypto_scalarmult_curve25519.h ----
    fn crypto_scalarmult_curve25519(q: *mut u8, n: *const u8, p: *const u8) -> c_int;
    fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> c_int;

    // ---- crypto_core_hsalsa20.h / crypto_core_hchacha20.h ----
    fn crypto_core_hsalsa20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;
    fn crypto_core_hchacha20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;

    // ---- crypto_secretbox_xsalsa20poly1305.h ----
    fn crypto_secretbox_xsalsa20poly1305(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_secretbox_xsalsa20poly1305_open(
        m: *mut u8,
        c: *const u8,
        clen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;

    // ---- crypto_secretbox.h ----
    fn crypto_secretbox_detached(
        c: *mut u8,
        mac: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_secretbox_open_detached(
        m: *mut u8,
        c: *const u8,
        mac: *const u8,
        clen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;

    // ---- crypto_secretbox_xchacha20poly1305.h ----
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

    // ---- crypto_generichash.h ----
    fn crypto_generichash_init(
        state: *mut crypto_generichash_state,
        key: *const u8,
        keylen: usize,
        outlen: usize,
    ) -> c_int;
    fn crypto_generichash_update(
        state: *mut crypto_generichash_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_generichash_final(
        state: *mut crypto_generichash_state,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;

    // ---- crypto_hash_sha512.h ----
    fn crypto_hash_sha512(out: *mut u8, inp: *const u8, inlen: u64) -> c_int;

    // ---- misc ----
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_misuse() -> !;
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

// =====================================================================
// crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut hash = [0u8; 64];

    crypto_hash_sha512(hash.as_mut_ptr(), seed, 32);
    memcpy(sk as *mut c_void, hash.as_ptr() as *const c_void, 32);
    sodium_memzero(hash.as_mut_ptr() as *mut c_void, hash.len());

    crypto_scalarmult_curve25519_base(pk, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_keypair(
    pk: *mut u8,
    sk: *mut u8,
) -> c_int {
    randombytes_buf(sk as *mut c_void, 32);

    crypto_scalarmult_curve25519_base(pk, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_beforenm(
    k: *mut u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    static ZERO: [u8; 16] = [0u8; 16];
    let mut s = [0u8; 32];

    if crypto_scalarmult_curve25519(s.as_mut_ptr(), sk, pk) != 0 {
        return -1;
    }
    crypto_core_hsalsa20(k, ZERO.as_ptr(), s.as_ptr(), core::ptr::null());
    sodium_memzero(s.as_mut_ptr() as *mut c_void, s.len());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_xsalsa20poly1305(c, m, mlen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_open_afternm(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_xsalsa20poly1305_open(m, c, clen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut k = [0u8; 32];

    if crypto_box_curve25519xsalsa20poly1305_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    let ret = crypto_box_curve25519xsalsa20poly1305_afternm(c, m, mlen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, k.len());

    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut k = [0u8; 32];

    if crypto_box_curve25519xsalsa20poly1305_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    let ret = crypto_box_curve25519xsalsa20poly1305_open_afternm(m, c, clen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, k.len());

    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_seedbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_publickeybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_secretkeybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_beforenmbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_noncebytes() -> usize {
    24
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_zerobytes() -> usize {
    16 + 16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_boxzerobytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_macbytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_messagebytes_max() -> usize {
    (SODIUM_SIZE_MAX - 16) as usize
}

// =====================================================================
// crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut hash = [0u8; 64];

    crypto_hash_sha512(hash.as_mut_ptr(), seed, 32);
    memcpy(sk as *mut c_void, hash.as_ptr() as *const c_void, 32);
    sodium_memzero(hash.as_mut_ptr() as *mut c_void, hash.len());

    crypto_scalarmult_curve25519_base(pk, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_keypair(
    pk: *mut u8,
    sk: *mut u8,
) -> c_int {
    randombytes_buf(sk as *mut c_void, 32);

    crypto_scalarmult_curve25519_base(pk, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_beforenm(
    k: *mut u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    static ZERO: [u8; 16] = [0u8; 16];
    let mut s = [0u8; 32];

    if crypto_scalarmult_curve25519(s.as_mut_ptr(), sk, pk) != 0 {
        return -1;
    }
    crypto_core_hchacha20(k, ZERO.as_ptr(), s.as_ptr(), core::ptr::null());
    sodium_memzero(s.as_mut_ptr() as *mut c_void, s.len());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_detached_afternm(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_xchacha20poly1305_detached(c, mac, m, mlen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_detached(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut k = [0u8; 32];

    if crypto_box_curve25519xchacha20poly1305_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    let ret =
        crypto_box_curve25519xchacha20poly1305_detached_afternm(c, mac, m, mlen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, k.len());

    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_easy_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > SODIUM_SIZE_MAX - 16 {
        sodium_misuse();
    }
    crypto_box_curve25519xchacha20poly1305_detached_afternm(c.add(16), c, m, mlen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_easy(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    if mlen > SODIUM_SIZE_MAX - 16 {
        sodium_misuse();
    }
    crypto_box_curve25519xchacha20poly1305_detached(c.add(16), c, m, mlen, n, pk, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_detached_afternm(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_xchacha20poly1305_open_detached(m, c, mac, clen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_detached(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut k = [0u8; 32];

    if crypto_box_curve25519xchacha20poly1305_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    let ret = crypto_box_curve25519xchacha20poly1305_open_detached_afternm(
        m,
        c,
        mac,
        clen,
        n,
        k.as_ptr(),
    );
    sodium_memzero(k.as_mut_ptr() as *mut c_void, k.len());

    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_easy_afternm(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen < 16 {
        return -1;
    }
    crypto_box_curve25519xchacha20poly1305_open_detached_afternm(
        m,
        c.add(16),
        c,
        clen - 16,
        n,
        k,
    )
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    if clen < 16 {
        return -1;
    }
    crypto_box_curve25519xchacha20poly1305_open_detached(
        m,
        c.add(16),
        c,
        clen - 16,
        n,
        pk,
        sk,
    )
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seedbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_publickeybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_secretkeybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_beforenmbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_noncebytes() -> usize {
    24
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_macbytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_messagebytes_max() -> usize {
    (SODIUM_SIZE_MAX - 16) as usize
}

// =====================================================================
// crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c
// =====================================================================

unsafe fn _crypto_box_curve25519xchacha20poly1305_seal_nonce(
    nonce: *mut u8,
    pk1: *const u8,
    pk2: *const u8,
) -> c_int {
    let mut st: crypto_generichash_state = core::mem::zeroed();

    crypto_generichash_init(&mut st, core::ptr::null(), 0, 24);
    crypto_generichash_update(&mut st, pk1, 32);
    crypto_generichash_update(&mut st, pk2, 32);
    crypto_generichash_final(&mut st, nonce, 24);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seal(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
) -> c_int {
    let mut nonce = [0u8; 24];
    let mut epk = [0u8; 32];
    let mut esk = [0u8; 32];

    if mlen > SODIUM_SIZE_MAX - 16 {
        sodium_misuse();
    }
    if crypto_box_curve25519xchacha20poly1305_keypair(epk.as_mut_ptr(), esk.as_mut_ptr()) != 0 {
        return -1;
    }
    _crypto_box_curve25519xchacha20poly1305_seal_nonce(nonce.as_mut_ptr(), epk.as_ptr(), pk);
    let ret = crypto_box_curve25519xchacha20poly1305_easy(
        c.add(32),
        m,
        mlen,
        nonce.as_ptr(),
        pk,
        esk.as_ptr(),
    );
    memcpy(c as *mut c_void, epk.as_ptr() as *const c_void, 32);
    sodium_memzero(esk.as_mut_ptr() as *mut c_void, esk.len());

    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seal_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut nonce = [0u8; 24];

    if clen < 32 + 16 {
        return -1;
    }
    _crypto_box_curve25519xchacha20poly1305_seal_nonce(nonce.as_mut_ptr(), c, pk);

    crypto_box_curve25519xchacha20poly1305_open_easy(
        m,
        c.add(32),
        clen - 32,
        nonce.as_ptr(),
        c,
        sk,
    )
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_sealbytes() -> usize {
    32 + 16
}

// =====================================================================
// crypto_box/crypto_box.c
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_box_seedbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_publickeybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_secretkeybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_beforenmbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_noncebytes() -> usize {
    24
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_zerobytes() -> usize {
    16 + 16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_boxzerobytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_macbytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_messagebytes_max() -> usize {
    (SODIUM_SIZE_MAX - 16) as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_primitive() -> *const c_char {
    b"curve25519xsalsa20poly1305\0".as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    crypto_box_curve25519xsalsa20poly1305_seed_keypair(pk, sk, seed)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    crypto_box_curve25519xsalsa20poly1305_keypair(pk, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_beforenm(k: *mut u8, pk: *const u8, sk: *const u8) -> c_int {
    crypto_box_curve25519xsalsa20poly1305_beforenm(k, pk, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_box_curve25519xsalsa20poly1305_afternm(c, m, mlen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_open_afternm(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_box_curve25519xsalsa20poly1305_open_afternm(m, c, clen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    crypto_box_curve25519xsalsa20poly1305(c, m, mlen, n, pk, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    crypto_box_curve25519xsalsa20poly1305_open(m, c, clen, n, pk, sk)
}

// =====================================================================
// crypto_box/crypto_box_easy.c
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_box_detached_afternm(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_detached(c, mac, m, mlen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_detached(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut k = [0u8; 32];

    if crypto_box_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    let ret = crypto_box_detached_afternm(c, mac, m, mlen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, k.len());

    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_easy_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > SODIUM_SIZE_MAX - 16 {
        sodium_misuse();
    }
    crypto_box_detached_afternm(c.add(16), c, m, mlen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_easy(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    if mlen > SODIUM_SIZE_MAX - 16 {
        sodium_misuse();
    }
    crypto_box_detached(c.add(16), c, m, mlen, n, pk, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_open_detached_afternm(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_open_detached(m, c, mac, clen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_open_detached(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut k = [0u8; 32];

    if crypto_box_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    let ret = crypto_box_open_detached_afternm(m, c, mac, clen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, k.len());

    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_open_easy_afternm(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen < 16 {
        return -1;
    }
    crypto_box_open_detached_afternm(m, c.add(16), c, clen - 16, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    if clen < 16 {
        return -1;
    }
    crypto_box_open_detached(m, c.add(16), c, clen - 16, n, pk, sk)
}

// =====================================================================
// crypto_box/crypto_box_seal.c
// =====================================================================

unsafe fn _crypto_box_seal_nonce(nonce: *mut u8, pk1: *const u8, pk2: *const u8) -> c_int {
    let mut st: crypto_generichash_state = core::mem::zeroed();

    crypto_generichash_init(&mut st, core::ptr::null(), 0, 24);
    crypto_generichash_update(&mut st, pk1, 32);
    crypto_generichash_update(&mut st, pk2, 32);
    crypto_generichash_final(&mut st, nonce, 24);

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_seal(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
) -> c_int {
    let mut nonce = [0u8; 24];
    let mut epk = [0u8; 32];
    let mut esk = [0u8; 32];

    if mlen > SODIUM_SIZE_MAX - 16 {
        sodium_misuse();
    }
    if crypto_box_keypair(epk.as_mut_ptr(), esk.as_mut_ptr()) != 0 {
        return -1;
    }
    _crypto_box_seal_nonce(nonce.as_mut_ptr(), epk.as_ptr(), pk);
    let ret = crypto_box_easy(c.add(32), m, mlen, nonce.as_ptr(), pk, esk.as_ptr());
    memcpy(c as *mut c_void, epk.as_ptr() as *const c_void, 32);
    sodium_memzero(esk.as_mut_ptr() as *mut c_void, esk.len());

    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_seal_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut nonce = [0u8; 24];

    if clen < 32 + 16 {
        return -1;
    }
    _crypto_box_seal_nonce(nonce.as_mut_ptr(), c, pk);

    crypto_box_open_easy(m, c.add(32), clen - 32, nonce.as_ptr(), c, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_box_sealbytes() -> usize {
    32 + 16
}
