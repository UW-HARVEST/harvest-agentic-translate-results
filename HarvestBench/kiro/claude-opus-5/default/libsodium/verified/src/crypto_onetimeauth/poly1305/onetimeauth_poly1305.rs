//! Translation of c_src/libsodium/crypto_onetimeauth/poly1305/onetimeauth_poly1305.c

use core::ffi::c_int;
use core::ffi::c_void;

// crypto_onetimeauth_poly1305_state is the public 256-byte aligned struct.
#[repr(C, align(16))]
struct CryptoOnetimeauthPoly1305State {
    opaque: [u8; 256],
}

// #[repr(C)] mirror of crypto_onetimeauth_poly1305_implementation from
// crypto_onetimeauth/poly1305/onetimeauth_poly1305.h.
#[repr(C)]
struct CryptoOnetimeauthPoly1305Implementation {
    onetimeauth: Option<
        unsafe extern "C" fn(out: *mut u8, in_: *const u8, inlen: u64, k: *const u8) -> c_int,
    >,
    onetimeauth_verify: Option<
        unsafe extern "C" fn(h: *const u8, in_: *const u8, inlen: u64, k: *const u8) -> c_int,
    >,
    onetimeauth_init: Option<
        unsafe extern "C" fn(
            state: *mut CryptoOnetimeauthPoly1305State,
            key: *const u8,
        ) -> c_int,
    >,
    onetimeauth_update: Option<
        unsafe extern "C" fn(
            state: *mut CryptoOnetimeauthPoly1305State,
            in_: *const u8,
            inlen: u64,
        ) -> c_int,
    >,
    onetimeauth_final: Option<
        unsafe extern "C" fn(
            state: *mut CryptoOnetimeauthPoly1305State,
            out: *mut u8,
        ) -> c_int,
    >,
}

unsafe impl Sync for CryptoOnetimeauthPoly1305Implementation {}

const CRYPTO_ONETIMEAUTH_POLY1305_BYTES: usize = 16;
const CRYPTO_ONETIMEAUTH_POLY1305_KEYBYTES: usize = 32;

extern "C" {
    // Defined in donna/poly1305_donna.c
    static crypto_onetimeauth_poly1305_donna_implementation:
        CryptoOnetimeauthPoly1305Implementation;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

// static const crypto_onetimeauth_poly1305_implementation *implementation =
//     &crypto_onetimeauth_poly1305_donna_implementation;
// HAVE_TI_MODE && HAVE_EMMINTRIN_H is false: only the donna implementation.
static mut IMPLEMENTATION: *const CryptoOnetimeauthPoly1305Implementation =
    core::ptr::addr_of!(crypto_onetimeauth_poly1305_donna_implementation);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    ((*IMPLEMENTATION).onetimeauth.unwrap_unchecked())(out, in_, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_verify(
    h: *const u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    ((*IMPLEMENTATION).onetimeauth_verify.unwrap_unchecked())(h, in_, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_init(
    state: *mut CryptoOnetimeauthPoly1305State,
    key: *const u8,
) -> c_int {
    ((*IMPLEMENTATION).onetimeauth_init.unwrap_unchecked())(state, key)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_update(
    state: *mut CryptoOnetimeauthPoly1305State,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    ((*IMPLEMENTATION).onetimeauth_update.unwrap_unchecked())(state, in_, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_final(
    state: *mut CryptoOnetimeauthPoly1305State,
    out: *mut u8,
) -> c_int {
    ((*IMPLEMENTATION).onetimeauth_final.unwrap_unchecked())(state, out)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_bytes() -> usize {
    CRYPTO_ONETIMEAUTH_POLY1305_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_keybytes() -> usize {
    CRYPTO_ONETIMEAUTH_POLY1305_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_statebytes() -> usize {
    core::mem::size_of::<CryptoOnetimeauthPoly1305State>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_ONETIMEAUTH_POLY1305_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_onetimeauth_poly1305_pick_best_implementation() -> c_int {
    IMPLEMENTATION = core::ptr::addr_of!(crypto_onetimeauth_poly1305_donna_implementation);
    // HAVE_TI_MODE && HAVE_EMMINTRIN_H is false: no SSE2 override.
    0
}
