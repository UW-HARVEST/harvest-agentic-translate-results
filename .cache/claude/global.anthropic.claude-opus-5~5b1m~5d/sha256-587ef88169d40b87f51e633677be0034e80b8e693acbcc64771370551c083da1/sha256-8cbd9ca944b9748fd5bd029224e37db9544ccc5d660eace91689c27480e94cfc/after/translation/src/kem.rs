//! Translation of `crypto_kem/crypto_kem.c`, `crypto_kem/mlkem768/kem_mlkem768.c`,
//! and `crypto_kem/xwing/kem_xwing.c`.
//!
//! `crypto_kem_*` dispatches to the xwing hybrid KEM (the only registered
//! primitive). `crypto_kem_mlkem768_*` wraps the ML-KEM-768 reference
//! implementation (`src/kem_mlkem768_ref.rs`, linked as `_sodium_mlkem768_ref_*`).
//! `crypto_kem_xwing_*` implements the X-Wing combiner on top of ML-KEM-768 and
//! X25519.

use core::ffi::{c_char, c_int, c_void};

extern "C" {
    // ---- crypto_kem/mlkem768/ref (src/kem_mlkem768_ref.rs) ----
    #[link_name = "_sodium_mlkem768_ref_keypair"]
    fn mlkem768_ref_keypair(pk: *mut u8, sk: *mut u8) -> c_int;
    #[link_name = "_sodium_mlkem768_ref_seed_keypair"]
    fn mlkem768_ref_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> c_int;
    #[link_name = "_sodium_mlkem768_ref_enc"]
    fn mlkem768_ref_enc(ct: *mut u8, ss: *mut u8, pk: *const u8) -> c_int;
    #[link_name = "_sodium_mlkem768_ref_enc_deterministic"]
    fn mlkem768_ref_enc_deterministic(
        ct: *mut u8,
        ss: *mut u8,
        pk: *const u8,
        seed: *const u8,
    ) -> c_int;
    #[link_name = "_sodium_mlkem768_ref_dec"]
    fn mlkem768_ref_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> c_int;

    // ---- crypto_scalarmult/curve25519 ----
    fn crypto_scalarmult_curve25519(q: *mut u8, n: *const u8, p: *const u8) -> c_int;
    fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> c_int;

    // ---- crypto_xof/shake256 ----
    fn crypto_xof_shake256(out: *mut u8, outlen: usize, inp: *const u8, inlen: u64) -> c_int;

    // ---- crypto_hash/sha3 ----
    fn crypto_hash_sha3256_init(state: *mut crypto_hash_sha3256_state) -> c_int;
    fn crypto_hash_sha3256_update(
        state: *mut crypto_hash_sha3256_state,
        inp: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha3256_final(state: *mut crypto_hash_sha3256_state, out: *mut u8) -> c_int;

    // ---- sodium/utils ----
    fn sodium_memzero(pnt: *mut c_void, len: usize);

    // ---- randombytes ----
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

#[repr(C, align(16))]
struct crypto_hash_sha3256_state {
    opaque: [u8; 256],
}

// ===========================================================================
// crypto_kem/crypto_kem.c
//
// Dispatches to xwing, the only registered KEM primitive.
// ===========================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_publickeybytes() -> usize {
    1216
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_secretkeybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_ciphertextbytes() -> usize {
    1120
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_sharedsecretbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_seedbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_primitive() -> *const c_char {
    b"xwing\0".as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    crypto_kem_xwing_seed_keypair(pk, sk, seed)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    crypto_kem_xwing_keypair(pk, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_enc(ct: *mut u8, ss: *mut u8, pk: *const u8) -> c_int {
    crypto_kem_xwing_enc(ct, ss, pk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> c_int {
    crypto_kem_xwing_dec(ss, ct, sk)
}

// ===========================================================================
// crypto_kem/mlkem768/kem_mlkem768.c
//
// Thin wrapper around the `ref` backend.
// ===========================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_mlkem768_publickeybytes() -> usize {
    1184
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_mlkem768_secretkeybytes() -> usize {
    2400
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_mlkem768_ciphertextbytes() -> usize {
    1088
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_mlkem768_sharedsecretbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_mlkem768_seedbytes() -> usize {
    64
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_mlkem768_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    mlkem768_ref_seed_keypair(pk, sk, seed)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_mlkem768_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    mlkem768_ref_keypair(pk, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_mlkem768_enc(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
) -> c_int {
    mlkem768_ref_enc(ct, ss, pk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_mlkem768_enc_deterministic(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
    seed: *const u8,
) -> c_int {
    mlkem768_ref_enc_deterministic(ct, ss, pk, seed)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_mlkem768_dec(
    ss: *mut u8,
    ct: *const u8,
    sk: *const u8,
) -> c_int {
    mlkem768_ref_dec(ss, ct, sk)
}

// ===========================================================================
// crypto_kem/xwing/kem_xwing.c
// ===========================================================================

static XWING_LABEL: [u8; 6] = [0x5c, 0x2e, 0x2f, 0x2f, 0x5e, 0x5c];

unsafe fn expand_decaps_key(
    pk_mlkem: *mut u8,
    sk_mlkem: *mut u8,
    pk_x25519: *mut u8,
    sk_x25519: *mut u8,
    seed: *const u8,
) {
    let mut expanded = [0u8; 96];
    let mut mlkem_seed = [0u8; 64];

    crypto_xof_shake256(expanded.as_mut_ptr(), 96, seed, 32);

    core::ptr::copy_nonoverlapping(expanded.as_ptr(), mlkem_seed.as_mut_ptr(), 64);
    core::ptr::copy_nonoverlapping(expanded.as_ptr().add(64), sk_x25519, 32);

    crypto_kem_mlkem768_seed_keypair(pk_mlkem, sk_mlkem, mlkem_seed.as_ptr());
    crypto_scalarmult_curve25519_base(pk_x25519, sk_x25519);

    sodium_memzero(expanded.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&expanded));
    sodium_memzero(mlkem_seed.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&mlkem_seed));
}

unsafe fn combiner(
    ss: *mut u8,
    ss_mlkem: *const u8,
    ss_x25519: *const u8,
    ct_x25519: *const u8,
    pk_x25519: *const u8,
) {
    let mut state = crypto_hash_sha3256_state { opaque: [0u8; 256] };

    crypto_hash_sha3256_init(&mut state);
    crypto_hash_sha3256_update(&mut state, ss_mlkem, 32);
    crypto_hash_sha3256_update(&mut state, ss_x25519, 32);
    crypto_hash_sha3256_update(&mut state, ct_x25519, 32);
    crypto_hash_sha3256_update(&mut state, pk_x25519, 32);
    crypto_hash_sha3256_update(&mut state, XWING_LABEL.as_ptr(), XWING_LABEL.len() as u64);
    crypto_hash_sha3256_final(&mut state, ss);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_xwing_publickeybytes() -> usize {
    1216
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_xwing_secretkeybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_xwing_ciphertextbytes() -> usize {
    1120
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_xwing_sharedsecretbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_xwing_seedbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_xwing_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut pk_mlkem = [0u8; 1184];
    let mut sk_mlkem = [0u8; 2400];
    let mut pk_x25519 = [0u8; 32];
    let mut sk_x25519 = [0u8; 32];

    expand_decaps_key(
        pk_mlkem.as_mut_ptr(),
        sk_mlkem.as_mut_ptr(),
        pk_x25519.as_mut_ptr(),
        sk_x25519.as_mut_ptr(),
        seed,
    );

    core::ptr::copy_nonoverlapping(pk_mlkem.as_ptr(), pk, 1184);
    core::ptr::copy_nonoverlapping(pk_x25519.as_ptr(), pk.add(1184), 32);

    core::ptr::copy_nonoverlapping(seed, sk, 32);

    sodium_memzero(sk_mlkem.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&sk_mlkem));
    sodium_memzero(sk_x25519.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&sk_x25519));

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_xwing_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed = [0u8; 32];

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, 32);
    crypto_kem_xwing_seed_keypair(pk, sk, seed.as_ptr());

    sodium_memzero(seed.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&seed));

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_xwing_enc_deterministic(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
    seed: *const u8,
) -> c_int {
    let pk_mlkem = pk;
    let pk_x25519 = pk.add(1184);

    let seed_mlkem = seed;
    let sk_e_x25519 = seed.add(32);

    let mut ct_mlkem = [0u8; 1088];
    let mut ss_mlkem = [0u8; 32];
    let mut ct_x25519 = [0u8; 32];
    let mut ss_x25519 = [0u8; 32];

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
        sodium_memzero(ss_mlkem.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&ss_mlkem));
        return -1;
    }

    core::ptr::copy_nonoverlapping(ct_mlkem.as_ptr(), ct, 1088);
    core::ptr::copy_nonoverlapping(ct_x25519.as_ptr(), ct.add(1088), 32);

    combiner(
        ss,
        ss_mlkem.as_ptr(),
        ss_x25519.as_ptr(),
        ct_x25519.as_ptr(),
        pk_x25519,
    );

    sodium_memzero(ss_mlkem.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&ss_mlkem));
    sodium_memzero(ss_x25519.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&ss_x25519));

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_xwing_enc(ct: *mut u8, ss: *mut u8, pk: *const u8) -> c_int {
    let mut seed = [0u8; 64];

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, 64);
    if crypto_kem_xwing_enc_deterministic(ct, ss, pk, seed.as_ptr()) != 0 {
        sodium_memzero(seed.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&seed));
        return -1;
    }
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&seed));

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kem_xwing_dec(
    ss: *mut u8,
    ct: *const u8,
    sk: *const u8,
) -> c_int {
    let mut pk_mlkem = [0u8; 1184];
    let mut sk_mlkem = [0u8; 2400];
    let mut pk_x25519 = [0u8; 32];
    let mut sk_x25519 = [0u8; 32];

    let ct_mlkem = ct;
    let ct_x25519 = ct.add(1088);

    let mut ss_mlkem = [0u8; 32];
    let mut ss_x25519 = [0u8; 32];

    expand_decaps_key(
        pk_mlkem.as_mut_ptr(),
        sk_mlkem.as_mut_ptr(),
        pk_x25519.as_mut_ptr(),
        sk_x25519.as_mut_ptr(),
        sk,
    );

    if crypto_kem_mlkem768_dec(ss_mlkem.as_mut_ptr(), ct_mlkem, sk_mlkem.as_ptr()) != 0 {
        sodium_memzero(sk_mlkem.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&sk_mlkem));
        sodium_memzero(sk_x25519.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&sk_x25519));
        return -1;
    }

    if crypto_scalarmult_curve25519(ss_x25519.as_mut_ptr(), sk_x25519.as_ptr(), ct_x25519) != 0 {
        sodium_memzero(ss_mlkem.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&ss_mlkem));
        sodium_memzero(sk_mlkem.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&sk_mlkem));
        sodium_memzero(sk_x25519.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&sk_x25519));
        return -1;
    }

    combiner(
        ss,
        ss_mlkem.as_ptr(),
        ss_x25519.as_ptr(),
        ct_x25519,
        pk_x25519.as_ptr(),
    );

    sodium_memzero(ss_mlkem.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&ss_mlkem));
    sodium_memzero(ss_x25519.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&ss_x25519));
    sodium_memzero(sk_mlkem.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&sk_mlkem));
    sodium_memzero(sk_x25519.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&sk_x25519));

    0
}
