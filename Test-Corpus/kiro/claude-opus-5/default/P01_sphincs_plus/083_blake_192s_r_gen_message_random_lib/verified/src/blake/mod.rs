//! The BLAKE hash backend (`lib/blake/`).

use crate::params::{SPX_ADDR_BYTES, SPX_N};

pub mod blake256;
pub mod blake512;
pub mod hash;

#[cfg(feature = "simple")]
pub mod thash_simple;
#[cfg(not(feature = "simple"))]
pub mod thash_robust;

pub use blake256::BlakeState256;
pub use blake512::BlakeState512;
pub use hash::*;

#[cfg(feature = "simple")]
pub use thash_simple::thash;
#[cfg(not(feature = "simple"))]
pub use thash_robust::thash;

/// `SPX_BLAKE512`: the 192/256 bit parameter sets use BLAKE-512, the 128 bit
/// ones use BLAKE-256 for all hashes.
pub const SPX_BLAKE512: bool = SPX_N >= 24;

/// Upper bound for the `inlen + 4` VLA in the MGF1 routines: the largest `in`
/// they are called with is either `SPX_N + SPX_ADDR_BYTES` (thash) or
/// `2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES` (hash_message).
pub const MGF1_INBUF_MAX: usize = {
    let a = SPX_N + SPX_ADDR_BYTES;
    let b = 2 * SPX_N + blake512::SPX_BLAKE512_OUTPUT_BYTES;
    (if a > b { a } else { b }) + 4
};

/// `spx_ctx` has no backend-specific tail for BLAKE.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BackendState;

impl BackendState {
    pub const fn new() -> Self {
        BackendState
    }
}

impl Default for BackendState {
    fn default() -> Self {
        Self::new()
    }
}
