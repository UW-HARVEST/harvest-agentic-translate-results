//! Haraka hash backend (`lib/haraka`).

mod haraka;
mod hash_haraka;
mod thash_haraka;

// Core hash interface used by the SPHINCS+ core.
pub use hash_haraka::{
    SPX_gen_message_random, SPX_hash_message, SPX_initialize_hash_function, SPX_prf_addr,
};
pub use thash_haraka::SPX_thash;

// Primitives used by the driver's KAT transcript.
pub use haraka::{
    SPX_haraka_S_inc_absorb as haraka_S_inc_absorb,
    SPX_haraka_S_inc_finalize as haraka_S_inc_finalize,
    SPX_haraka_S_inc_init as haraka_S_inc_init,
    SPX_haraka_S_inc_squeeze as haraka_S_inc_squeeze, SPX_tweak_constants as tweak_constants,
};
