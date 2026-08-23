//! Translation of `c_src/libsodium/crypto_kem/xwing/kem_xwing.c`.
//!
//! X-Wing = ML-KEM-768 + X25519 hybrid KEM.
//! Constants come from `include/sodium/crypto_kem_xwing.h`,
//! `include/sodium/crypto_kem_mlkem768.h` and
//! `include/sodium/crypto_scalarmult_curve25519.h`.

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/* crypto_kem_xwing.h */
const crypto_kem_xwing_PUBLICKEYBYTES: usize = 1216;
const crypto_kem_xwing_SECRETKEYBYTES: usize = 32;
const crypto_kem_xwing_CIPHERTEXTBYTES: usize = 1120;
const crypto_kem_xwing_SHAREDSECRETBYTES: usize = 32;
const crypto_kem_xwing_SEEDBYTES: usize = 32;

/* crypto_kem_mlkem768.h */
const crypto_kem_mlkem768_PUBLICKEYBYTES: usize = 1184;
const crypto_kem_mlkem768_SECRETKEYBYTES: usize = 2400;
const crypto_kem_mlkem768_CIPHERTEXTBYTES: usize = 1088;
const crypto_kem_mlkem768_SHAREDSECRETBYTES: usize = 32;
const crypto_kem_mlkem768_SEEDBYTES: usize = 64;

/* crypto_scalarmult_curve25519.h */
const crypto_scalarmult_curve25519_BYTES: usize = 32;
const crypto_scalarmult_curve25519_SCALARBYTES: usize = 32;

/// `crypto_hash_sha3256_state` from `include/sodium/crypto_hash_sha3.h`.
#[repr(C, align(16))]
struct crypto_hash_sha3256_state {
    opaque: [u8; 256],
}

