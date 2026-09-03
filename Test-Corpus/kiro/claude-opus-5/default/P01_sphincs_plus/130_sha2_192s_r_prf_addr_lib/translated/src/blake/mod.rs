//! `lib/blake` -- BLAKE-256/BLAKE-512 based hash backend.

pub mod blake256;
pub mod blake512;
pub mod hash;

#[cfg(feature = "simple")]
#[path = "thash_blake_simple.rs"]
pub mod thash;

#[cfg(not(feature = "simple"))]
#[path = "thash_blake_robust.rs"]
pub mod thash;
