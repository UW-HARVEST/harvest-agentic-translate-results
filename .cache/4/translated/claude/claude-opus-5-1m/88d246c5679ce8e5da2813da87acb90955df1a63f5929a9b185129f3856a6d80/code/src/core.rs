//! Translation of `sodium/core.c`.
//!
//! Exports:
//!   * `sodium_crit_enter`
//!   * `sodium_crit_leave`
//!   * `sodium_init`
//!   * `sodium_misuse`
//!   * `sodium_set_misuse_handler`
//!
//! The reference build defines neither `_WIN32`, `HAVE_PTHREAD` nor
//! `HAVE_ATOMIC_OPS`, so the `#else` branch of the locking implementation is
//! taken: `sodium_crit_enter()`/`sodium_crit_leave()` are unconditional
//! `return 0;` stubs and there is no lock object at all.  This was confirmed by
//! running the C preprocessor over `sodium/core.c` with the reference include
//! path.

use core::ffi::c_int;

/// `void (*)(void)` -- the type of the user supplied misuse handler.
type misuse_handler_t = unsafe extern "C" fn();

extern "C" {
    /* <stdlib.h> */
    fn abort() -> !;

    /* sodium/runtime.c */
    fn _sodium_runtime_get_cpu_features() -> c_int;

    /* randombytes/randombytes.c */
    fn randombytes_stir();

    /* sodium/utils.c */
    fn _sodium_alloc_init() -> c_int;

    /* private/implementations.h */
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

/* static volatile int initialized; */
static mut initialized: c_int = 0;

/* static volatile int locked;
 * Only referenced by the `_WIN32` / `HAVE_PTHREAD` locking implementations,
 * which are not compiled in the reference build.  Kept for fidelity. */
static mut locked: c_int = 0;

/* static void (*_misuse_handler)(void); */
static mut _misuse_handler: Option<misuse_handler_t> = None;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_init() -> c_int {
    if sodium_crit_enter() != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    if core::ptr::read_volatile(core::ptr::addr_of!(initialized)) != 0 {
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
    core::ptr::write_volatile(core::ptr::addr_of_mut!(initialized), 1);
    if sodium_crit_leave() != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_crit_enter() -> c_int {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_crit_leave() -> c_int {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_misuse() -> ! {
    let handler: Option<misuse_handler_t>;

    let _ = sodium_crit_leave();
    if sodium_crit_enter() == 0 {
        handler = core::ptr::read_volatile(core::ptr::addr_of!(_misuse_handler));
        if sodium_crit_leave() == 0 {
            if let Some(h) = handler {
                h();
            }
        }
    }
    /* LCOV_EXCL_START */
    abort();
}
/* LCOV_EXCL_STOP */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_set_misuse_handler(handler: Option<misuse_handler_t>) -> c_int {
    if sodium_crit_enter() != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    core::ptr::write_volatile(core::ptr::addr_of_mut!(_misuse_handler), handler);
    if sodium_crit_leave() != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}
