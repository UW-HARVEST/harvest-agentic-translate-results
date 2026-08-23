//! Translated from sodium/core.c
//! HAVE_PTHREAD path for crit_enter/leave.
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

static mut SODIUM_LOCK: libc::pthread_mutex_t = unsafe { core::mem::zeroed() };
static LOCK_ONCE: std::sync::Once = std::sync::Once::new();

unsafe fn ensure_lock_init() {
    LOCK_ONCE.call_once(|| {
        libc::pthread_mutex_init(core::ptr::addr_of_mut!(SODIUM_LOCK), core::ptr::null());
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_crit_enter() -> c_int {
    ensure_lock_init();
    let ret = libc::pthread_mutex_lock(core::ptr::addr_of_mut!(SODIUM_LOCK));
    if ret == 0 {
        LOCKED = 1;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_crit_leave() -> c_int {
    if LOCKED == 0 {
        *libc::__errno_location() = libc::EPERM;
        return -1;
    }
    LOCKED = 0;
    libc::pthread_mutex_unlock(core::ptr::addr_of_mut!(SODIUM_LOCK))
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
