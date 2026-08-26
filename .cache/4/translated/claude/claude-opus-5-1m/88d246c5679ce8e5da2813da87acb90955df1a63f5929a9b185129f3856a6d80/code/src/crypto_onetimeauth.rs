//! Translation of `crypto_onetimeauth/crypto_onetimeauth.c`.

use core::ffi::{c_char, c_int, c_ulonglong, c_void};

/// `crypto_onetimeauth_poly1305_state` from
/// `include/sodium/crypto_onetimeauth_poly1305.h`;
/// `crypto_onetimeauth_state` is a typedef of it.
#[repr(C, align(16))]
pub struct crypto_onetimeauth_state {
    pub opaque: [u8; 256],
}

extern "C" {
    fn crypto_onetimeauth_poly1305(
        out: *mut u8,
        in_: *const u8,
        inlen: c_ulonglong,
        k: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_verify(
        h: *const u8,
        in_: *const u8,
        inlen: c_ulonglong,
        k: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_init(
        state: *mut crypto_onetimeauth_state,
        key: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_update(
        state: *mut crypto_onetimeauth_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_final(
        state: *mut crypto_onetimeauth_state,
        out: *mut u8,
    ) -> c_int;

    fn randombytes_buf(buf: *mut c_void, size: usize);
}

/* #define crypto_onetimeauth_BYTES crypto_onetimeauth_poly1305_BYTES  (16U) */
const crypto_onetimeauth_BYTES: usize = 16;
/* #define crypto_onetimeauth_KEYBYTES crypto_onetimeauth_poly1305_KEYBYTES (32U) */
const crypto_onetimeauth_KEYBYTES: usize = 32;
/* #define crypto_onetimeauth_PRIMITIVE "poly1305" */
static crypto_onetimeauth_PRIMITIVE: [u8; 9] = *b"poly1305\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_statebytes() -> usize {
    core::mem::size_of::<crypto_onetimeauth_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_bytes() -> usize {
    crypto_onetimeauth_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_keybytes() -> usize {
    crypto_onetimeauth_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth(
    out: *mut u8,
    in_: *const u8,
    inlen: c_ulonglong,
    k: *const u8,
) -> c_int {
    crypto_onetimeauth_poly1305(out, in_, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_verify(
    h: *const u8,
    in_: *const u8,
    inlen: c_ulonglong,
    k: *const u8,
) -> c_int {
    crypto_onetimeauth_poly1305_verify(h, in_, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_init(
    state: *mut crypto_onetimeauth_state,
    key: *const u8,
) -> c_int {
    crypto_onetimeauth_poly1305_init(state, key)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_update(
    state: *mut crypto_onetimeauth_state,
    in_: *const u8,
    inlen: c_ulonglong,
) -> c_int {
    crypto_onetimeauth_poly1305_update(state, in_, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_final(
    state: *mut crypto_onetimeauth_state,
    out: *mut u8,
) -> c_int {
    crypto_onetimeauth_poly1305_final(state, out)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_primitive() -> *const c_char {
    crypto_onetimeauth_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_onetimeauth_KEYBYTES);
}
