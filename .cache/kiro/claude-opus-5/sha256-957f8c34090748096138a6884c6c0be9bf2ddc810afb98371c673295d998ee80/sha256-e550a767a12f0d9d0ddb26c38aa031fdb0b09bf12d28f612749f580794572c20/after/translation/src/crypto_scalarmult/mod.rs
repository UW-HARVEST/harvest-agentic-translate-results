pub mod curve25519;
pub mod ed25519;
pub mod ristretto255;

// Translation of `crypto_scalarmult/crypto_scalarmult.c` and
// `include/sodium/crypto_scalarmult.h`.

use core::ffi::{c_char, c_int};

use crate::crypto_scalarmult::curve25519::{
    crypto_scalarmult_curve25519, crypto_scalarmult_curve25519_base,
    crypto_scalarmult_curve25519_BYTES, crypto_scalarmult_curve25519_SCALARBYTES,
};

/* ---- constants from crypto_scalarmult.h ---- */

pub const crypto_scalarmult_BYTES: usize = crypto_scalarmult_curve25519_BYTES;
pub const crypto_scalarmult_SCALARBYTES: usize = crypto_scalarmult_curve25519_SCALARBYTES;
pub const crypto_scalarmult_PRIMITIVE: &[u8] = b"curve25519\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_primitive() -> *const c_char {
    crypto_scalarmult_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_base(q: *mut u8, n: *const u8) -> c_int {
    crypto_scalarmult_curve25519_base(q, n)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    crypto_scalarmult_curve25519(q, n, p)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_bytes() -> usize {
    crypto_scalarmult_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_scalarbytes() -> usize {
    crypto_scalarmult_SCALARBYTES
}
