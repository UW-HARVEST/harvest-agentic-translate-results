//! `lib/haraka` -- Haraka based hash backend.

/// Bit-sliced constant-time AES round primitives from the first half of
/// `lib/haraka/src/haraka.c` (BearSSL derived).
pub mod aes_ct;

#[path = "haraka.rs"]
pub mod haraka;
pub mod hash;

#[cfg(feature = "simple")]
#[path = "thash_haraka_simple.rs"]
pub mod thash;

#[cfg(not(feature = "simple"))]
#[path = "thash_haraka_robust.rs"]
pub mod thash;
