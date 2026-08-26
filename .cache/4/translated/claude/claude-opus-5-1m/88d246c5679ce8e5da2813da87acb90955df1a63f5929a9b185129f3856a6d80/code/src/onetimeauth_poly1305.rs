//! Translation of `crypto_onetimeauth/poly1305/onetimeauth_poly1305.c`.
//!
//! `HAVE_TI_MODE` / `HAVE_EMMINTRIN_H` are not defined in the reference build,
//! so the SSE2 implementation does not exist and
//! `_crypto_onetimeauth_poly1305_pick_best_implementation()` unconditionally
//! selects the donna one.

use core::ffi::{c_int, c_ulonglong, c_void};

/// `crypto_onetimeauth_poly1305_state` from
/// `include/sodium/crypto_onetimeauth_poly1305.h`:
/// `typedef struct CRYPTO_ALIGN(16) { unsigned char opaque[256]; }`.
#[repr(C, align(16))]
pub struct crypto_onetimeauth_poly1305_state {
    pub opaque: [u8; 256],
}

/// `typedef struct crypto_onetimeauth_poly1305_implementation` from
/// `crypto_onetimeauth/poly1305/onetimeauth_poly1305.h`.
#[repr(C)]
pub struct crypto_onetimeauth_poly1305_implementation {
    pub onetimeauth: unsafe extern "C" fn(
        out: *mut u8,
        in_: *const u8,
        inlen: c_ulonglong,
        k: *const u8,
    ) -> c_int,
    pub onetimeauth_verify: unsafe extern "C" fn(
        h: *const u8,
        in_: *const u8,
        inlen: c_ulonglong,
        k: *const u8,
    ) -> c_int,
    pub onetimeauth_init: unsafe extern "C" fn(
        state: *mut crypto_onetimeauth_poly1305_state,
        key: *const u8,
    ) -> c_int,
    pub onetimeauth_update: unsafe extern "C" fn(
        state: *mut crypto_onetimeauth_poly1305_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int,
    pub onetimeauth_final: unsafe extern "C" fn(
        state: *mut crypto_onetimeauth_poly1305_state,
        out: *mut u8,
    ) -> c_int,
}

extern "C" {
    /// Defined in `crypto_onetimeauth/poly1305/donna/poly1305_donna.c`.
    static crypto_onetimeauth_poly1305_donna_implementation:
        crypto_onetimeauth_poly1305_implementation;

    fn randombytes_buf(buf: *mut c_void, size: usize);
}

/* #define crypto_onetimeauth_poly1305_BYTES 16U */
const crypto_onetimeauth_poly1305_BYTES: usize = 16;
/* #define crypto_onetimeauth_poly1305_KEYBYTES 32U */
const crypto_onetimeauth_poly1305_KEYBYTES: usize = 32;

static mut implementation: *const crypto_onetimeauth_poly1305_implementation =
    core::ptr::addr_of!(crypto_onetimeauth_poly1305_donna_implementation);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305(
    out: *mut u8,
    in_: *const u8,
    inlen: c_ulonglong,
    k: *const u8,
) -> c_int {
    ((*implementation).onetimeauth)(out, in_, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_verify(
    h: *const u8,
    in_: *const u8,
    inlen: c_ulonglong,
    k: *const u8,
) -> c_int {
    ((*implementation).onetimeauth_verify)(h, in_, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_init(
    state: *mut crypto_onetimeauth_poly1305_state,
    key: *const u8,
) -> c_int {
    ((*implementation).onetimeauth_init)(state, key)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_update(
    state: *mut crypto_onetimeauth_poly1305_state,
    in_: *const u8,
    inlen: c_ulonglong,
) -> c_int {
    ((*implementation).onetimeauth_update)(state, in_, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_final(
    state: *mut crypto_onetimeauth_poly1305_state,
    out: *mut u8,
) -> c_int {
    ((*implementation).onetimeauth_final)(state, out)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_bytes() -> usize {
    crypto_onetimeauth_poly1305_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_keybytes() -> usize {
    crypto_onetimeauth_poly1305_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_statebytes() -> usize {
    core::mem::size_of::<crypto_onetimeauth_poly1305_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_onetimeauth_poly1305_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_onetimeauth_poly1305_pick_best_implementation() -> c_int {
    implementation = core::ptr::addr_of!(crypto_onetimeauth_poly1305_donna_implementation);

    0
}
