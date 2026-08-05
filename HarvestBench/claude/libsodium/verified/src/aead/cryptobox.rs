//! Translated from crypto_box/{crypto_box.c, crypto_box_easy.c, crypto_box_seal.c,
//! curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c,
//! curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c,
//! curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c}
use crate::primitives::generichash::crypto_generichash_blake2b_state;
use core::ffi::{c_char, c_void};

extern "C" {
    fn crypto_hash_sha512(out: *mut u8, input: *const u8, inlen: u64) -> i32;
    fn crypto_scalarmult_curve25519(q: *mut u8, n: *const u8, p: *const u8) -> i32;
    fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> i32;
    fn crypto_core_hsalsa20(out: *mut u8, input: *const u8, k: *const u8, c: *const u8) -> i32;
    fn crypto_core_hchacha20(out: *mut u8, input: *const u8, k: *const u8, c: *const u8) -> i32;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_misuse() -> !;
    fn randombytes_buf(buf: *mut c_void, size: usize);

    // crypto_secretbox family (this package)
    fn crypto_secretbox_xsalsa20poly1305(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> i32;
    fn crypto_secretbox_xsalsa20poly1305_open(
        m: *mut u8,
        c: *const u8,
        clen: u64,
        n: *const u8,
        k: *const u8,
    ) -> i32;
    fn crypto_secretbox_detached(
        c: *mut u8,
        mac: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> i32;
    fn crypto_secretbox_open_detached(
        m: *mut u8,
        c: *const u8,
        mac: *const u8,
        clen: u64,
        n: *const u8,
        k: *const u8,
    ) -> i32;
    fn crypto_secretbox_xchacha20poly1305_detached(
        c: *mut u8,
        mac: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> i32;
    fn crypto_secretbox_xchacha20poly1305_open_detached(
        m: *mut u8,
        c: *const u8,
        mac: *const u8,
        clen: u64,
        n: *const u8,
        k: *const u8,
    ) -> i32;

    fn crypto_generichash_init(
        state: *mut crypto_generichash_blake2b_state,
        key: *const u8,
        keylen: usize,
        outlen: usize,
    ) -> i32;
    fn crypto_generichash_update(
        state: *mut crypto_generichash_blake2b_state,
        input: *const u8,
        inlen: u64,
    ) -> i32;
    fn crypto_generichash_final(
        state: *mut crypto_generichash_blake2b_state,
        out: *mut u8,
        outlen: usize,
    ) -> i32;
}

const SEEDBYTES: usize = 32;
const PUBLICKEYBYTES: usize = 32;
const SECRETKEYBYTES: usize = 32;
const BEFORENMBYTES: usize = 32;
const NONCEBYTES: usize = 24;
const ZEROBYTES: usize = 32;
const BOXZEROBYTES: usize = 16;
const MACBYTES: usize = 16;
// crypto_stream_xsalsa20_MESSAGEBYTES_MAX = SODIUM_SIZE_MAX
const MESSAGEBYTES_MAX: u64 = u64::MAX - MACBYTES as u64;
const SEALBYTES: usize = PUBLICKEYBYTES + MACBYTES;

// ======== curve25519xsalsa20poly1305 ========

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
    let mut hash = [0u8; 64];
    crypto_hash_sha512(hash.as_mut_ptr(), seed, 32);
    core::ptr::copy_nonoverlapping(hash.as_ptr(), sk, 32);
    sodium_memzero(hash.as_mut_ptr() as *mut c_void, 64);
    crypto_scalarmult_curve25519_base(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_keypair(
    pk: *mut u8,
    sk: *mut u8,
) -> i32 {
    randombytes_buf(sk as *mut c_void, 32);
    crypto_scalarmult_curve25519_base(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_beforenm(
    k: *mut u8,
    pk: *const u8,
    sk: *const u8,
) -> i32 {
    let zero = [0u8; 16];
    let mut s = [0u8; 32];
    if crypto_scalarmult_curve25519(s.as_mut_ptr(), sk, pk) != 0 {
        return -1;
    }
    crypto_core_hsalsa20(k, zero.as_ptr(), s.as_ptr(), core::ptr::null());
    sodium_memzero(s.as_mut_ptr() as *mut c_void, 32);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    crypto_secretbox_xsalsa20poly1305(c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_open_afternm(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    crypto_secretbox_xsalsa20poly1305_open(m, c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> i32 {
    let mut k = [0u8; BEFORENMBYTES];
    if crypto_box_curve25519xsalsa20poly1305_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    let ret = crypto_box_curve25519xsalsa20poly1305_afternm(c, m, mlen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, BEFORENMBYTES);
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xsalsa20poly1305_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> i32 {
    let mut k = [0u8; BEFORENMBYTES];
    if crypto_box_curve25519xsalsa20poly1305_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    let ret = crypto_box_curve25519xsalsa20poly1305_open_afternm(m, c, clen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, BEFORENMBYTES);
    ret
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xsalsa20poly1305_seedbytes() -> usize {
    SEEDBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xsalsa20poly1305_publickeybytes() -> usize {
    PUBLICKEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xsalsa20poly1305_secretkeybytes() -> usize {
    SECRETKEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xsalsa20poly1305_beforenmbytes() -> usize {
    BEFORENMBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xsalsa20poly1305_noncebytes() -> usize {
    NONCEBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xsalsa20poly1305_zerobytes() -> usize {
    ZEROBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xsalsa20poly1305_boxzerobytes() -> usize {
    BOXZEROBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xsalsa20poly1305_macbytes() -> usize {
    MACBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xsalsa20poly1305_messagebytes_max() -> usize {
    MESSAGEBYTES_MAX as usize
}

// ======== crypto_box.c dispatch ========

#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_seedbytes() -> usize {
    SEEDBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_publickeybytes() -> usize {
    PUBLICKEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_secretkeybytes() -> usize {
    SECRETKEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_beforenmbytes() -> usize {
    BEFORENMBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_noncebytes() -> usize {
    NONCEBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_zerobytes() -> usize {
    ZEROBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_boxzerobytes() -> usize {
    BOXZEROBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_macbytes() -> usize {
    MACBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_messagebytes_max() -> usize {
    MESSAGEBYTES_MAX as usize
}

static BOX_PRIMITIVE: &[u8] = b"curve25519xsalsa20poly1305\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_primitive() -> *const c_char {
    BOX_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> i32 {
    crypto_box_curve25519xsalsa20poly1305_seed_keypair(pk, sk, seed)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    crypto_box_curve25519xsalsa20poly1305_keypair(pk, sk)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_beforenm(k: *mut u8, pk: *const u8, sk: *const u8) -> i32 {
    crypto_box_curve25519xsalsa20poly1305_beforenm(k, pk, sk)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    crypto_box_curve25519xsalsa20poly1305_afternm(c, m, mlen, n, k)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_afternm(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    crypto_box_curve25519xsalsa20poly1305_open_afternm(m, c, clen, n, k)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> i32 {
    crypto_box_curve25519xsalsa20poly1305(c, m, mlen, n, pk, sk)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> i32 {
    crypto_box_curve25519xsalsa20poly1305_open(m, c, clen, n, pk, sk)
}

// ======== crypto_box_easy.c ========

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_detached_afternm(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    crypto_secretbox_detached(c, mac, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_detached(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> i32 {
    let mut k = [0u8; BEFORENMBYTES];
    if crypto_box_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    let ret = crypto_box_detached_afternm(c, mac, m, mlen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, BEFORENMBYTES);
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_easy_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    if mlen > MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    crypto_box_detached_afternm(c.add(MACBYTES), c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_easy(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> i32 {
    if mlen > MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    crypto_box_detached(c.add(MACBYTES), c, m, mlen, n, pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_detached_afternm(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    crypto_secretbox_open_detached(m, c, mac, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_detached(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> i32 {
    let mut k = [0u8; BEFORENMBYTES];
    if crypto_box_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    let ret = crypto_box_open_detached_afternm(m, c, mac, clen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, BEFORENMBYTES);
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_easy_afternm(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    if clen < MACBYTES as u64 {
        return -1;
    }
    crypto_box_open_detached_afternm(m, c.add(MACBYTES), c, clen - MACBYTES as u64, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> i32 {
    if clen < MACBYTES as u64 {
        return -1;
    }
    crypto_box_open_detached(m, c.add(MACBYTES), c, clen - MACBYTES as u64, n, pk, sk)
}

// ======== crypto_box_seal.c ========

unsafe fn _crypto_box_seal_nonce(nonce: *mut u8, pk1: *const u8, pk2: *const u8) -> i32 {
    let mut st = crypto_generichash_blake2b_state { opaque: [0u8; 384] };
    crypto_generichash_init(&mut st, core::ptr::null(), 0, NONCEBYTES);
    crypto_generichash_update(&mut st, pk1, PUBLICKEYBYTES as u64);
    crypto_generichash_update(&mut st, pk2, PUBLICKEYBYTES as u64);
    crypto_generichash_final(&mut st, nonce, NONCEBYTES);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_seal(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
) -> i32 {
    let mut nonce = [0u8; NONCEBYTES];
    let mut epk = [0u8; PUBLICKEYBYTES];
    let mut esk = [0u8; SECRETKEYBYTES];

    if mlen > MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    if crypto_box_keypair(epk.as_mut_ptr(), esk.as_mut_ptr()) != 0 {
        return -1;
    }
    _crypto_box_seal_nonce(nonce.as_mut_ptr(), epk.as_ptr(), pk);
    let ret = crypto_box_easy(c.add(PUBLICKEYBYTES), m, mlen, nonce.as_ptr(), pk, esk.as_ptr());
    core::ptr::copy_nonoverlapping(epk.as_ptr(), c, PUBLICKEYBYTES);
    sodium_memzero(esk.as_mut_ptr() as *mut c_void, SECRETKEYBYTES);
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_seal_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    pk: *const u8,
    sk: *const u8,
) -> i32 {
    let mut nonce = [0u8; NONCEBYTES];
    if clen < SEALBYTES as u64 {
        return -1;
    }
    _crypto_box_seal_nonce(nonce.as_mut_ptr(), c, pk);
    crypto_box_open_easy(
        m,
        c.add(PUBLICKEYBYTES),
        clen - PUBLICKEYBYTES as u64,
        nonce.as_ptr(),
        c,
        sk,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_sealbytes() -> usize {
    SEALBYTES
}

// ======== curve25519xchacha20poly1305 ========

const X_BEFORENMBYTES: usize = 32;
const X_MACBYTES: usize = 16;
const X_PUBLICKEYBYTES: usize = 32;
const X_SECRETKEYBYTES: usize = 32;
const X_NONCEBYTES: usize = 24;
const X_SEEDBYTES: usize = 32;
// crypto_stream_xchacha20_MESSAGEBYTES_MAX = SODIUM_SIZE_MAX
const X_MESSAGEBYTES_MAX: u64 = u64::MAX - X_MACBYTES as u64;
const X_SEALBYTES: usize = X_PUBLICKEYBYTES + X_MACBYTES;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
    let mut hash = [0u8; 64];
    crypto_hash_sha512(hash.as_mut_ptr(), seed, 32);
    core::ptr::copy_nonoverlapping(hash.as_ptr(), sk, 32);
    sodium_memzero(hash.as_mut_ptr() as *mut c_void, 64);
    crypto_scalarmult_curve25519_base(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_keypair(
    pk: *mut u8,
    sk: *mut u8,
) -> i32 {
    randombytes_buf(sk as *mut c_void, 32);
    crypto_scalarmult_curve25519_base(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_beforenm(
    k: *mut u8,
    pk: *const u8,
    sk: *const u8,
) -> i32 {
    let zero = [0u8; 16];
    let mut s = [0u8; 32];
    if crypto_scalarmult_curve25519(s.as_mut_ptr(), sk, pk) != 0 {
        return -1;
    }
    crypto_core_hchacha20(k, zero.as_ptr(), s.as_ptr(), core::ptr::null());
    sodium_memzero(s.as_mut_ptr() as *mut c_void, 32);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_detached_afternm(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    crypto_secretbox_xchacha20poly1305_detached(c, mac, m, mlen, n, k)
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
) -> i32 {
    let mut k = [0u8; X_BEFORENMBYTES];
    if crypto_box_curve25519xchacha20poly1305_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    let ret = crypto_box_curve25519xchacha20poly1305_detached_afternm(c, mac, m, mlen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, X_BEFORENMBYTES);
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_easy_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    if mlen > X_MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    crypto_box_curve25519xchacha20poly1305_detached_afternm(c.add(X_MACBYTES), c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_easy(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> i32 {
    if mlen > X_MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    crypto_box_curve25519xchacha20poly1305_detached(c.add(X_MACBYTES), c, m, mlen, n, pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_detached_afternm(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    crypto_secretbox_xchacha20poly1305_open_detached(m, c, mac, clen, n, k)
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
) -> i32 {
    let mut k = [0u8; X_BEFORENMBYTES];
    if crypto_box_curve25519xchacha20poly1305_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    let ret = crypto_box_curve25519xchacha20poly1305_open_detached_afternm(m, c, mac, clen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, X_BEFORENMBYTES);
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_easy_afternm(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    if clen < X_MACBYTES as u64 {
        return -1;
    }
    crypto_box_curve25519xchacha20poly1305_open_detached_afternm(
        m,
        c.add(X_MACBYTES),
        c,
        clen - X_MACBYTES as u64,
        n,
        k,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> i32 {
    if clen < X_MACBYTES as u64 {
        return -1;
    }
    crypto_box_curve25519xchacha20poly1305_open_detached(
        m,
        c.add(X_MACBYTES),
        c,
        clen - X_MACBYTES as u64,
        n,
        pk,
        sk,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_seedbytes() -> usize {
    X_SEEDBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_publickeybytes() -> usize {
    X_PUBLICKEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_secretkeybytes() -> usize {
    X_SECRETKEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_beforenmbytes() -> usize {
    X_BEFORENMBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_noncebytes() -> usize {
    X_NONCEBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_macbytes() -> usize {
    X_MACBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_messagebytes_max() -> usize {
    X_MESSAGEBYTES_MAX as usize
}

// ======== box_seal_curve25519xchacha20poly1305.c ========

unsafe fn _crypto_box_curve25519xchacha20poly1305_seal_nonce(
    nonce: *mut u8,
    pk1: *const u8,
    pk2: *const u8,
) -> i32 {
    let mut st = crypto_generichash_blake2b_state { opaque: [0u8; 384] };
    crypto_generichash_init(&mut st, core::ptr::null(), 0, X_NONCEBYTES);
    crypto_generichash_update(&mut st, pk1, X_PUBLICKEYBYTES as u64);
    crypto_generichash_update(&mut st, pk2, X_PUBLICKEYBYTES as u64);
    crypto_generichash_final(&mut st, nonce, X_NONCEBYTES);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seal(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
) -> i32 {
    let mut nonce = [0u8; X_NONCEBYTES];
    let mut epk = [0u8; X_PUBLICKEYBYTES];
    let mut esk = [0u8; X_SECRETKEYBYTES];

    if mlen > X_MESSAGEBYTES_MAX {
        sodium_misuse();
    }
    if crypto_box_curve25519xchacha20poly1305_keypair(epk.as_mut_ptr(), esk.as_mut_ptr()) != 0 {
        return -1;
    }
    _crypto_box_curve25519xchacha20poly1305_seal_nonce(nonce.as_mut_ptr(), epk.as_ptr(), pk);
    let ret = crypto_box_curve25519xchacha20poly1305_easy(
        c.add(X_PUBLICKEYBYTES),
        m,
        mlen,
        nonce.as_ptr(),
        pk,
        esk.as_ptr(),
    );
    core::ptr::copy_nonoverlapping(epk.as_ptr(), c, X_PUBLICKEYBYTES);
    sodium_memzero(esk.as_mut_ptr() as *mut c_void, X_SECRETKEYBYTES);
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seal_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    pk: *const u8,
    sk: *const u8,
) -> i32 {
    let mut nonce = [0u8; X_NONCEBYTES];
    if clen < X_SEALBYTES as u64 {
        return -1;
    }
    _crypto_box_curve25519xchacha20poly1305_seal_nonce(nonce.as_mut_ptr(), c, pk);
    crypto_box_curve25519xchacha20poly1305_open_easy(
        m,
        c.add(X_PUBLICKEYBYTES),
        clen - X_PUBLICKEYBYTES as u64,
        nonce.as_ptr(),
        c,
        sk,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_sealbytes() -> usize {
    X_SEALBYTES
}
