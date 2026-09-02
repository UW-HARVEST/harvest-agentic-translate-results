pub mod poly1305;

// Translation of:
//   - crypto_onetimeauth/crypto_onetimeauth.c
//   - include/sodium/crypto_onetimeauth.h

use core::ffi::{c_char, c_int, c_void};

use crate::randombytes::randombytes_buf;

use self::poly1305::{
    crypto_onetimeauth_poly1305, crypto_onetimeauth_poly1305_BYTES,
    crypto_onetimeauth_poly1305_KEYBYTES, crypto_onetimeauth_poly1305_final,
    crypto_onetimeauth_poly1305_init, crypto_onetimeauth_poly1305_state,
    crypto_onetimeauth_poly1305_update, crypto_onetimeauth_poly1305_verify,
};

/* crypto_onetimeauth.h */

pub const crypto_onetimeauth_BYTES: usize = crypto_onetimeauth_poly1305_BYTES;
pub const crypto_onetimeauth_KEYBYTES: usize = crypto_onetimeauth_poly1305_KEYBYTES;
pub const crypto_onetimeauth_PRIMITIVE: &[u8] = b"poly1305\0";

/// `typedef crypto_onetimeauth_poly1305_state crypto_onetimeauth_state;`
pub type crypto_onetimeauth_state = crypto_onetimeauth_poly1305_state;

/* crypto_onetimeauth.c */

#[unsafe(no_mangle)]
pub extern "C" fn crypto_onetimeauth_statebytes() -> usize {
    core::mem::size_of::<crypto_onetimeauth_state>()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_onetimeauth_bytes() -> usize {
    crypto_onetimeauth_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_onetimeauth_keybytes() -> usize {
    crypto_onetimeauth_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    crypto_onetimeauth_poly1305(out, in_, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_verify(
    h: *const u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    crypto_onetimeauth_poly1305_verify(h, in_, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_init(
    state: *mut crypto_onetimeauth_state,
    key: *const u8,
) -> c_int {
    crypto_onetimeauth_poly1305_init(
        state as *mut crypto_onetimeauth_poly1305_state,
        key,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_update(
    state: *mut crypto_onetimeauth_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    crypto_onetimeauth_poly1305_update(
        state as *mut crypto_onetimeauth_poly1305_state,
        in_,
        inlen,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_final(
    state: *mut crypto_onetimeauth_state,
    out: *mut u8,
) -> c_int {
    crypto_onetimeauth_poly1305_final(
        state as *mut crypto_onetimeauth_poly1305_state,
        out,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_onetimeauth_primitive() -> *const c_char {
    crypto_onetimeauth_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_onetimeauth_KEYBYTES);
}
