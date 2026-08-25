//! `lib/shake` — the SHAKE256 hash backend.

pub mod fips202;
pub mod hash_shake;

#[cfg(any(feature = "robust", not(feature = "simple")))]
pub mod thash_shake_robust;
#[cfg(all(feature = "simple", not(feature = "robust")))]
pub mod thash_shake_simple;

#[cfg(any(feature = "robust", not(feature = "simple")))]
pub use thash_shake_robust::SPX_thash;
#[cfg(all(feature = "simple", not(feature = "robust")))]
pub use thash_shake_simple::SPX_thash;

pub use hash_shake::{
    SPX_gen_message_random, SPX_hash_message, SPX_initialize_hash_function, SPX_prf_addr,
};
