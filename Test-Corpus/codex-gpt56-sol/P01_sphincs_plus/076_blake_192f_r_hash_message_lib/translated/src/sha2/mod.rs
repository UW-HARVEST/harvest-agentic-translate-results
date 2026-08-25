//! `lib/sha2` — the SHA2 hash backend.

pub mod hash_sha2;
pub mod sha2;

#[cfg(any(feature = "robust", not(feature = "simple")))]
pub mod thash_sha2_robust;
#[cfg(all(feature = "simple", not(feature = "robust")))]
pub mod thash_sha2_simple;

#[cfg(any(feature = "robust", not(feature = "simple")))]
pub use thash_sha2_robust::SPX_thash;
#[cfg(all(feature = "simple", not(feature = "robust")))]
pub use thash_sha2_simple::SPX_thash;

pub use hash_sha2::{
    SPX_gen_message_random, SPX_hash_message, SPX_initialize_hash_function, SPX_prf_addr,
};
pub use sha2::{
    sha256, sha256_inc_blocks, sha256_inc_finalize, sha256_inc_init, sha512, sha512_inc_blocks,
    sha512_inc_finalize, sha512_inc_init, SPX_mgf1_256, SPX_mgf1_512, SPX_seed_state,
};
