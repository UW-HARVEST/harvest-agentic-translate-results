//! Translation of crypto_kem/xwing/kem_xwing.c (+ crypto_kem_xwing.h).

use core::ffi::c_void;

use crate::crypto_hash::sha3::{
    crypto_hash_sha3256_final, crypto_hash_sha3256_init, crypto_hash_sha3256_state,
    crypto_hash_sha3256_update,
};
use crate::crypto_kem::mlkem768::{
    crypto_kem_mlkem768_dec, crypto_kem_mlkem768_enc_deterministic, crypto_kem_mlkem768_seed_keypair,
};
use crate::crypto_scalarmult::curve25519::{
    crypto_scalarmult_curve25519, crypto_scalarmult_curve25519_base,
};
use crate::crypto_xof::shake256::crypto_xof_shake256;
use crate::randombytes::randombytes_buf;
use crate::sodium_utils::sodium_memzero;

/* ---- public API sizes ---- */

const crypto_kem_xwing_PUBLICKEYBYTES: usize = 1216;
const crypto_kem_xwing_SECRETKEYBYTES: usize = 32;
const crypto_kem_xwing_CIPHERTEXTBYTES: usize = 1120;
const crypto_kem_xwing_SHAREDSECRETBYTES: usize = 32;
const crypto_kem_xwing_SEEDBYTES: usize = 32;

const crypto_kem_mlkem768_PUBLICKEYBYTES: usize = 1184;
const crypto_kem_mlkem768_SECRETKEYBYTES: usize = 2400;
const crypto_kem_mlkem768_CIPHERTEXTBYTES: usize = 1088;
const crypto_kem_mlkem768_SHAREDSECRETBYTES: usize = 32;

const crypto_scalarmult_curve25519_BYTES: usize = 32;
const crypto_scalarmult_curve25519_SCALARBYTES: usize = 32;

static xwing_label: [u8; 6] = [0x5c, 0x2e, 0x2f, 0x2f, 0x5e, 0x5c];

unsafe fn expand_decaps_key(
    pk_mlkem: *mut u8,
    sk_mlkem: *mut u8,
    pk_x25519: *mut u8,
    sk_x25519: *mut u8,
    seed: *const u8,
) {
    let mut expanded = [0u8; 96];
    let mut mlkem_seed = [0u8; 64];

    crypto_xof_shake256(
        expanded.as_mut_ptr(),
        96,
        seed,
        crypto_kem_xwing_SEEDBYTES as u64,
    );

    core::ptr::copy_nonoverlapping(expanded.as_ptr(), mlkem_seed.as_mut_ptr(), 64);
    core::ptr::copy_nonoverlapping(expanded.as_ptr().add(64), sk_x25519, 32);

    crypto_kem_mlkem768_seed_keypair(pk_mlkem, sk_mlkem, mlkem_seed.as_ptr());
    crypto_scalarmult_curve25519_base(pk_x25519, sk_x25519);

    sodium_memzero(expanded.as_mut_ptr() as *mut c_void, expanded.len());
    sodium_memzero(mlkem_seed.as_mut_ptr() as *mut c_void, mlkem_seed.len());
}

