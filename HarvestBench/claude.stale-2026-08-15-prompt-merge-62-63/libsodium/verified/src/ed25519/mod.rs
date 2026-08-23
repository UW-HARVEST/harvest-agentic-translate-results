//! P2: ed25519 / curve25519 ref10 field arithmetic, group ops, scalarmult,
//! sign, crypto_core ed25519/ristretto255/h2c.

pub mod fe25519;
pub mod sc25519;
pub mod base;
pub mod base2;
pub mod ge25519;
pub mod ristretto255;
pub mod x25519;
pub mod scalarmult;
pub mod sha512;
pub mod h2c;
pub mod core_ed25519;
pub mod core_ristretto255;
pub mod sign;

extern "C" {
    fn sodium_is_zero(n: *const u8, nlen: usize) -> core::ffi::c_int;
}

/// Wrapper used by fe25519 iszero (calls foundation sodium_is_zero).
pub fn is_zero(s: &[u8]) -> i32 {
    unsafe { sodium_is_zero(s.as_ptr(), s.len()) }
}
