//! Translation of `app/include/context.h`.
//!
//! `spx_ctx` holds the public/secret seeds and any backend-specific
//! precomputation state. Backend-specific fields are feature-gated to match
//! the `#ifdef SPX_SHA2` / `#ifdef SPX_HARAKA` sections of the C header.

use crate::params::SPX_N;

#[repr(C)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],

    // SHA2 backend: sha256 state that absorbed pub_seed, plus sha512 state
    // (only used when SPX_SHA512, but always present to keep the layout simple).
    #[cfg(feature = "sha2")]
    pub state_seeded: [u8; 40],
    #[cfg(feature = "sha2")]
    pub state_seeded_512: [u8; 72],

    // Haraka backend: tweaked round constants (active when no other backend is
    // selected, i.e. the default `haraka` backend).
    #[cfg(all(
        not(feature = "sha2"),
        not(feature = "shake"),
        not(feature = "blake")
    ))]
    pub tweaked512_rc64: [[u64; 8]; 10],
    #[cfg(all(
        not(feature = "sha2"),
        not(feature = "shake"),
        not(feature = "blake")
    ))]
    pub tweaked256_rc32: [[u32; 8]; 10],
}

impl SpxCtx {
    /// Creates a zero-initialised context (equivalent to `spx_ctx ctx;` after
    /// the seeds are populated by the caller).
    pub fn new() -> Self {
        SpxCtx {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
            #[cfg(feature = "sha2")]
            state_seeded: [0u8; 40],
            #[cfg(feature = "sha2")]
            state_seeded_512: [0u8; 72],
            #[cfg(all(
                not(feature = "sha2"),
                not(feature = "shake"),
                not(feature = "blake")
            ))]
            tweaked512_rc64: [[0u64; 8]; 10],
            #[cfg(all(
                not(feature = "sha2"),
                not(feature = "shake"),
                not(feature = "blake")
            ))]
            tweaked256_rc32: [[0u32; 8]; 10],
        }
    }
}

impl Default for SpxCtx {
    fn default() -> Self {
        Self::new()
    }
}
