//! Translation of `c_src/libsodium/sodium/core.c`.
//!
//! The reference build defines neither `HAVE_PTHREAD` nor `HAVE_ATOMIC_OPS`
//! (and this is not `_WIN32`), so `sodium_crit_enter`/`sodium_crit_leave` are
//! the trivial no-op versions at the bottom of the C file.

use core::ffi::c_int;

extern "C" {
    fn _sodium_runtime_get_cpu_features() -> c_int;
    fn randombytes_stir();
    fn _sodium_alloc_init() -> c_int;
    fn _crypto_pwhash_argon2_pick_best_implementation() -> c_int;
    fn _crypto_generichash_blake2b_pick_best_implementation() -> c_int;
    fn _crypto_onetimeauth_poly1305_pick_best_implementation() -> c_int;
    fn _crypto_scalarmult_curve25519_pick_best_implementation() -> c_int;
    fn _crypto_stream_chacha20_pick_best_implementation() -> c_int;
    fn _crypto_stream_salsa20_pick_best_implementation() -> c_int;
    fn _crypto_aead_aegis128l_pick_best_implementation() -> c_int;
    fn _crypto_aead_aegis256_pick_best_implementation() -> c_int;
    fn _crypto_ipcrypt_pick_best_implementation() -> c_int;
}

static mut initialized: i32 = 0;
static mut locked: i32 = 0;

#[no_mangle]
pub unsafe extern "C" fn sodium_init() -> c_int {
    if sodium_crit_enter() != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    if initialized != 0 {
        if sodium_crit_leave() != 0 {
            return -1; /* LCOV_EXCL_LINE */
        }
        return 1;
    }
    _sodium_runtime_get_cpu_features();
    randombytes_stir();
    _sodium_alloc_init();
    _crypto_pwhash_argon2_pick_best_implementation();
    _crypto_generichash_blake2b_pick_best_implementation();
    _crypto_onetimeauth_poly1305_pick_best_implementation();
    _crypto_scalarmult_curve25519_pick_best_implementation();
    _crypto_stream_chacha20_pick_best_implementation();
    _crypto_stream_salsa20_pick_best_implementation();
    _crypto_aead_aegis128l_pick_best_implementation();
    _crypto_aead_aegis256_pick_best_implementation();
    _crypto_ipcrypt_pick_best_implementation();
    initialized = 1;
    if sodium_crit_leave() != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

// `HAVE_PTHREAD` and `HAVE_ATOMIC_OPS` are NOT defined and this is not
// `_WIN32`, so these are the trivial no-op versions from the C source.

#[no_mangle]
pub unsafe extern "C" fn sodium_crit_enter() -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn sodium_crit_leave() -> c_int {
    0
}

static mut _misuse_handler: Option<extern "C" fn()> = None;

#[no_mangle]
pub unsafe extern "C" fn sodium_misuse() -> ! {
    let _ = sodium_crit_leave();
    if sodium_crit_enter() == 0 {
        let handler = _misuse_handler;
        if sodium_crit_leave() == 0 {
            if let Some(h) = handler {
                h();
            }
        }
    }
    /* LCOV_EXCL_START */
    crate::csys::abort();
    /* LCOV_EXCL_STOP */
}

#[no_mangle]
pub unsafe extern "C" fn sodium_set_misuse_handler(handler: Option<extern "C" fn()>) -> c_int {
    if sodium_crit_enter() != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    _misuse_handler = handler;
    if sodium_crit_leave() != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}