extern "C" {
    /* crypto_hash/sha3/hash_sha3.c */
    fn crypto_hash_sha3256_init(state: *mut crypto_hash_sha3256_state) -> c_int;
    fn crypto_hash_sha3256_update(
        state: *mut crypto_hash_sha3256_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_hash_sha3256_final(state: *mut crypto_hash_sha3256_state, out: *mut u8) -> c_int;

    /* crypto_xof/shake/xof_shake256.c */
    fn crypto_xof_shake256(
        out: *mut u8,
        outlen: usize,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;

    /* crypto_kem/mlkem768/kem_mlkem768.c */
    fn crypto_kem_mlkem768_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> c_int;
    fn crypto_kem_mlkem768_enc_deterministic(
        ct: *mut u8,
        ss: *mut u8,
        pk: *const u8,
        seed: *const u8,
    ) -> c_int;
    fn crypto_kem_mlkem768_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> c_int;

    /* crypto_scalarmult/curve25519/scalarmult_curve25519.c */
    fn crypto_scalarmult_curve25519(q: *mut u8, n: *const u8, p: *const u8) -> c_int;
    fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> c_int;

    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);

    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

/// `static const unsigned char xwing_label[6] = { 0x5c, 0x2e, 0x2f, 0x2f, 0x5e, 0x5c };`
static xwing_label: [u8; 6] = [0x5c, 0x2e, 0x2f, 0x2f, 0x5e, 0x5c];

/// ```c
/// static void expand_decaps_key(unsigned char pk_mlkem[...], unsigned char sk_mlkem[...],
///                               unsigned char pk_x25519[...], unsigned char sk_x25519[...],
///                               const unsigned char seed[crypto_kem_xwing_SEEDBYTES]);
/// ```
unsafe fn expand_decaps_key(
    pk_mlkem: *mut u8,
    sk_mlkem: *mut u8,
    pk_x25519: *mut u8,
    sk_x25519: *mut u8,
    seed: *const u8,
) {
    let mut expanded = [0u8; 96];
    let mut mlkem_seed = [0u8; crypto_kem_mlkem768_SEEDBYTES];

    crypto_xof_shake256(
        expanded.as_mut_ptr(),
        96,
        seed,
        crypto_kem_xwing_SEEDBYTES as c_ulonglong,
    );

    memcpy(mlkem_seed.as_mut_ptr(), expanded.as_ptr(), 64);
    memcpy(sk_x25519, expanded.as_ptr().add(64), 32);

    crypto_kem_mlkem768_seed_keypair(pk_mlkem, sk_mlkem, mlkem_seed.as_ptr());
    crypto_scalarmult_curve25519_base(pk_x25519, sk_x25519);

    sodium_memzero(expanded.as_mut_ptr() as *mut c_void, 96);
    sodium_memzero(
        mlkem_seed.as_mut_ptr() as *mut c_void,
        crypto_kem_mlkem768_SEEDBYTES,
    );
}

/// ```c
/// static void combiner(unsigned char ss[...], const unsigned char ss_mlkem[...],
///                      const unsigned char ss_x25519[...], const unsigned char ct_x25519[...],
///                      const unsigned char pk_x25519[...]);
/// ```
unsafe fn combiner(
    ss: *mut u8,
    ss_mlkem: *const u8,
    ss_x25519: *const u8,
    ct_x25519: *const u8,
    pk_x25519: *const u8,
) {
    let mut state = crypto_hash_sha3256_state { opaque: [0u8; 256] };

    crypto_hash_sha3256_init(&mut state);
    crypto_hash_sha3256_update(
        &mut state,
        ss_mlkem,
        crypto_kem_mlkem768_SHAREDSECRETBYTES as c_ulonglong,
    );
    crypto_hash_sha3256_update(
        &mut state,
        ss_x25519,
        crypto_scalarmult_curve25519_BYTES as c_ulonglong,
    );
    crypto_hash_sha3256_update(
        &mut state,
        ct_x25519,
        crypto_scalarmult_curve25519_BYTES as c_ulonglong,
    );
    crypto_hash_sha3256_update(
        &mut state,
        pk_x25519,
        crypto_scalarmult_curve25519_BYTES as c_ulonglong,
    );
    crypto_hash_sha3256_update(&mut state, xwing_label.as_ptr(), 6);
    crypto_hash_sha3256_final(&mut state, ss);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_publickeybytes() -> usize {
    crypto_kem_xwing_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_secretkeybytes() -> usize {
    crypto_kem_xwing_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_ciphertextbytes() -> usize {
    crypto_kem_xwing_CIPHERTEXTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_sharedsecretbytes() -> usize {
    crypto_kem_xwing_SHAREDSECRETBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_seedbytes() -> usize {
    crypto_kem_xwing_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut pk_mlkem = [0u8; crypto_kem_mlkem768_PUBLICKEYBYTES];
    let mut sk_mlkem = [0u8; crypto_kem_mlkem768_SECRETKEYBYTES];
    let mut pk_x25519 = [0u8; crypto_scalarmult_curve25519_BYTES];
    let mut sk_x25519 = [0u8; crypto_scalarmult_curve25519_SCALARBYTES];

    expand_decaps_key(
        pk_mlkem.as_mut_ptr(),
        sk_mlkem.as_mut_ptr(),
        pk_x25519.as_mut_ptr(),
        sk_x25519.as_mut_ptr(),
        seed,
    );

    memcpy(pk, pk_mlkem.as_ptr(), crypto_kem_mlkem768_PUBLICKEYBYTES);
    memcpy(
        pk.add(crypto_kem_mlkem768_PUBLICKEYBYTES),
        pk_x25519.as_ptr(),
        crypto_scalarmult_curve25519_BYTES,
    );

    memcpy(sk, seed, crypto_kem_xwing_SEEDBYTES);

    sodium_memzero(
        sk_mlkem.as_mut_ptr() as *mut c_void,
        crypto_kem_mlkem768_SECRETKEYBYTES,
    );
    sodium_memzero(
        sk_x25519.as_mut_ptr() as *mut c_void,
        crypto_scalarmult_curve25519_SCALARBYTES,
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed = [0u8; crypto_kem_xwing_SEEDBYTES];

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, crypto_kem_xwing_SEEDBYTES);
    crypto_kem_xwing_seed_keypair(pk, sk, seed.as_ptr());

    sodium_memzero(seed.as_mut_ptr() as *mut c_void, crypto_kem_xwing_SEEDBYTES);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_enc_deterministic(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
    seed: *const u8,
) -> c_int {
    let pk_mlkem: *const u8 = pk;
    let pk_x25519: *const u8 = pk.add(crypto_kem_mlkem768_PUBLICKEYBYTES);

    let seed_mlkem: *const u8 = seed;
    let sk_e_x25519: *const u8 = seed.add(32);

    let mut ct_mlkem = [0u8; crypto_kem_mlkem768_CIPHERTEXTBYTES];
    let mut ss_mlkem = [0u8; crypto_kem_mlkem768_SHAREDSECRETBYTES];
    let mut ct_x25519 = [0u8; crypto_scalarmult_curve25519_BYTES];
    let mut ss_x25519 = [0u8; crypto_scalarmult_curve25519_BYTES];

    if crypto_kem_mlkem768_enc_deterministic(
        ct_mlkem.as_mut_ptr(),
        ss_mlkem.as_mut_ptr(),
        pk_mlkem,
        seed_mlkem,
    ) != 0
    {
        return -1; /* LCOV_EXCL_LINE */
    }

    crypto_scalarmult_curve25519_base(ct_x25519.as_mut_ptr(), sk_e_x25519);

    if crypto_scalarmult_curve25519(ss_x25519.as_mut_ptr(), sk_e_x25519, pk_x25519) != 0 {
        sodium_memzero(
            ss_mlkem.as_mut_ptr() as *mut c_void,
            crypto_kem_mlkem768_SHAREDSECRETBYTES,
        ); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }

    memcpy(ct, ct_mlkem.as_ptr(), crypto_kem_mlkem768_CIPHERTEXTBYTES);
    memcpy(
        ct.add(crypto_kem_mlkem768_CIPHERTEXTBYTES),
        ct_x25519.as_ptr(),
        crypto_scalarmult_curve25519_BYTES,
    );

    combiner(
        ss,
        ss_mlkem.as_ptr(),
        ss_x25519.as_ptr(),
        ct_x25519.as_ptr(),
        pk_x25519,
    );

    sodium_memzero(
        ss_mlkem.as_mut_ptr() as *mut c_void,
        crypto_kem_mlkem768_SHAREDSECRETBYTES,
    );
    sodium_memzero(
        ss_x25519.as_mut_ptr() as *mut c_void,
        crypto_scalarmult_curve25519_BYTES,
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_enc(ct: *mut u8, ss: *mut u8, pk: *const u8) -> c_int {
    let mut seed = [0u8; 64];

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, 64);
    if crypto_kem_xwing_enc_deterministic(ct, ss, pk, seed.as_ptr()) != 0 {
        sodium_memzero(seed.as_mut_ptr() as *mut c_void, 64); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, 64);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> c_int {
    let mut pk_mlkem = [0u8; crypto_kem_mlkem768_PUBLICKEYBYTES];
    let mut sk_mlkem = [0u8; crypto_kem_mlkem768_SECRETKEYBYTES];
    let mut pk_x25519 = [0u8; crypto_scalarmult_curve25519_BYTES];
    let mut sk_x25519 = [0u8; crypto_scalarmult_curve25519_SCALARBYTES];

    let ct_mlkem: *const u8 = ct;
    let ct_x25519: *const u8 = ct.add(crypto_kem_mlkem768_CIPHERTEXTBYTES);

    let mut ss_mlkem = [0u8; crypto_kem_mlkem768_SHAREDSECRETBYTES];
    let mut ss_x25519 = [0u8; crypto_scalarmult_curve25519_BYTES];

    expand_decaps_key(
        pk_mlkem.as_mut_ptr(),
        sk_mlkem.as_mut_ptr(),
        pk_x25519.as_mut_ptr(),
        sk_x25519.as_mut_ptr(),
        sk,
    );

    /* LCOV_EXCL_START */
    if crypto_kem_mlkem768_dec(ss_mlkem.as_mut_ptr(), ct_mlkem, sk_mlkem.as_ptr()) != 0 {
        sodium_memzero(
            sk_mlkem.as_mut_ptr() as *mut c_void,
            crypto_kem_mlkem768_SECRETKEYBYTES,
        );
        sodium_memzero(
            sk_x25519.as_mut_ptr() as *mut c_void,
            crypto_scalarmult_curve25519_SCALARBYTES,
        );
        return -1;
    }

    if crypto_scalarmult_curve25519(ss_x25519.as_mut_ptr(), sk_x25519.as_ptr(), ct_x25519) != 0 {
        sodium_memzero(
            ss_mlkem.as_mut_ptr() as *mut c_void,
            crypto_kem_mlkem768_SHAREDSECRETBYTES,
        );
        sodium_memzero(
            sk_mlkem.as_mut_ptr() as *mut c_void,
            crypto_kem_mlkem768_SECRETKEYBYTES,
        );
        sodium_memzero(
            sk_x25519.as_mut_ptr() as *mut c_void,
            crypto_scalarmult_curve25519_SCALARBYTES,
        );
        return -1;
    }
    /* LCOV_EXCL_STOP */

    combiner(
        ss,
        ss_mlkem.as_ptr(),
        ss_x25519.as_ptr(),
        ct_x25519,
        pk_x25519.as_ptr(),
    );

    sodium_memzero(
        ss_mlkem.as_mut_ptr() as *mut c_void,
        crypto_kem_mlkem768_SHAREDSECRETBYTES,
    );
    sodium_memzero(
        ss_x25519.as_mut_ptr() as *mut c_void,
        crypto_scalarmult_curve25519_BYTES,
    );
    sodium_memzero(
        sk_mlkem.as_mut_ptr() as *mut c_void,
        crypto_kem_mlkem768_SECRETKEYBYTES,
    );
    sodium_memzero(
        sk_x25519.as_mut_ptr() as *mut c_void,
        crypto_scalarmult_curve25519_SCALARBYTES,
    );

    0
}
