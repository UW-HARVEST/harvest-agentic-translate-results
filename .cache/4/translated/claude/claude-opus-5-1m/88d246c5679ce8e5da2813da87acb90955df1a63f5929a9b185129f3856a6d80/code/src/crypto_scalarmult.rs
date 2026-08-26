//! Translation of `c_src/libsodium/crypto_scalarmult/crypto_scalarmult.c`.
//!
//! Constants come from `include/sodium/crypto_scalarmult.h`:
//!   `crypto_scalarmult_BYTES       == crypto_scalarmult_curve25519_BYTES       == 32U`
//!   `crypto_scalarmult_SCALARBYTES == crypto_scalarmult_curve25519_SCALARBYTES == 32U`
//!   `crypto_scalarmult_PRIMITIVE   == "curve25519"`

use core::ffi::{c_char, c_int};

/* crypto_scalarmult.h */
const crypto_scalarmult_BYTES: usize = 32;
const crypto_scalarmult_SCALARBYTES: usize = 32;
const crypto_scalarmult_PRIMITIVE: &[u8] = b"curve25519\0";

extern "C" {
    /* crypto_scalarmult/curve25519/scalarmult_curve25519.c */
    fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> c_int;
    fn crypto_scalarmult_curve25519(q: *mut u8, n: *const u8, p: *const u8) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_primitive() -> *const c_char {
    crypto_scalarmult_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_base(q: *mut u8, n: *const u8) -> c_int {
    crypto_scalarmult_curve25519_base(q, n)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult(q: *mut u8, n: *const u8, p: *const u8) -> c_int {
    crypto_scalarmult_curve25519(q, n, p)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_bytes() -> usize {
    crypto_scalarmult_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_scalarbytes() -> usize {
    crypto_scalarmult_SCALARBYTES
}
