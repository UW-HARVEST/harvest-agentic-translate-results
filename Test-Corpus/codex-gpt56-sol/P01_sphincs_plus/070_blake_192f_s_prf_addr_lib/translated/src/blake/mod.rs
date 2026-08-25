//! `lib/blake` — the BLAKE hash backend.

pub mod blake256;
pub mod blake512;
pub mod hash_blake;

#[cfg(any(feature = "robust", not(feature = "simple")))]
pub mod thash_blake_robust;
#[cfg(all(feature = "simple", not(feature = "robust")))]
pub mod thash_blake_simple;

#[cfg(any(feature = "robust", not(feature = "simple")))]
pub use thash_blake_robust::SPX_thash;
#[cfg(all(feature = "simple", not(feature = "robust")))]
pub use thash_blake_simple::SPX_thash;

pub use blake256::{
    blake256, blake256_compress, blake256_final, blake256_init, blake256_update, SPX_blake256_mgf1,
};
pub use blake512::{
    blake512, blake512_compress, blake512_final, blake512_init, blake512_update, SPX_blake512_mgf1,
};
pub use hash_blake::{
    SPX_gen_message_random, SPX_hash_message, SPX_initialize_hash_function, SPX_prf_addr,
};
