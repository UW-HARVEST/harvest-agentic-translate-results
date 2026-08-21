//! BLAKE hash backend (`lib/blake`).

mod blake256;
mod blake512;
mod hash_blake;
mod thash_blake;

// Core hash interface used by the SPHINCS+ core.
pub use hash_blake::{
    SPX_gen_message_random, SPX_hash_message, SPX_initialize_hash_function, SPX_prf_addr,
};
pub use thash_blake::SPX_thash;

// Primitives used by the driver's KAT transcript.
pub use blake256::{blake256_final, blake256_init, blake256_update, blakestate256};
pub use blake512::{blake512_final, blake512_init, blake512_update, blakestate512};
