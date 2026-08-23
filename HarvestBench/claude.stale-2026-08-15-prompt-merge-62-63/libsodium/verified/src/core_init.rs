//! Translated from sodium/core.c
//! `c_src/CMakeLists.txt` defines no `HAVE_*` macros, so core.c compiles the
//! final `#else` branch for the critical section: `sodium_crit_enter()` and
//! `sodium_crit_leave()` are unconditional `return 0` (the `locked` static is
//! never touched, so `sodium_crit_leave()` never reports EPERM).
#![allow(dead_code)]

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

static mut INITIALIZED: c_int = 0;
static mut LOCKED: c_int = 0;
static mut MISUSE_HANDLER: Option<extern "C" fn()> = None;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_crit_enter() -> c_int {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_crit_leave() -> c_int {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_init() -> c_int {
    if sodium_crit_enter() != 0 {
        return -1;
    }
    if INITIALIZED != 0 {
        if sodium_crit_leave() != 0 {
            return -1;
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
    INITIALIZED = 1;
    if sodium_crit_leave() != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_misuse() -> ! {
    unsafe {
        let _ = sodium_crit_leave();
        if sodium_crit_enter() == 0 {
            let handler = MISUSE_HANDLER;
            if sodium_crit_leave() == 0 {
                if let Some(h) = handler {
                    h();
                }
            }
        }
        libc::abort();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_set_misuse_handler(handler: Option<extern "C" fn()>) -> c_int {
    if sodium_crit_enter() != 0 {
        return -1;
    }
    MISUSE_HANDLER = handler;
    if sodium_crit_leave() != 0 {
        return -1;
    }
    0
}
