//! `lib/shake` -- SHAKE256 based hash backend.

pub mod fips202;
pub mod hash;

#[cfg(feature = "simple")]
#[path = "thash_shake_simple.rs"]
pub mod thash;

#[cfg(not(feature = "simple"))]
#[path = "thash_shake_robust.rs"]
pub mod thash;
