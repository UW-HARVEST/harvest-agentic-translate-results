//! Translation of `crypto_kx/crypto_kx.c`.

use core::ffi::{c_char, c_int, c_void};

use crate::randombytes::randombytes_buf;
use crate::sodium::core::sodium_misuse;
use crate::sodium::utils::sodium_memzero;

// Constants from include/sodium/crypto_kx.h
pub const crypto_kx_PUBLICKEYBYTES: usize = 32;
pub const crypto_kx_SECRETKEYBYTES: usize = 32;
pub const crypto_kx_SEEDBYTES: usize = 32;
pub const crypto_kx_SESSIONKEYBYTES: usize = 32;

/// `#define crypto_kx_PRIMITIVE "x25519blake2b"`
static crypto_kx_PRIMITIVE: [u8; 14] = *b"x25519blake2b\0";

// Constants from include/sodium/crypto_scalarmult_curve25519.h
const crypto_scalarmult_BYTES: usize = 32;

/// `crypto_generichash_state` == `crypto_generichash_blake2b_state`:
/// ```c
/// #pragma pack(push, 1)
/// typedef struct CRYPTO_ALIGN(64) crypto_generichash_blake2b_state {
///     unsigned char opaque[384];
/// } crypto_generichash_blake2b_state;
/// #pragma pack(pop)
/// ```
/// `sizeof == 384`, `_Alignof == 64`.
#[repr(C, align(64))]
struct crypto_generichash_state {
    opaque: [u8; 384],
}

