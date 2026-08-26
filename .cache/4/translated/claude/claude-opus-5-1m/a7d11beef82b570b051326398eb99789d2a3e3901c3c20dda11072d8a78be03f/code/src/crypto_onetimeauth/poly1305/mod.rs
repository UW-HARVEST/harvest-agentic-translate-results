pub mod donna;

// `crypto_onetimeauth/poly1305/onetimeauth_poly1305.c`
//
// `HAVE_TI_MODE` / `HAVE_EMMINTRIN_H` are not defined in the reference build,
// so the sse2 backend does not exist and the donna backend is the only one.

use core::ffi::{c_int, c_void};

use crate::randombytes::randombytes_buf;

/// `include/sodium/crypto_onetimeauth_poly1305.h`:
/// `typedef struct CRYPTO_ALIGN(16) crypto_onetimeauth_poly1305_state {
///      unsigned char opaque[256]; } crypto_onetimeauth_poly1305_state;`
#[repr(C, align(16))]
pub struct crypto_onetimeauth_poly1305_state {
    pub opaque: [u8; 256],
}

pub const crypto_onetimeauth_poly1305_BYTES: usize = 16;
pub const crypto_onetimeauth_poly1305_KEYBYTES: usize = 32;

/// `crypto_onetimeauth/poly1305/onetimeauth_poly1305.h`
#[repr(C)]
pub struct crypto_onetimeauth_poly1305_implementation {
    pub onetimeauth: unsafe extern "C" fn(
        out: *mut u8,
        in_: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> c_int,
    pub onetimeauth_verify: unsafe extern "C" fn(
        h: *const u8,
        in_: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> c_int,
    pub onetimeauth_init: unsafe extern "C" fn(
        state: *mut crypto_onetimeauth_poly1305_state,
        key: *const u8,
    ) -> c_int,
    pub onetimeauth_update: unsafe extern "C" fn(
        state: *mut crypto_onetimeauth_poly1305_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int,
    pub onetimeauth_final: unsafe extern "C" fn(
        state: *mut crypto_onetimeauth_poly1305_state,
        out: *mut u8,
    ) -> c_int,
}

/// `static const crypto_onetimeauth_poly1305_implementation *implementation =
///      &crypto_onetimeauth_poly1305_donna_implementation;`
static mut implementation: *const crypto_onetimeauth_poly1305_implementation =
    &raw const donna::crypto_onetimeauth_poly1305_donna_implementation;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    unsafe { ((*implementation).onetimeauth)(out, in_, inlen, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_verify(
    h: *const u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    unsafe { ((*implementation).onetimeauth_verify)(h, in_, inlen, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_init(
    state: *mut crypto_onetimeauth_poly1305_state,
    key: *const u8,
) -> c_int {
    unsafe { ((*implementation).onetimeauth_init)(state, key) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_update(
    state: *mut crypto_onetimeauth_poly1305_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    unsafe { ((*implementation).onetimeauth_update)(state, in_, inlen) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_onetimeauth_poly1305_final(
    state: *mut crypto_onetimeauth_poly1305_state,
    out: *mut u8,
) -> c_int {
    unsafe { ((*implementation).onetimeauth_final)(state, out) }
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
    unsafe {
        implementation = &raw const donna::crypto_onetimeauth_poly1305_donna_implementation;
    }
    0
}
