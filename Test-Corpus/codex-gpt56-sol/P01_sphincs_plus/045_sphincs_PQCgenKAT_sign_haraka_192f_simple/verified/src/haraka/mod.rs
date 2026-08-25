//! `lib/haraka` — the Haraka hash backend.

pub mod aes_ct;
pub mod haraka;
pub mod hash_haraka;

#[cfg(any(feature = "robust", not(feature = "simple")))]
pub mod thash_haraka_robust;
#[cfg(all(feature = "simple", not(feature = "robust")))]
pub mod thash_haraka_simple;

#[cfg(any(feature = "robust", not(feature = "simple")))]
pub use thash_haraka_robust::SPX_thash;
#[cfg(all(feature = "simple", not(feature = "robust")))]
pub use thash_haraka_simple::SPX_thash;

pub use haraka::{
    SPX_haraka256, SPX_haraka512, SPX_haraka512_perm, SPX_haraka_S, SPX_haraka_S_inc_absorb,
    SPX_haraka_S_inc_finalize, SPX_haraka_S_inc_init, SPX_haraka_S_inc_squeeze,
    SPX_tweak_constants,
};
pub use hash_haraka::{
    SPX_gen_message_random, SPX_hash_message, SPX_initialize_hash_function, SPX_prf_addr,
};
