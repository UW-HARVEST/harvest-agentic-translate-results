//! Translation of `sodium/core.c`
//!
//! Neither `_WIN32`, `HAVE_PTHREAD` nor `HAVE_ATOMIC_OPS` is defined by the
//! reference build, so `sodium_crit_enter()`/`sodium_crit_leave()` fall through
//! to the no-op variants that always return 0.

use core::ffi::c_int;

static mut INITIALIZED: c_int = 0;

#[unsafe(no_mangle)]
pub extern "C" fn sodium_init() -> c_int {
    if sodium_crit_enter() != 0 {
        return -1;
    }
    unsafe {
        if INITIALIZED != 0 {
            if sodium_crit_leave() != 0 {
                return -1;
            }
            return 1;
        }
    }
    crate::sodium_runtime::_sodium_runtime_get_cpu_features();
    crate::randombytes::randombytes_stir();
    crate::sodium_utils::_sodium_alloc_init();
    unsafe {
        crate::crypto_pwhash::argon2::argon2_core::_crypto_pwhash_argon2_pick_best_implementation();
        crate::crypto_generichash::blake2b::_crypto_generichash_blake2b_pick_best_implementation();
        crate::crypto_onetimeauth::poly1305::_crypto_onetimeauth_poly1305_pick_best_implementation(
        );
        crate::crypto_scalarmult::curve25519::_crypto_scalarmult_curve25519_pick_best_implementation();
        crate::crypto_stream::chacha20::_crypto_stream_chacha20_pick_best_implementation();
        crate::crypto_stream::salsa20::_crypto_stream_salsa20_pick_best_implementation();
        crate::crypto_aead::aegis128l::_crypto_aead_aegis128l_pick_best_implementation();
        crate::crypto_aead::aegis256::_crypto_aead_aegis256_pick_best_implementation();
        crate::crypto_ipcrypt::_crypto_ipcrypt_pick_best_implementation();
    }
    unsafe {
        INITIALIZED = 1;
    }
    if sodium_crit_leave() != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_crit_enter() -> c_int {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_crit_leave() -> c_int {
    0
}

static mut _MISUSE_HANDLER: Option<extern "C" fn()> = None;

#[unsafe(no_mangle)]
pub extern "C" fn sodium_misuse() -> ! {
    let handler;

    let _ = sodium_crit_leave();
    if sodium_crit_enter() == 0 {
        handler = unsafe { *(&raw const _MISUSE_HANDLER) };
        if sodium_crit_leave() == 0 {
            if let Some(h) = handler {
                h();
            }
        }
    }
    crate::abort()
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_set_misuse_handler(handler: Option<extern "C" fn()>) -> c_int {
    if sodium_crit_enter() != 0 {
        return -1;
    }
    unsafe {
        *(&raw mut _MISUSE_HANDLER) = handler;
    }
    if sodium_crit_leave() != 0 {
        return -1;
    }
    0
}
