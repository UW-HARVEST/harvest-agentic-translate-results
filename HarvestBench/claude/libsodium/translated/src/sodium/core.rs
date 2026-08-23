//! `sodium/core.c`
//!
//! Neither `_WIN32`, `HAVE_PTHREAD` nor `HAVE_ATOMIC_OPS` is defined in the
//! reference build, so `sodium_crit_enter()`/`sodium_crit_leave()` are the
//! trivial `return 0;` variants.

use core::ffi::c_int;

use crate::common::abort;

// Cross-module calls go through the exported C symbol names so that module
// layout stays decoupled (see `private/implementations.h`).
unsafe extern "C" {
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

type MisuseHandler = extern "C" fn();

static mut MISUSE_HANDLER: Option<MisuseHandler> = None;

#[unsafe(no_mangle)]
pub extern "C" fn sodium_crit_enter() -> c_int {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_crit_leave() -> c_int {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_init() -> c_int {
    if sodium_crit_enter() != 0 {
        return -1;
    }
    if unsafe { INITIALIZED } != 0 {
        if sodium_crit_leave() != 0 {
            return -1;
        }
        return 1;
    }
    crate::sodium::runtime::_sodium_runtime_get_cpu_features();
    crate::randombytes::randombytes_stir();
    crate::sodium::utils::_sodium_alloc_init();
    unsafe {
        _crypto_pwhash_argon2_pick_best_implementation();
        _crypto_generichash_blake2b_pick_best_implementation();
        _crypto_onetimeauth_poly1305_pick_best_implementation();
        _crypto_scalarmult_curve25519_pick_best_implementation();
        _crypto_stream_chacha20_pick_best_implementation();
        _crypto_stream_salsa20_pick_best_implementation();
        _crypto_aead_aegis128l_pick_best_implementation();
        _crypto_aead_aegis256_pick_best_implementation();
        _crypto_ipcrypt_pick_best_implementation();
    }
    unsafe { INITIALIZED = 1 };
    if sodium_crit_leave() != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_misuse() -> ! {
    let _ = sodium_crit_leave();
    if sodium_crit_enter() == 0 {
        let handler = unsafe { MISUSE_HANDLER };
        if sodium_crit_leave() == 0 {
            if let Some(h) = handler {
                h();
            }
        }
    }
    unsafe { abort() }
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_set_misuse_handler(handler: Option<MisuseHandler>) -> c_int {
    if sodium_crit_enter() != 0 {
        return -1;
    }
    unsafe { MISUSE_HANDLER = handler };
    if sodium_crit_leave() != 0 {
        return -1;
    }
    0
}
