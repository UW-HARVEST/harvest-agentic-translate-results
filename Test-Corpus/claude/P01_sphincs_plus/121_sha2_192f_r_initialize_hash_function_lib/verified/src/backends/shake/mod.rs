//! SHAKE256 hash backend (`lib/shake`).

pub mod fips202;
mod hash_shake;
mod thash_shake;

// Core hash interface used by the SPHINCS+ core.
pub use hash_shake::{
    SPX_gen_message_random, SPX_hash_message, SPX_initialize_hash_function, SPX_prf_addr,
};
pub use thash_shake::SPX_thash;

// Primitives used by the driver's KAT transcript.
pub use fips202::{
    shake256_inc_absorb, shake256_inc_finalize, shake256_inc_init, shake256_inc_squeeze,
};
