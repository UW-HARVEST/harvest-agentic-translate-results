//! Translation of c_src/libsodium/crypto_kx/crypto_kx.c

use core::ffi::{c_char, c_int, c_void};

const CRYPTO_KX_PUBLICKEYBYTES: usize = 32;
const CRYPTO_KX_SECRETKEYBYTES: usize = 32;
const CRYPTO_KX_SEEDBYTES: usize = 32;
const CRYPTO_KX_SESSIONKEYBYTES: usize = 32;
const CRYPTO_KX_PRIMITIVE: &[u8] = b"x25519blake2b\0";

const CRYPTO_SCALARMULT_BYTES: usize = 32;

// crypto_generichash_state == crypto_generichash_blake2b_state:
// #pragma pack(1) + CRYPTO_ALIGN(64) { unsigned char opaque[384]; }
// For an opaque [u8; 384], pack(1) has no effect; CRYPTO_ALIGN(64) sets alignment.
#[repr(C, align(64))]
struct CryptoGenerichashState {
    opaque: [u8; 384],
}

extern "C" {
    fn crypto_generichash(
        out: *mut u8,
        outlen: usize,
        in_: *const u8,
        inlen: u64,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    fn crypto_generichash_init(
        state: *mut CryptoGenerichashState,
        key: *const u8,
        keylen: usize,
        outlen: usize,
    ) -> c_int;
    fn crypto_generichash_update(
        state: *mut CryptoGenerichashState,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_generichash_final(
        state: *mut CryptoGenerichashState,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;

    fn crypto_scalarmult_base(q: *mut u8, n: *const u8) -> c_int;
    fn crypto_scalarmult(q: *mut u8, n: *const u8, p: *const u8) -> c_int;

    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    crypto_generichash(
        sk,
        CRYPTO_KX_SECRETKEYBYTES,
        seed,
        CRYPTO_KX_SEEDBYTES as u64,
        core::ptr::null(),
        0,
    );
    crypto_scalarmult_base(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    // COMPILER_ASSERT: crypto_kx_SECRETKEYBYTES == crypto_scalarmult_SCALARBYTES
    // COMPILER_ASSERT: crypto_kx_PUBLICKEYBYTES == crypto_scalarmult_BYTES

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
    let mut h = CryptoGenerichashState { opaque: [0; 384] };
    let mut q: [u8; CRYPTO_SCALARMULT_BYTES] = [0; CRYPTO_SCALARMULT_BYTES];
    let mut keys: [u8; 2 * CRYPTO_KX_SESSIONKEYBYTES] = [0; 2 * CRYPTO_KX_SESSIONKEYBYTES];
    let mut i: c_int;

    if rx.is_null() {
        rx = tx;
    }
    if tx.is_null() {
        tx = rx;
    }
    if rx.is_null() {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if crypto_scalarmult(q.as_mut_ptr(), client_sk, server_pk) != 0 {
        return -1;
    }
    // COMPILER_ASSERT: sizeof keys <= crypto_generichash_BYTES_MAX
    crypto_generichash_init(&mut h, core::ptr::null(), 0, core::mem::size_of_val(&keys));
    crypto_generichash_update(&mut h, q.as_ptr(), CRYPTO_SCALARMULT_BYTES as u64);
    sodium_memzero(q.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&q));
    crypto_generichash_update(&mut h, client_pk, CRYPTO_KX_PUBLICKEYBYTES as u64);
    crypto_generichash_update(&mut h, server_pk, CRYPTO_KX_PUBLICKEYBYTES as u64);
    crypto_generichash_final(&mut h, keys.as_mut_ptr(), core::mem::size_of_val(&keys));
    sodium_memzero(&mut h as *mut _ as *mut c_void, core::mem::size_of::<CryptoGenerichashState>());
    i = 0;
    while i < CRYPTO_KX_SESSIONKEYBYTES as c_int {
        *rx.add(i as usize) = keys[i as usize]; /* rx cannot be NULL */
        *tx.add(i as usize) = keys[i as usize + CRYPTO_KX_SESSIONKEYBYTES]; /* tx cannot be NULL */
        i += 1;
    }
    sodium_memzero(keys.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&keys));

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
    let mut h = CryptoGenerichashState { opaque: [0; 384] };
    let mut q: [u8; CRYPTO_SCALARMULT_BYTES] = [0; CRYPTO_SCALARMULT_BYTES];
    let mut keys: [u8; 2 * CRYPTO_KX_SESSIONKEYBYTES] = [0; 2 * CRYPTO_KX_SESSIONKEYBYTES];
    let mut i: c_int;

    if rx.is_null() {
        rx = tx;
    }
    if tx.is_null() {
        tx = rx;
    }
    if rx.is_null() {
        crate::sodium::core::sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if crypto_scalarmult(q.as_mut_ptr(), server_sk, client_pk) != 0 {
        return -1;
    }
    // COMPILER_ASSERT: sizeof keys <= crypto_generichash_BYTES_MAX
    crypto_generichash_init(&mut h, core::ptr::null(), 0, core::mem::size_of_val(&keys));
    crypto_generichash_update(&mut h, q.as_ptr(), CRYPTO_SCALARMULT_BYTES as u64);
    sodium_memzero(q.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&q));
    crypto_generichash_update(&mut h, client_pk, CRYPTO_KX_PUBLICKEYBYTES as u64);
    crypto_generichash_update(&mut h, server_pk, CRYPTO_KX_PUBLICKEYBYTES as u64);
    crypto_generichash_final(&mut h, keys.as_mut_ptr(), core::mem::size_of_val(&keys));
    sodium_memzero(&mut h as *mut _ as *mut c_void, core::mem::size_of::<CryptoGenerichashState>());
    i = 0;
    while i < CRYPTO_KX_SESSIONKEYBYTES as c_int {
        *tx.add(i as usize) = keys[i as usize];
        *rx.add(i as usize) = keys[i as usize + CRYPTO_KX_SESSIONKEYBYTES];
        i += 1;
    }
    sodium_memzero(keys.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&keys));

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_publickeybytes() -> usize {
    CRYPTO_KX_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_secretkeybytes() -> usize {
    CRYPTO_KX_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_seedbytes() -> usize {
    CRYPTO_KX_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_sessionkeybytes() -> usize {
    CRYPTO_KX_SESSIONKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_primitive() -> *const c_char {
    CRYPTO_KX_PRIMITIVE.as_ptr() as *const c_char
}