// `crypto_generichash*()` forward verbatim to the blake2b implementations
// (crypto_generichash/crypto_generichash.c), and `crypto_scalarmult*()`
// forward verbatim to curve25519 (crypto_scalarmult/crypto_scalarmult.c).
unsafe extern "C" {
    fn crypto_generichash_blake2b(
        out: *mut u8,
        outlen: usize,
        in_: *const u8,
        inlen: u64,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    fn crypto_generichash_blake2b_init(
        state: *mut crypto_generichash_state,
        key: *const u8,
        keylen: usize,
        outlen: usize,
    ) -> c_int;
    fn crypto_generichash_blake2b_update(
        state: *mut crypto_generichash_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_generichash_blake2b_final(
        state: *mut crypto_generichash_state,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;
    fn crypto_scalarmult_curve25519(q: *mut u8, n: *const u8, p: *const u8) -> c_int;
    fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    unsafe {
        crypto_generichash_blake2b(
            sk,
            crypto_kx_SECRETKEYBYTES,
            seed,
            crypto_kx_SEEDBYTES as u64,
            core::ptr::null(),
            0,
        );
        crypto_scalarmult_curve25519_base(pk, sk)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    /* COMPILER_ASSERT(crypto_kx_SECRETKEYBYTES == crypto_scalarmult_SCALARBYTES); */
    /* COMPILER_ASSERT(crypto_kx_PUBLICKEYBYTES == crypto_scalarmult_BYTES); */

    randombytes_buf(sk as *mut c_void, crypto_kx_SECRETKEYBYTES);
    unsafe { crypto_scalarmult_curve25519_base(pk, sk) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_client_session_keys(
    mut rx: *mut u8,
    mut tx: *mut u8,
    client_pk: *const u8,
    client_sk: *const u8,
    server_pk: *const u8,
) -> c_int {
    unsafe {
        let mut h = core::mem::MaybeUninit::<crypto_generichash_state>::uninit();
        let mut q = [0u8; crypto_scalarmult_BYTES];
        let mut keys = [0u8; 2 * crypto_kx_SESSIONKEYBYTES];
        let mut i: c_int;

        if rx.is_null() {
            rx = tx;
        }
        if tx.is_null() {
            tx = rx;
        }
        if rx.is_null() {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
        if crypto_scalarmult_curve25519(q.as_mut_ptr(), client_sk, server_pk) != 0 {
            return -1;
        }
        /* COMPILER_ASSERT(sizeof keys <= crypto_generichash_BYTES_MAX); */
        crypto_generichash_blake2b_init(h.as_mut_ptr(), core::ptr::null(), 0, keys.len());
        crypto_generichash_blake2b_update(
            h.as_mut_ptr(),
            q.as_ptr(),
            crypto_scalarmult_BYTES as u64,
        );
        sodium_memzero(q.as_mut_ptr() as *mut c_void, q.len());
        crypto_generichash_blake2b_update(
            h.as_mut_ptr(),
            client_pk,
            crypto_kx_PUBLICKEYBYTES as u64,
        );
        crypto_generichash_blake2b_update(
            h.as_mut_ptr(),
            server_pk,
            crypto_kx_PUBLICKEYBYTES as u64,
        );
        crypto_generichash_blake2b_final(h.as_mut_ptr(), keys.as_mut_ptr(), keys.len());
        sodium_memzero(
            h.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<crypto_generichash_state>(),
        );
        i = 0;
        while i < crypto_kx_SESSIONKEYBYTES as c_int {
            *rx.offset(i as isize) = keys[i as usize]; /* rx cannot be NULL */
            *tx.offset(i as isize) = keys[i as usize + crypto_kx_SESSIONKEYBYTES]; /* tx cannot be NULL */
            i += 1;
        }
        sodium_memzero(keys.as_mut_ptr() as *mut c_void, keys.len());

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_server_session_keys(
    mut rx: *mut u8,
    mut tx: *mut u8,
    server_pk: *const u8,
    server_sk: *const u8,
    client_pk: *const u8,
) -> c_int {
    unsafe {
        let mut h = core::mem::MaybeUninit::<crypto_generichash_state>::uninit();
        let mut q = [0u8; crypto_scalarmult_BYTES];
        let mut keys = [0u8; 2 * crypto_kx_SESSIONKEYBYTES];
        let mut i: c_int;

        if rx.is_null() {
            rx = tx;
        }
        if tx.is_null() {
            tx = rx;
        }
        if rx.is_null() {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
        if crypto_scalarmult_curve25519(q.as_mut_ptr(), server_sk, client_pk) != 0 {
            return -1;
        }
        /* COMPILER_ASSERT(sizeof keys <= crypto_generichash_BYTES_MAX); */
        crypto_generichash_blake2b_init(h.as_mut_ptr(), core::ptr::null(), 0, keys.len());
        crypto_generichash_blake2b_update(
            h.as_mut_ptr(),
            q.as_ptr(),
            crypto_scalarmult_BYTES as u64,
        );
        sodium_memzero(q.as_mut_ptr() as *mut c_void, q.len());
        crypto_generichash_blake2b_update(
            h.as_mut_ptr(),
            client_pk,
            crypto_kx_PUBLICKEYBYTES as u64,
        );
        crypto_generichash_blake2b_update(
            h.as_mut_ptr(),
            server_pk,
            crypto_kx_PUBLICKEYBYTES as u64,
        );
        crypto_generichash_blake2b_final(h.as_mut_ptr(), keys.as_mut_ptr(), keys.len());
        sodium_memzero(
            h.as_mut_ptr() as *mut c_void,
            core::mem::size_of::<crypto_generichash_state>(),
        );
        i = 0;
        while i < crypto_kx_SESSIONKEYBYTES as c_int {
            *tx.offset(i as isize) = keys[i as usize];
            *rx.offset(i as isize) = keys[i as usize + crypto_kx_SESSIONKEYBYTES];
            i += 1;
        }
        sodium_memzero(keys.as_mut_ptr() as *mut c_void, keys.len());

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_publickeybytes() -> usize {
    crypto_kx_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_secretkeybytes() -> usize {
    crypto_kx_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_seedbytes() -> usize {
    crypto_kx_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_sessionkeybytes() -> usize {
    crypto_kx_SESSIONKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_primitive() -> *const c_char {
    crypto_kx_PRIMITIVE.as_ptr() as *const c_char
}
