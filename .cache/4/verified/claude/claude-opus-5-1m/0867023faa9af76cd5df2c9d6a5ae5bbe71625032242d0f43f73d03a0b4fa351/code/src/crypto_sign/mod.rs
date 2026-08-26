pub mod ed25519;

// ---------------------------------------------------------------------------
// Translation of `crypto_sign/crypto_sign.c`
// ---------------------------------------------------------------------------

use core::ffi::{c_char, c_int};

use crate::crypto_sign::ed25519::crypto_sign_ed25519ph_state;

/// C: `typedef crypto_sign_ed25519ph_state crypto_sign_state;`
pub type crypto_sign_state = crypto_sign_ed25519ph_state;

/// `#define crypto_sign_BYTES crypto_sign_ed25519_BYTES`
pub const crypto_sign_BYTES: usize = self::ed25519::crypto_sign_ed25519_BYTES;

/// `#define crypto_sign_SEEDBYTES crypto_sign_ed25519_SEEDBYTES`
pub const crypto_sign_SEEDBYTES: usize = self::ed25519::crypto_sign_ed25519_SEEDBYTES;

/// `#define crypto_sign_PUBLICKEYBYTES crypto_sign_ed25519_PUBLICKEYBYTES`
pub const crypto_sign_PUBLICKEYBYTES: usize = self::ed25519::crypto_sign_ed25519_PUBLICKEYBYTES;

/// `#define crypto_sign_SECRETKEYBYTES crypto_sign_ed25519_SECRETKEYBYTES`
pub const crypto_sign_SECRETKEYBYTES: usize = self::ed25519::crypto_sign_ed25519_SECRETKEYBYTES;

/// `#define crypto_sign_MESSAGEBYTES_MAX crypto_sign_ed25519_MESSAGEBYTES_MAX`
pub const crypto_sign_MESSAGEBYTES_MAX: usize = self::ed25519::crypto_sign_ed25519_MESSAGEBYTES_MAX;

/// `#define crypto_sign_PRIMITIVE "ed25519"`
pub const crypto_sign_PRIMITIVE: &[u8; 8] = b"ed25519\0";

/// ```c
/// size_t crypto_sign_statebytes(void) { return sizeof(crypto_sign_state); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_statebytes() -> usize {
    core::mem::size_of::<crypto_sign_state>()
}

/// ```c
/// size_t crypto_sign_bytes(void) { return crypto_sign_BYTES; }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_bytes() -> usize {
    crypto_sign_BYTES
}

/// ```c
/// size_t crypto_sign_seedbytes(void) { return crypto_sign_SEEDBYTES; }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seedbytes() -> usize {
    crypto_sign_SEEDBYTES
}

/// ```c
/// size_t crypto_sign_publickeybytes(void) { return crypto_sign_PUBLICKEYBYTES; }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_publickeybytes() -> usize {
    crypto_sign_PUBLICKEYBYTES
}

/// ```c
/// size_t crypto_sign_secretkeybytes(void) { return crypto_sign_SECRETKEYBYTES; }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_secretkeybytes() -> usize {
    crypto_sign_SECRETKEYBYTES
}

/// ```c
/// size_t crypto_sign_messagebytes_max(void) { return crypto_sign_MESSAGEBYTES_MAX; }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_messagebytes_max() -> usize {
    crypto_sign_MESSAGEBYTES_MAX
}

/// ```c
/// const char *crypto_sign_primitive(void) { return crypto_sign_PRIMITIVE; }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_primitive() -> *const c_char {
    crypto_sign_PRIMITIVE.as_ptr() as *const c_char
}

/// ```c
/// int crypto_sign_seed_keypair(unsigned char *pk, unsigned char *sk,
///                              const unsigned char *seed);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    unsafe { self::ed25519::ref10::keypair::crypto_sign_ed25519_seed_keypair(pk, sk, seed) }
}

/// ```c
/// int crypto_sign_keypair(unsigned char *pk, unsigned char *sk);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    unsafe { self::ed25519::ref10::keypair::crypto_sign_ed25519_keypair(pk, sk) }
}

/// ```c
/// int crypto_sign(unsigned char *sm, unsigned long long *smlen_p,
///                 const unsigned char *m, unsigned long long mlen,
///                 const unsigned char *sk);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    unsafe { self::ed25519::ref10::sign::crypto_sign_ed25519(sm, smlen_p, m, mlen, sk) }
}

/// ```c
/// int crypto_sign_open(unsigned char *m, unsigned long long *mlen_p,
///                      const unsigned char *sm, unsigned long long smlen,
///                      const unsigned char *pk);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen_p: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> c_int {
    unsafe { self::ed25519::ref10::open::crypto_sign_ed25519_open(m, mlen_p, sm, smlen, pk) }
}

/// ```c
/// int crypto_sign_detached(unsigned char *sig, unsigned long long *siglen_p,
///                          const unsigned char *m, unsigned long long mlen,
///                          const unsigned char *sk);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_detached(
    sig: *mut u8,
    siglen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    unsafe { self::ed25519::ref10::sign::crypto_sign_ed25519_detached(sig, siglen_p, m, mlen, sk) }
}

/// ```c
/// int crypto_sign_verify_detached(const unsigned char *sig, const unsigned char *m,
///                                 unsigned long long mlen, const unsigned char *pk);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
) -> c_int {
    unsafe { self::ed25519::ref10::open::crypto_sign_ed25519_verify_detached(sig, m, mlen, pk) }
}

/// ```c
/// int crypto_sign_init(crypto_sign_state *state);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_init(state: *mut crypto_sign_state) -> c_int {
    unsafe { self::ed25519::crypto_sign_ed25519ph_init(state) }
}

/// ```c
/// int crypto_sign_update(crypto_sign_state *state, const unsigned char *m,
///                        unsigned long long mlen);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_update(
    state: *mut crypto_sign_state,
    m: *const u8,
    mlen: u64,
) -> c_int {
    unsafe { self::ed25519::crypto_sign_ed25519ph_update(state, m, mlen) }
}

/// ```c
/// int crypto_sign_final_create(crypto_sign_state *state, unsigned char *sig,
///                              unsigned long long *siglen_p, const unsigned char *sk);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_final_create(
    state: *mut crypto_sign_state,
    sig: *mut u8,
    siglen_p: *mut u64,
    sk: *const u8,
) -> c_int {
    unsafe { self::ed25519::crypto_sign_ed25519ph_final_create(state, sig, siglen_p, sk) }
}

/// ```c
/// int crypto_sign_final_verify(crypto_sign_state *state, const unsigned char *sig,
///                              const unsigned char *pk);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_final_verify(
    state: *mut crypto_sign_state,
    sig: *const u8,
    pk: *const u8,
) -> c_int {
    unsafe { self::ed25519::crypto_sign_ed25519ph_final_verify(state, sig, pk) }
}
