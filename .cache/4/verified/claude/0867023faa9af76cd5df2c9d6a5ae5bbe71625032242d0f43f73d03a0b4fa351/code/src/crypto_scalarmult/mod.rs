pub mod curve25519;
pub mod ed25519;
pub mod ristretto255;

// ---------------------------------------------------------------------------
// Translation of `crypto_scalarmult/crypto_scalarmult.c`
// ---------------------------------------------------------------------------

use core::ffi::{c_char, c_int};

/// `#define crypto_scalarmult_BYTES crypto_scalarmult_curve25519_BYTES`
pub const crypto_scalarmult_BYTES: usize = self::curve25519::crypto_scalarmult_curve25519_BYTES;

/// `#define crypto_scalarmult_SCALARBYTES crypto_scalarmult_curve25519_SCALARBYTES`
pub const crypto_scalarmult_SCALARBYTES: usize =
    self::curve25519::crypto_scalarmult_curve25519_SCALARBYTES;

/// `#define crypto_scalarmult_PRIMITIVE "curve25519"`
pub const crypto_scalarmult_PRIMITIVE: &[u8; 11] = b"curve25519\0";

unsafe extern "C" {
    fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> c_int;
    fn crypto_scalarmult_curve25519(q: *mut u8, n: *const u8, p: *const u8) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_primitive() -> *const c_char {
    crypto_scalarmult_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_base(q: *mut u8, n: *const u8) -> c_int {
    unsafe { crypto_scalarmult_curve25519_base(q, n) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult(q: *mut u8, n: *const u8, p: *const u8) -> c_int {
    unsafe { crypto_scalarmult_curve25519(q, n, p) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_bytes() -> usize {
    crypto_scalarmult_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_scalarbytes() -> usize {
    crypto_scalarmult_SCALARBYTES
}
