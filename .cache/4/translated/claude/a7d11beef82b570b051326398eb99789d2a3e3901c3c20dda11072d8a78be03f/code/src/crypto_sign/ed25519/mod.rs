pub mod ref10;

// ---------------------------------------------------------------------------
// Translation of `crypto_sign/ed25519/sign_ed25519.c`
// ---------------------------------------------------------------------------

use core::ffi::c_int;
use core::ptr::addr_of_mut;

use crate::common::SODIUM_SIZE_MAX;
use crate::crypto_hash::sha512::{crypto_hash_sha512_BYTES, crypto_hash_sha512_state};

unsafe extern "C" {
    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int;
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha512_final(state: *mut crypto_hash_sha512_state, out: *mut u8) -> c_int;
}

/// ```c
/// typedef struct crypto_sign_ed25519ph_state {
///     crypto_hash_sha512_state hs;
/// } crypto_sign_ed25519ph_state;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct crypto_sign_ed25519ph_state {
    pub hs: crypto_hash_sha512_state,
}

/// `#define crypto_sign_ed25519_BYTES 64U`
pub const crypto_sign_ed25519_BYTES: usize = 64;

/// `#define crypto_sign_ed25519_SEEDBYTES 32U`
pub const crypto_sign_ed25519_SEEDBYTES: usize = 32;

/// `#define crypto_sign_ed25519_PUBLICKEYBYTES 32U`
pub const crypto_sign_ed25519_PUBLICKEYBYTES: usize = 32;

/// `#define crypto_sign_ed25519_SECRETKEYBYTES (32U + 32U)`
pub const crypto_sign_ed25519_SECRETKEYBYTES: usize = 32 + 32;

/// `#define crypto_sign_ed25519_MESSAGEBYTES_MAX (SODIUM_SIZE_MAX - crypto_sign_ed25519_BYTES)`
pub const crypto_sign_ed25519_MESSAGEBYTES_MAX: usize =
    (SODIUM_SIZE_MAX - crypto_sign_ed25519_BYTES as u64) as usize;

/// ```c
/// size_t crypto_sign_ed25519ph_statebytes(void)
/// {
///     return sizeof(crypto_sign_ed25519ph_state);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_statebytes() -> usize {
    core::mem::size_of::<crypto_sign_ed25519ph_state>()
}

/// ```c
/// size_t crypto_sign_ed25519_bytes(void) { return crypto_sign_ed25519_BYTES; }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_bytes() -> usize {
    crypto_sign_ed25519_BYTES
}

/// ```c
/// size_t crypto_sign_ed25519_seedbytes(void) { return crypto_sign_ed25519_SEEDBYTES; }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_seedbytes() -> usize {
    crypto_sign_ed25519_SEEDBYTES
}

/// ```c
/// size_t crypto_sign_ed25519_publickeybytes(void) { return crypto_sign_ed25519_PUBLICKEYBYTES; }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_publickeybytes() -> usize {
    crypto_sign_ed25519_PUBLICKEYBYTES
}

/// ```c
/// size_t crypto_sign_ed25519_secretkeybytes(void) { return crypto_sign_ed25519_SECRETKEYBYTES; }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_secretkeybytes() -> usize {
    crypto_sign_ed25519_SECRETKEYBYTES
}

/// ```c
/// size_t crypto_sign_ed25519_messagebytes_max(void)
/// {
///     return crypto_sign_ed25519_MESSAGEBYTES_MAX;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_messagebytes_max() -> usize {
    crypto_sign_ed25519_MESSAGEBYTES_MAX
}

/// ```c
/// int crypto_sign_ed25519_sk_to_seed(unsigned char *seed, const unsigned char *sk)
/// {
///     memmove(seed, sk, crypto_sign_ed25519_SEEDBYTES);
///
///     return 0;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_seed(seed: *mut u8, sk: *const u8) -> c_int {
    unsafe { core::ptr::copy(sk, seed, crypto_sign_ed25519_SEEDBYTES) };

    0
}

/// ```c
/// int crypto_sign_ed25519_sk_to_pk(unsigned char *pk, const unsigned char *sk)
/// {
///     memmove(pk, sk + crypto_sign_ed25519_SEEDBYTES,
///             crypto_sign_ed25519_PUBLICKEYBYTES);
///     return 0;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_pk(pk: *mut u8, sk: *const u8) -> c_int {
    unsafe {
        core::ptr::copy(
            sk.add(crypto_sign_ed25519_SEEDBYTES),
            pk,
            crypto_sign_ed25519_PUBLICKEYBYTES,
        )
    };
    0
}

/// ```c
/// int crypto_sign_ed25519ph_init(crypto_sign_ed25519ph_state *state)
/// {
///     crypto_hash_sha512_init(&state->hs);
///     return 0;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_init(
    state: *mut crypto_sign_ed25519ph_state,
) -> c_int {
    unsafe { crypto_hash_sha512_init(addr_of_mut!((*state).hs)) };
    0
}

/// ```c
/// int crypto_sign_ed25519ph_update(crypto_sign_ed25519ph_state *state,
///                                  const unsigned char *m, unsigned long long mlen)
/// {
///     return crypto_hash_sha512_update(&state->hs, m, mlen);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_update(
    state: *mut crypto_sign_ed25519ph_state,
    m: *const u8,
    mlen: u64,
) -> c_int {
    unsafe { crypto_hash_sha512_update(addr_of_mut!((*state).hs), m, mlen) }
}

/// ```c
/// int crypto_sign_ed25519ph_final_create(crypto_sign_ed25519ph_state *state,
///                                        unsigned char               *sig,
///                                        unsigned long long          *siglen_p,
///                                        const unsigned char         *sk)
/// {
///     unsigned char ph[crypto_hash_sha512_BYTES];
///
///     crypto_hash_sha512_final(&state->hs, ph);
///
///     return _crypto_sign_ed25519_detached(sig, siglen_p, ph, sizeof ph, sk, 1);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_final_create(
    state: *mut crypto_sign_ed25519ph_state,
    sig: *mut u8,
    siglen_p: *mut u64,
    sk: *const u8,
) -> c_int {
    let mut ph = [0u8; crypto_hash_sha512_BYTES];

    unsafe { crypto_hash_sha512_final(addr_of_mut!((*state).hs), ph.as_mut_ptr()) };

    unsafe {
        self::ref10::sign::_crypto_sign_ed25519_detached(
            sig,
            siglen_p,
            ph.as_ptr(),
            crypto_hash_sha512_BYTES as u64,
            sk,
            1,
        )
    }
}

/// ```c
/// int crypto_sign_ed25519ph_final_verify(crypto_sign_ed25519ph_state *state,
///                                        const unsigned char         *sig,
///                                        const unsigned char         *pk)
/// {
///     unsigned char ph[crypto_hash_sha512_BYTES];
///
///     crypto_hash_sha512_final(&state->hs, ph);
///
///     return _crypto_sign_ed25519_verify_detached(sig, ph, sizeof ph, pk, 1);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_final_verify(
    state: *mut crypto_sign_ed25519ph_state,
    sig: *const u8,
    pk: *const u8,
) -> c_int {
    let mut ph = [0u8; crypto_hash_sha512_BYTES];

    unsafe { crypto_hash_sha512_final(addr_of_mut!((*state).hs), ph.as_mut_ptr()) };

    unsafe {
        self::ref10::open::_crypto_sign_ed25519_verify_detached(
            sig,
            ph.as_ptr(),
            crypto_hash_sha512_BYTES as u64,
            pk,
            1,
        )
    }
}