unsafe fn combiner(
    ss: *mut u8,
    ss_mlkem: *const u8,
    ss_x25519: *const u8,
    ct_x25519: *const u8,
    pk_x25519: *const u8,
) {
    let mut state: crypto_hash_sha3256_state = core::mem::zeroed();

    crypto_hash_sha3256_init(&mut state);
    crypto_hash_sha3256_update(
        &mut state,
        ss_mlkem,
        crypto_kem_mlkem768_SHAREDSECRETBYTES as u64,
    );
    crypto_hash_sha3256_update(
        &mut state,
        ss_x25519,
        crypto_scalarmult_curve25519_BYTES as u64,
    );
    crypto_hash_sha3256_update(
        &mut state,
        ct_x25519,
        crypto_scalarmult_curve25519_BYTES as u64,
    );
    crypto_hash_sha3256_update(
        &mut state,
        pk_x25519,
        crypto_scalarmult_curve25519_BYTES as u64,
    );
    crypto_hash_sha3256_update(&mut state, xwing_label.as_ptr(), xwing_label.len() as u64);
    crypto_hash_sha3256_final(&mut state, ss);
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_xwing_publickeybytes() -> usize {
    crypto_kem_xwing_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_xwing_secretkeybytes() -> usize {
    crypto_kem_xwing_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_xwing_ciphertextbytes() -> usize {
    crypto_kem_xwing_CIPHERTEXTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_xwing_sharedsecretbytes() -> usize {
    crypto_kem_xwing_SHAREDSECRETBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_xwing_seedbytes() -> usize {
    crypto_kem_xwing_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
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

    core::ptr::copy_nonoverlapping(pk_mlkem.as_ptr(), pk, crypto_kem_mlkem768_PUBLICKEYBYTES);
    core::ptr::copy_nonoverlapping(
        pk_x25519.as_ptr(),
        pk.add(crypto_kem_mlkem768_PUBLICKEYBYTES),
        crypto_scalarmult_curve25519_BYTES,
    );

    core::ptr::copy_nonoverlapping(seed, sk, crypto_kem_xwing_SEEDBYTES);

    sodium_memzero(sk_mlkem.as_mut_ptr() as *mut c_void, sk_mlkem.len());
    sodium_memzero(sk_x25519.as_mut_ptr() as *mut c_void, sk_x25519.len());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let mut seed = [0u8; crypto_kem_xwing_SEEDBYTES];

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, crypto_kem_xwing_SEEDBYTES);
    crypto_kem_xwing_seed_keypair(pk, sk, seed.as_ptr());

    sodium_memzero(seed.as_mut_ptr() as *mut c_void, seed.len());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_enc_deterministic(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
    seed: *const u8,
) -> i32 {
    let pk_mlkem = pk;
    let pk_x25519 = pk.add(crypto_kem_mlkem768_PUBLICKEYBYTES);

    let seed_mlkem = seed;
    let sk_e_x25519 = seed.add(32);

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
        return -1;
    }

    crypto_scalarmult_curve25519_base(ct_x25519.as_mut_ptr(), sk_e_x25519);

    if crypto_scalarmult_curve25519(ss_x25519.as_mut_ptr(), sk_e_x25519, pk_x25519) != 0 {
        sodium_memzero(ss_mlkem.as_mut_ptr() as *mut c_void, ss_mlkem.len());
        return -1;
    }

    core::ptr::copy_nonoverlapping(
        ct_mlkem.as_ptr(),
        ct,
        crypto_kem_mlkem768_CIPHERTEXTBYTES,
    );
    core::ptr::copy_nonoverlapping(
        ct_x25519.as_ptr(),
        ct.add(crypto_kem_mlkem768_CIPHERTEXTBYTES),
        crypto_scalarmult_curve25519_BYTES,
    );

    combiner(
        ss,
        ss_mlkem.as_ptr(),
        ss_x25519.as_ptr(),
        ct_x25519.as_ptr(),
        pk_x25519,
    );

    sodium_memzero(ss_mlkem.as_mut_ptr() as *mut c_void, ss_mlkem.len());
    sodium_memzero(ss_x25519.as_mut_ptr() as *mut c_void, ss_x25519.len());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_enc(ct: *mut u8, ss: *mut u8, pk: *const u8) -> i32 {
    let mut seed = [0u8; 64];

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, 64);
    if crypto_kem_xwing_enc_deterministic(ct, ss, pk, seed.as_ptr()) != 0 {
        sodium_memzero(seed.as_mut_ptr() as *mut c_void, seed.len());
        return -1;
    }
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, seed.len());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> i32 {
    let mut pk_mlkem = [0u8; crypto_kem_mlkem768_PUBLICKEYBYTES];
    let mut sk_mlkem = [0u8; crypto_kem_mlkem768_SECRETKEYBYTES];
    let mut pk_x25519 = [0u8; crypto_scalarmult_curve25519_BYTES];
    let mut sk_x25519 = [0u8; crypto_scalarmult_curve25519_SCALARBYTES];

    let ct_mlkem = ct;
    let ct_x25519 = ct.add(crypto_kem_mlkem768_CIPHERTEXTBYTES);

    let mut ss_mlkem = [0u8; crypto_kem_mlkem768_SHAREDSECRETBYTES];
    let mut ss_x25519 = [0u8; crypto_scalarmult_curve25519_BYTES];

    expand_decaps_key(
        pk_mlkem.as_mut_ptr(),
        sk_mlkem.as_mut_ptr(),
        pk_x25519.as_mut_ptr(),
        sk_x25519.as_mut_ptr(),
        sk,
    );

    if crypto_kem_mlkem768_dec(ss_mlkem.as_mut_ptr(), ct_mlkem, sk_mlkem.as_ptr()) != 0 {
        sodium_memzero(sk_mlkem.as_mut_ptr() as *mut c_void, sk_mlkem.len());
        sodium_memzero(sk_x25519.as_mut_ptr() as *mut c_void, sk_x25519.len());
        return -1;
    }

    if crypto_scalarmult_curve25519(ss_x25519.as_mut_ptr(), sk_x25519.as_ptr(), ct_x25519) != 0 {
        sodium_memzero(ss_mlkem.as_mut_ptr() as *mut c_void, ss_mlkem.len());
        sodium_memzero(sk_mlkem.as_mut_ptr() as *mut c_void, sk_mlkem.len());
        sodium_memzero(sk_x25519.as_mut_ptr() as *mut c_void, sk_x25519.len());
        return -1;
    }

    combiner(
        ss,
        ss_mlkem.as_ptr(),
        ss_x25519.as_ptr(),
        ct_x25519,
        pk_x25519.as_ptr(),
    );

    sodium_memzero(ss_mlkem.as_mut_ptr() as *mut c_void, ss_mlkem.len());
    sodium_memzero(ss_x25519.as_mut_ptr() as *mut c_void, ss_x25519.len());
    sodium_memzero(sk_mlkem.as_mut_ptr() as *mut c_void, sk_mlkem.len());
    sodium_memzero(sk_x25519.as_mut_ptr() as *mut c_void, sk_x25519.len());

    0
}
