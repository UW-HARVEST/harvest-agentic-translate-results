//! SHA2 hash backend (`lib/sha2`).

mod hash_sha2;
mod sha2;
mod thash_sha2;

// Core hash interface used by the SPHINCS+ core.
pub use hash_sha2::{
    SPX_gen_message_random, SPX_hash_message, SPX_initialize_hash_function, SPX_prf_addr,
};
pub use thash_sha2::SPX_thash;

// Primitives used by the driver's KAT transcript.
pub use sha2::{
    sha256_inc_blocks, sha256_inc_finalize, sha256_inc_init, sha512_inc_blocks,
    sha512_inc_finalize, sha512_inc_init,
};
