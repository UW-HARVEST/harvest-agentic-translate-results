//! Translation of c_src/libsodium/crypto_kem/xwing/kem_xwing.c

use core::ffi::{c_int, c_void};

const CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES: usize = 1184;
const CRYPTO_KEM_MLKEM768_SECRETKEYBYTES: usize = 2400;
const CRYPTO_KEM_MLKEM768_CIPHERTEXTBYTES: usize = 1088;
const CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES: usize = 32;
const CRYPTO_KEM_MLKEM768_SEEDBYTES: usize = 64;

const CRYPTO_SCALARMULT_CURVE25519_BYTES: usize = 32;
const CRYPTO_SCALARMULT_CURVE25519_SCALARBYTES: usize = 32;

const CRYPTO_KEM_XWING_PUBLICKEYBYTES: usize = 1216;
const CRYPTO_KEM_XWING_SECRETKEYBYTES: usize = 32;
const CRYPTO_KEM_XWING_CIPHERTEXTBYTES: usize = 1120;
const CRYPTO_KEM_XWING_SHAREDSECRETBYTES: usize = 32;
const CRYPTO_KEM_XWING_SEEDBYTES: usize = 32;

// crypto_hash_sha3.h: typedef struct CRYPTO_ALIGN(16) { unsigned char opaque[256]; }
#[repr(C, align(16))]
struct CryptoHashSha3256State {
    opaque: [u8; 256],
}

extern "C" {
    fn crypto_xof_shake256(out: *mut u8, outlen: usize, in_: *const u8, inlen: u64) -> c_int;

    fn crypto_kem_mlkem768_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> c_int;
    fn crypto_kem_mlkem768_enc_deterministic(
        ct: *mut u8,
        ss: *mut u8,
        pk: *const u8,
        seed: *const u8,
    ) -> c_int;
    fn crypto_kem_mlkem768_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> c_int;

    fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> c_int;
    fn crypto_scalarmult_curve25519(q: *mut u8, n: *const u8, p: *const u8) -> c_int;

    fn crypto_hash_sha3256_init(state: *mut CryptoHashSha3256State) -> c_int;
    fn crypto_hash_sha3256_update(
        state: *mut CryptoHashSha3256State,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha3256_final(state: *mut CryptoHashSha3256State, out: *mut u8) -> c_int;

    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

static xwing_label: [u8; 6] = [0x5c, 0x2e, 0x2f, 0x2f, 0x5e, 0x5c];

unsafe fn expand_decaps_key(
    pk_mlkem: *mut u8,
    sk_mlkem: *mut u8,
    pk_x25519: *mut u8,
    sk_x25519: *mut u8,
    seed: *const u8,
) {
    let mut expanded: [u8; 96] = [0; 96];
    let mut mlkem_seed: [u8; CRYPTO_KEM_MLKEM768_SEEDBYTES] = [0; CRYPTO_KEM_MLKEM768_SEEDBYTES];

    crypto_xof_shake256(
        expanded.as_mut_ptr(),
        96,
        seed,
        CRYPTO_KEM_XWING_SEEDBYTES as u64,
    );

    core::ptr::copy_nonoverlapping(expanded.as_ptr(), mlkem_seed.as_mut_ptr(), 64);
    core::ptr::copy_nonoverlapping(expanded.as_ptr().add(64), sk_x25519, 32);

    crypto_kem_mlkem768_seed_keypair(pk_mlkem, sk_mlkem, mlkem_seed.as_ptr());
    crypto_scalarmult_curve25519_base(pk_x25519, sk_x25519);

    sodium_memzero(expanded.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&expanded));
    sodium_memzero(
        mlkem_seed.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&mlkem_seed),
    );
}

