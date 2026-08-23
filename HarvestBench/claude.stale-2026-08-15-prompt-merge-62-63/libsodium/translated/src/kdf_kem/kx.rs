// Translation of crypto_kx/crypto_kx.c (also defines crypto_scalarmult_base here,
// per P5_files.txt).

use core::ffi::{c_char, c_int, c_void};

const CRYPTO_KX_PUBLICKEYBYTES: usize = 32;
const CRYPTO_KX_SECRETKEYBYTES: usize = 32;
const CRYPTO_KX_SEEDBYTES: usize = 32;
const CRYPTO_KX_SESSIONKEYBYTES: usize = 32;
const CRYPTO_KX_PRIMITIVE: &[u8] = b"x25519blake2b\0";

const CRYPTO_SCALARMULT_BYTES: usize = 32;

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_misuse() -> !;
    fn crypto_generichash(
        out: *mut u8,
        outlen: usize,
        inp: *const u8,
        inlen: u64,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    fn crypto_generichash_init(state: *mut c_void, key: *const u8, keylen: usize, outlen: usize) -> c_int;
    fn crypto_generichash_update(state: *mut c_void, inp: *const u8, inlen: u64) -> c_int;
    fn crypto_generichash_final(state: *mut c_void, out: *mut u8, outlen: usize) -> c_int;
    fn crypto_scalarmult(q: *mut u8, n: *const u8, p: *const u8) -> c_int;
    fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> c_int;
}

// crypto_scalarmult_base -> crypto_scalarmult_curve25519_base
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_base(q: *mut u8, n: *const u8) -> c_int {
    crypto_scalarmult_curve25519_base(q, n)
}

// crypto_generichash_state is CRYPTO_ALIGN(64) opaque[384].
#[repr(C, align(64))]
struct GenericHashState {
    opaque: [u8; 384],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    crypto_generichash(sk, CRYPTO_KX_SECRETKEYBYTES, seed, CRYPTO_KX_SEEDBYTES as u64, core::ptr::null(), 0);
    crypto_scalarmult_base(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    randombytes_buf(sk as *mut c_void, CRYPTO_KX_SECRETKEYBYTES);
    crypto_scalarmult_base(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_client_session_keys(
    mut rx: *mut u8,
    mut tx: *mut u8,
    client_pk: *const u8,
    client_sk: *const u8,
    server_pk: *const u8,
) -> c_int {
    let mut h = core::mem::MaybeUninit::<GenericHashState>::uninit();
    let hp = h.as_mut_ptr() as *mut c_void;
    let mut q = [0u8; CRYPTO_SCALARMULT_BYTES];
    let mut keys = [0u8; 2 * CRYPTO_KX_SESSIONKEYBYTES];

    if rx.is_null() {
        rx = tx;
    }
    if tx.is_null() {
        tx = rx;
    }
    if rx.is_null() {
        sodium_misuse();
    }
    if crypto_scalarmult(q.as_mut_ptr(), client_sk, server_pk) != 0 {
        return -1;
    }
    crypto_generichash_init(hp, core::ptr::null(), 0, keys.len());
    crypto_generichash_update(hp, q.as_ptr(), CRYPTO_SCALARMULT_BYTES as u64);
    sodium_memzero(q.as_mut_ptr() as *mut c_void, q.len());
    crypto_generichash_update(hp, client_pk, CRYPTO_KX_PUBLICKEYBYTES as u64);
    crypto_generichash_update(hp, server_pk, CRYPTO_KX_PUBLICKEYBYTES as u64);
    crypto_generichash_final(hp, keys.as_mut_ptr(), keys.len());
    sodium_memzero(hp, core::mem::size_of::<GenericHashState>());
    for i in 0..CRYPTO_KX_SESSIONKEYBYTES {
        *rx.add(i) = keys[i];
        *tx.add(i) = keys[i + CRYPTO_KX_SESSIONKEYBYTES];
    }
    sodium_memzero(keys.as_mut_ptr() as *mut c_void, keys.len());
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_server_session_keys(
    mut rx: *mut u8,
    mut tx: *mut u8,
    server_pk: *const u8,
    server_sk: *const u8,
    client_pk: *const u8,
) -> c_int {
    let mut h = core::mem::MaybeUninit::<GenericHashState>::uninit();
    let hp = h.as_mut_ptr() as *mut c_void;
    let mut q = [0u8; CRYPTO_SCALARMULT_BYTES];
    let mut keys = [0u8; 2 * CRYPTO_KX_SESSIONKEYBYTES];

    if rx.is_null() {
        rx = tx;
    }
    if tx.is_null() {
        tx = rx;
    }
    if rx.is_null() {
        sodium_misuse();
    }
    if crypto_scalarmult(q.as_mut_ptr(), server_sk, client_pk) != 0 {
        return -1;
    }
    crypto_generichash_init(hp, core::ptr::null(), 0, keys.len());
    crypto_generichash_update(hp, q.as_ptr(), CRYPTO_SCALARMULT_BYTES as u64);
    sodium_memzero(q.as_mut_ptr() as *mut c_void, q.len());
    crypto_generichash_update(hp, client_pk, CRYPTO_KX_PUBLICKEYBYTES as u64);
    crypto_generichash_update(hp, server_pk, CRYPTO_KX_PUBLICKEYBYTES as u64);
    crypto_generichash_final(hp, keys.as_mut_ptr(), keys.len());
    sodium_memzero(hp, core::mem::size_of::<GenericHashState>());
    for i in 0..CRYPTO_KX_SESSIONKEYBYTES {
        *tx.add(i) = keys[i];
        *rx.add(i) = keys[i + CRYPTO_KX_SESSIONKEYBYTES];
    }
    sodium_memzero(keys.as_mut_ptr() as *mut c_void, keys.len());
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kx_publickeybytes() -> usize {
    CRYPTO_KX_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kx_secretkeybytes() -> usize {
    CRYPTO_KX_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kx_seedbytes() -> usize {
    CRYPTO_KX_SEEDBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kx_sessionkeybytes() -> usize {
    CRYPTO_KX_SESSIONKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kx_primitive() -> *const c_char {
    CRYPTO_KX_PRIMITIVE.as_ptr() as *const c_char
}
