// Translation of crypto_kem/crypto_kem.c and crypto_kem/xwing/kem_xwing.c

use core::ffi::{c_char, c_int, c_void};

// xwing constants
const XWING_PUBLICKEYBYTES: usize = 1216;
const XWING_SECRETKEYBYTES: usize = 32;
const XWING_CIPHERTEXTBYTES: usize = 1120;
const XWING_SHAREDSECRETBYTES: usize = 32;
const XWING_SEEDBYTES: usize = 32;

const MLKEM768_PUBLICKEYBYTES: usize = 1184;
const MLKEM768_SECRETKEYBYTES: usize = 2400;
const MLKEM768_CIPHERTEXTBYTES: usize = 1088;
const MLKEM768_SHAREDSECRETBYTES: usize = 32;
const MLKEM768_SEEDBYTES: usize = 64;

const CURVE25519_BYTES: usize = 32;
const CURVE25519_SCALARBYTES: usize = 32;

const CRYPTO_KEM_PRIMITIVE: &[u8] = b"xwing\0";

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn crypto_xof_shake256(out: *mut u8, outlen: usize, inp: *const u8, inlen: u64) -> c_int;
    fn crypto_scalarmult_curve25519(q: *mut u8, n: *const u8, p: *const u8) -> c_int;
    fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> c_int;
    fn crypto_hash_sha3256_init(state: *mut c_void) -> c_int;
    fn crypto_hash_sha3256_update(state: *mut c_void, inp: *const u8, inlen: u64) -> c_int;
    fn crypto_hash_sha3256_final(state: *mut c_void, out: *mut u8) -> c_int;

    // mlkem768 ref, defined in this package (mlkem.rs)
    fn crypto_kem_mlkem768_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> c_int;
    fn crypto_kem_mlkem768_enc_deterministic(
        ct: *mut u8,
        ss: *mut u8,
        pk: *const u8,
        seed: *const u8,
    ) -> c_int;
    fn crypto_kem_mlkem768_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> c_int;
}

#[repr(C, align(16))]
struct Sha3256State {
    opaque: [u8; 256],
}

static XWING_LABEL: [u8; 6] = [0x5c, 0x2e, 0x2f, 0x2f, 0x5e, 0x5c];

