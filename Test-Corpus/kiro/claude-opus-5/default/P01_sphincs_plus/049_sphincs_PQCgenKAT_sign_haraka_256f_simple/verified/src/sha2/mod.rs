//! The SHA-2 hash backend (`lib/sha2/`).

use crate::params::SPX_N;

pub mod hash;
pub mod sha2;

#[cfg(feature = "simple")]
pub mod thash_simple;
#[cfg(not(feature = "simple"))]
pub mod thash_robust;

pub use hash::*;

#[cfg(feature = "simple")]
pub use thash_simple::thash;
#[cfg(not(feature = "simple"))]
pub use thash_robust::thash;

/// `SPX_SHA512`: the 192/256 bit parameter sets use SHA-512 for `H` and
/// `T_l, l >= 2`, the 128 bit ones use SHA-256 throughout.
pub const SPX_SHA512: bool = SPX_N >= 24;

/// `state_seeded_512` only exists when `SPX_SHA512` is set (see the
/// `# if SPX_SHA512` in `app/include/context.h`), so it collapses to a
/// zero-sized array otherwise and keeps `spx_ctx` the same size as in C.
pub type SpxCtxSha512State = [u8; if SPX_SHA512 { 72 } else { 0 }];

/// The `#ifdef SPX_SHA2` tail of `spx_ctx`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BackendState {
    /// sha256 state that absorbed pub_seed
    pub state_seeded: [u8; 40],
    /// sha512 state that absorbed pub_seed
    pub state_seeded_512: SpxCtxSha512State,
}

impl BackendState {
    pub const fn new() -> Self {
        BackendState {
            state_seeded: [0u8; 40],
            state_seeded_512: [0u8; if SPX_SHA512 { 72 } else { 0 }],
        }
    }
}

impl Default for BackendState {
    fn default() -> Self {
        Self::new()
    }
}
