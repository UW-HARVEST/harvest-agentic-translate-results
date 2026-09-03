//! `lib/sha2` -- SHA-256/SHA-512 based hash backend.

#[path = "sha2.rs"]
pub mod sha2;
pub mod hash;

#[cfg(feature = "simple")]
#[path = "thash_sha2_simple.rs"]
pub mod thash;

#[cfg(not(feature = "simple"))]
#[path = "thash_sha2_robust.rs"]
pub mod thash;