unsafe fn expand_decaps_key(
    pk_mlkem: *mut u8,
    sk_mlkem: *mut u8,
    pk_x25519: *mut u8,
    sk_x25519: *mut u8,
    seed: *const u8,
) {
    let mut expanded = [0u8; 96];
    let mut mlkem_seed = [0u8; MLKEM768_SEEDBYTES];

    crypto_xof_shake256(expanded.as_mut_ptr(), 96, seed, XWING_SEEDBYTES as u64);

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
    let mut state = core::mem::MaybeUninit::<Sha3256State>::uninit();
    let sp = state.as_mut_ptr() as *mut c_void;

    crypto_hash_sha3256_init(sp);
    crypto_hash_sha3256_update(sp, ss_mlkem, MLKEM768_SHAREDSECRETBYTES as u64);
    crypto_hash_sha3256_update(sp, ss_x25519, CURVE25519_BYTES as u64);
    crypto_hash_sha3256_update(sp, ct_x25519, CURVE25519_BYTES as u64);
    crypto_hash_sha3256_update(sp, pk_x25519, CURVE25519_BYTES as u64);
    crypto_hash_sha3256_update(sp, XWING_LABEL.as_ptr(), XWING_LABEL.len() as u64);
    crypto_hash_sha3256_final(sp, ss);
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_xwing_publickeybytes() -> usize {
    XWING_PUBLICKEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_xwing_secretkeybytes() -> usize {
    XWING_SECRETKEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_xwing_ciphertextbytes() -> usize {
    XWING_CIPHERTEXTBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_xwing_sharedsecretbytes() -> usize {
    XWING_SHAREDSECRETBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_xwing_seedbytes() -> usize {
    XWING_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut pk_mlkem = [0u8; MLKEM768_PUBLICKEYBYTES];
    let mut sk_mlkem = [0u8; MLKEM768_SECRETKEYBYTES];
    let mut pk_x25519 = [0u8; CURVE25519_BYTES];
    let mut sk_x25519 = [0u8; CURVE25519_SCALARBYTES];

    expand_decaps_key(
        pk_mlkem.as_mut_ptr(),
        sk_mlkem.as_mut_ptr(),
        pk_x25519.as_mut_ptr(),
        sk_x25519.as_mut_ptr(),
        seed,
    );

    core::ptr::copy_nonoverlapping(pk_mlkem.as_ptr(), pk, MLKEM768_PUBLICKEYBYTES);
    core::ptr::copy_nonoverlapping(pk_x25519.as_ptr(), pk.add(MLKEM768_PUBLICKEYBYTES), CURVE25519_BYTES);

    core::ptr::copy_nonoverlapping(seed, sk, XWING_SEEDBYTES);

    sodium_memzero(sk_mlkem.as_mut_ptr() as *mut c_void, sk_mlkem.len());
    sodium_memzero(sk_x25519.as_mut_ptr() as *mut c_void, sk_x25519.len());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed = [0u8; XWING_SEEDBYTES];

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, XWING_SEEDBYTES);
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
) -> c_int {
    let pk_mlkem = pk;
    let pk_x25519 = pk.add(MLKEM768_PUBLICKEYBYTES);

    let seed_mlkem = seed;
    let sk_e_x25519 = seed.add(32);

    let mut ct_mlkem = [0u8; MLKEM768_CIPHERTEXTBYTES];
    let mut ss_mlkem = [0u8; MLKEM768_SHAREDSECRETBYTES];
    let mut ct_x25519 = [0u8; CURVE25519_BYTES];
    let mut ss_x25519 = [0u8; CURVE25519_BYTES];

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

    core::ptr::copy_nonoverlapping(ct_mlkem.as_ptr(), ct, MLKEM768_CIPHERTEXTBYTES);
    core::ptr::copy_nonoverlapping(ct_x25519.as_ptr(), ct.add(MLKEM768_CIPHERTEXTBYTES), CURVE25519_BYTES);

    combiner(ss, ss_mlkem.as_ptr(), ss_x25519.as_ptr(), ct_x25519.as_ptr(), pk_x25519);

    sodium_memzero(ss_mlkem.as_mut_ptr() as *mut c_void, ss_mlkem.len());
    sodium_memzero(ss_x25519.as_mut_ptr() as *mut c_void, ss_x25519.len());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_xwing_enc(ct: *mut u8, ss: *mut u8, pk: *const u8) -> c_int {
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
pub unsafe extern "C" fn crypto_kem_xwing_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> c_int {
    let mut pk_mlkem = [0u8; MLKEM768_PUBLICKEYBYTES];
    let mut sk_mlkem = [0u8; MLKEM768_SECRETKEYBYTES];
    let mut pk_x25519 = [0u8; CURVE25519_BYTES];
    let mut sk_x25519 = [0u8; CURVE25519_SCALARBYTES];

    let ct_mlkem = ct;
    let ct_x25519 = ct.add(MLKEM768_CIPHERTEXTBYTES);

    let mut ss_mlkem = [0u8; MLKEM768_SHAREDSECRETBYTES];
    let mut ss_x25519 = [0u8; CURVE25519_BYTES];

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

    combiner(ss, ss_mlkem.as_ptr(), ss_x25519.as_ptr(), ct_x25519, pk_x25519.as_ptr());

    sodium_memzero(ss_mlkem.as_mut_ptr() as *mut c_void, ss_mlkem.len());
    sodium_memzero(ss_x25519.as_mut_ptr() as *mut c_void, ss_x25519.len());
    sodium_memzero(sk_mlkem.as_mut_ptr() as *mut c_void, sk_mlkem.len());
    sodium_memzero(sk_x25519.as_mut_ptr() as *mut c_void, sk_x25519.len());

    0
}

// ---------------- crypto_kem (dispatch to xwing) ----------------

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_publickeybytes() -> usize {
    XWING_PUBLICKEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_secretkeybytes() -> usize {
    XWING_SECRETKEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_ciphertextbytes() -> usize {
    XWING_CIPHERTEXTBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_sharedsecretbytes() -> usize {
    XWING_SHAREDSECRETBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_seedbytes() -> usize {
    XWING_SEEDBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_primitive() -> *const c_char {
    CRYPTO_KEM_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> c_int {
    crypto_kem_xwing_seed_keypair(pk, sk, seed)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    crypto_kem_xwing_keypair(pk, sk)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_enc(ct: *mut u8, ss: *mut u8, pk: *const u8) -> c_int {
    crypto_kem_xwing_enc(ct, ss, pk)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> c_int {
    crypto_kem_xwing_dec(ss, ct, sk)
}
