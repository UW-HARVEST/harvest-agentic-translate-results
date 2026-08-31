//! Translation of c_src/libsodium/crypto_onetimeauth/crypto_onetimeauth.c

use core::ffi::{c_char, c_int, c_void};

// crypto_onetimeauth_state == crypto_onetimeauth_poly1305_state: public
// 256-byte aligned struct.
#[repr(C, align(16))]
struct CryptoOnetimeauthState {
    opaque: [u8; 256],
}

const CRYPTO_ONETIMEAUTH_BYTES: usize = 16; // crypto_onetimeauth_poly1305_BYTES
const CRYPTO_ONETIMEAUTH_KEYBYTES: usize = 32; // crypto_onetimeauth_poly1305_KEYBYTES

// crypto_onetimeauth_PRIMITIVE "poly1305"
const CRYPTO_ONETIMEAUTH_PRIMITIVE: &[u8] = b"poly1305\0";

extern "C" {
    fn crypto_onetimeauth_poly1305(
        out: *mut u8,
        in_: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_verify(
        h: *const u8,
        in_: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_init(
        state: *mut CryptoOnetimeauthState,
        key: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_update(
        state: *mut CryptoOnetimeauthState,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_final(
        state: *mut CryptoOnetimeauthState,
        out: *mut u8,
    ) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_statebytes() -> usize {
    core::mem::size_of::<CryptoOnetimeauthState>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_bytes() -> usize {
    CRYPTO_ONETIMEAUTH_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_keybytes() -> usize {
    CRYPTO_ONETIMEAUTH_KEYBYTES
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
    state: *mut CryptoOnetimeauthState,
    key: *const u8,
) -> c_int {
    crypto_onetimeauth_poly1305_init(state as *mut CryptoOnetimeauthState, key)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_update(
    state: *mut CryptoOnetimeauthState,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    crypto_onetimeauth_poly1305_update(state as *mut CryptoOnetimeauthState, in_, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_final(
    state: *mut CryptoOnetimeauthState,
    out: *mut u8,
) -> c_int {
    crypto_onetimeauth_poly1305_final(state as *mut CryptoOnetimeauthState, out)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_primitive() -> *const c_char {
    CRYPTO_ONETIMEAUTH_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_ONETIMEAUTH_KEYBYTES);
}