unsafe fn combiner(
    ss: *mut u8,
    ss_mlkem: *const u8,
    ss_x25519: *const u8,
    ct_x25519: *const u8,
    pk_x25519: *const u8,
) {
    let mut state = CryptoHashSha3256State { opaque: [0; 256] };

    crypto_hash_sha3256_init(&mut state);
    crypto_hash_sha3256_update(&mut state, ss_mlkem, CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES as u64);
    crypto_hash_sha3256_update(&mut state, ss_x25519, CRYPTO_SCALARMULT_CURVE25519_BYTES as u64);
    crypto_hash_sha3256_update(&mut state, ct_x25519, CRYPTO_SCALARMULT_CURVE25519_BYTES as u64);
    crypto_hash_sha3256_update(&mut state, pk_x25519, CRYPTO_SCALARMULT_CURVE25519_BYTES as u64);
    crypto_hash_sha3256_update(
        &mut state,
        xwing_label.as_ptr(),
        core::mem::size_of_val(&xwing_label) as u64,
    );
    crypto_hash_sha3256_final(&mut state, ss);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_publickeybytes() -> usize {
    CRYPTO_KEM_XWING_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_secretkeybytes() -> usize {
    CRYPTO_KEM_XWING_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_ciphertextbytes() -> usize {
    CRYPTO_KEM_XWING_CIPHERTEXTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_sharedsecretbytes() -> usize {
    CRYPTO_KEM_XWING_SHAREDSECRETBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_seedbytes() -> usize {
    CRYPTO_KEM_XWING_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut pk_mlkem: [u8; CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES] =
        [0; CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES];
    let mut sk_mlkem: [u8; CRYPTO_KEM_MLKEM768_SECRETKEYBYTES] =
        [0; CRYPTO_KEM_MLKEM768_SECRETKEYBYTES];
    let mut pk_x25519: [u8; CRYPTO_SCALARMULT_CURVE25519_BYTES] =
        [0; CRYPTO_SCALARMULT_CURVE25519_BYTES];
    let mut sk_x25519: [u8; CRYPTO_SCALARMULT_CURVE25519_SCALARBYTES] =
        [0; CRYPTO_SCALARMULT_CURVE25519_SCALARBYTES];

    expand_decaps_key(
        pk_mlkem.as_mut_ptr(),
        sk_mlkem.as_mut_ptr(),
        pk_x25519.as_mut_ptr(),
        sk_x25519.as_mut_ptr(),
        seed,
    );

    core::ptr::copy_nonoverlapping(pk_mlkem.as_ptr(), pk, CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES);
    core::ptr::copy_nonoverlapping(
        pk_x25519.as_ptr(),
        pk.add(CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES),
        CRYPTO_SCALARMULT_CURVE25519_BYTES,
    );

    core::ptr::copy_nonoverlapping(seed, sk, CRYPTO_KEM_XWING_SEEDBYTES);

    sodium_memzero(
        sk_mlkem.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&sk_mlkem),
    );
    sodium_memzero(
        sk_x25519.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&sk_x25519),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed: [u8; CRYPTO_KEM_XWING_SEEDBYTES] = [0; CRYPTO_KEM_XWING_SEEDBYTES];

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, CRYPTO_KEM_XWING_SEEDBYTES);
    crypto_kem_xwing_seed_keypair(pk, sk, seed.as_ptr());

    sodium_memzero(seed.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&seed));

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
    let pk_x25519: *const u8 = pk.add(CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES);

    let seed_mlkem: *const u8 = seed;
    let sk_e_x25519: *const u8 = seed.add(32);

    let mut ct_mlkem: [u8; CRYPTO_KEM_MLKEM768_CIPHERTEXTBYTES] =
        [0; CRYPTO_KEM_MLKEM768_CIPHERTEXTBYTES];
    let mut ss_mlkem: [u8; CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES] =
        [0; CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES];
    let mut ct_x25519: [u8; CRYPTO_SCALARMULT_CURVE25519_BYTES] =
        [0; CRYPTO_SCALARMULT_CURVE25519_BYTES];
    let mut ss_x25519: [u8; CRYPTO_SCALARMULT_CURVE25519_BYTES] =
        [0; CRYPTO_SCALARMULT_CURVE25519_BYTES];

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
            core::mem::size_of_val(&ss_mlkem),
        ); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }

    core::ptr::copy_nonoverlapping(ct_mlkem.as_ptr(), ct, CRYPTO_KEM_MLKEM768_CIPHERTEXTBYTES);
    core::ptr::copy_nonoverlapping(
        ct_x25519.as_ptr(),
        ct.add(CRYPTO_KEM_MLKEM768_CIPHERTEXTBYTES),
        CRYPTO_SCALARMULT_CURVE25519_BYTES,
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
        core::mem::size_of_val(&ss_mlkem),
    );
    sodium_memzero(
        ss_x25519.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&ss_x25519),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_enc(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
) -> c_int {
    let mut seed: [u8; 64] = [0; 64];

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, 64);
    if crypto_kem_xwing_enc_deterministic(ct, ss, pk, seed.as_ptr()) != 0 {
        sodium_memzero(seed.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&seed)); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&seed));

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_dec(
    ss: *mut u8,
    ct: *const u8,
    sk: *const u8,
) -> c_int {
    let mut pk_mlkem: [u8; CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES] =
        [0; CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES];
    let mut sk_mlkem: [u8; CRYPTO_KEM_MLKEM768_SECRETKEYBYTES] =
        [0; CRYPTO_KEM_MLKEM768_SECRETKEYBYTES];
    let mut pk_x25519: [u8; CRYPTO_SCALARMULT_CURVE25519_BYTES] =
        [0; CRYPTO_SCALARMULT_CURVE25519_BYTES];
    let mut sk_x25519: [u8; CRYPTO_SCALARMULT_CURVE25519_SCALARBYTES] =
        [0; CRYPTO_SCALARMULT_CURVE25519_SCALARBYTES];

    let ct_mlkem: *const u8 = ct;
    let ct_x25519: *const u8 = ct.add(CRYPTO_KEM_MLKEM768_CIPHERTEXTBYTES);

    let mut ss_mlkem: [u8; CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES] =
        [0; CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES];
    let mut ss_x25519: [u8; CRYPTO_SCALARMULT_CURVE25519_BYTES] =
        [0; CRYPTO_SCALARMULT_CURVE25519_BYTES];

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
            core::mem::size_of_val(&sk_mlkem),
        );
        sodium_memzero(
            sk_x25519.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&sk_x25519),
        );
        return -1;
    }

    if crypto_scalarmult_curve25519(ss_x25519.as_mut_ptr(), sk_x25519.as_ptr(), ct_x25519) != 0 {
        sodium_memzero(
            ss_mlkem.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&ss_mlkem),
        );
        sodium_memzero(
            sk_mlkem.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&sk_mlkem),
        );
        sodium_memzero(
            sk_x25519.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&sk_x25519),
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
        core::mem::size_of_val(&ss_mlkem),
    );
    sodium_memzero(
        ss_x25519.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&ss_x25519),
    );
    sodium_memzero(
        sk_mlkem.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&sk_mlkem),
    );
    sodium_memzero(
        sk_x25519.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&sk_x25519),
    );

    0
}
