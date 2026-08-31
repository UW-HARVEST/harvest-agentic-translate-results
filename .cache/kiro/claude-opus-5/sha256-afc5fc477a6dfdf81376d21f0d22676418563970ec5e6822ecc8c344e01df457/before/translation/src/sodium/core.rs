//! Translation of `libsodium/sodium/core.c`
//!
//! Neither `_WIN32`, `HAVE_PTHREAD` nor `HAVE_ATOMIC_OPS` is defined by the
//! reference build, so `sodium_crit_enter()`/`sodium_crit_leave()` compile to
//! the trivial "always succeed" variants.

use core::ffi::c_int;

static mut INITIALIZED: c_int = 0;

extern "C" {
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

#[unsafe(no_mangle)]
pub extern "C" fn sodium_init() -> c_int {
    unsafe {
        if sodium_crit_enter() != 0 {
            return -1;
        }
        if core::ptr::read_volatile(core::ptr::addr_of!(INITIALIZED)) != 0 {
            if sodium_crit_leave() != 0 {
                return -1;
            }
            return 1;
        }
        crate::sodium::runtime::_sodium_runtime_get_cpu_features();
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
        core::ptr::write_volatile(core::ptr::addr_of_mut!(INITIALIZED), 1);
        if sodium_crit_leave() != 0 {
            return -1;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_crit_enter() -> c_int {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_crit_leave() -> c_int {
    0
}

type MisuseHandler = Option<extern "C" fn()>;

static mut MISUSE_HANDLER: MisuseHandler = None;

#[unsafe(no_mangle)]
pub extern "C" fn sodium_misuse() -> ! {
    let _ = sodium_crit_leave();
    if sodium_crit_enter() == 0 {
        let handler = unsafe { core::ptr::read(core::ptr::addr_of!(MISUSE_HANDLER)) };
        if sodium_crit_leave() == 0 {
            if let Some(h) = handler {
                h();
            }
        }
    }
    std::process::abort()
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_set_misuse_handler(handler: MisuseHandler) -> c_int {
    if sodium_crit_enter() != 0 {
        return -1;
    }
    unsafe {
        core::ptr::write(core::ptr::addr_of_mut!(MISUSE_HANDLER), handler);
    }
    if sodium_crit_leave() != 0 {
        return -1;
    }
    0
}
