//! Translation of `app/include/context.h` (the `spx_ctx` structure).
//!
//! The fields present depend on the active hash backend, exactly as the C
//! header uses `#ifdef SPX_SHA2` / `#ifdef SPX_HARAKA`.

use crate::params::SPX_N;

#[derive(Clone)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],

    #[cfg(spx_backend = "sha2")]
    /// SHA-256 state that absorbed pub_seed.
    pub state_seeded: [u8; 40],

    #[cfg(all(spx_backend = "sha2", spx_sha512))]
    /// SHA-512 state that absorbed pub_seed.
    pub state_seeded_512: [u8; 72],

    #[cfg(spx_backend = "haraka")]
    pub tweaked512_rc64: [[u64; 8]; 10],
    #[cfg(spx_backend = "haraka")]
    pub tweaked256_rc32: [[u32; 8]; 10],
}

impl SpxCtx {
    /// A zero-initialised context (matches C stack `spx_ctx ctx;` followed by
    /// field assignment; the seeds are always overwritten before use).
    pub fn new() -> Self {
        SpxCtx {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
            #[cfg(spx_backend = "sha2")]
            state_seeded: [0u8; 40],
            #[cfg(all(spx_backend = "sha2", spx_sha512))]
            state_seeded_512: [0u8; 72],
            #[cfg(spx_backend = "haraka")]
            tweaked512_rc64: [[0u64; 8]; 10],
            #[cfg(spx_backend = "haraka")]
            tweaked256_rc32: [[0u32; 8]; 10],
        }
    }
}

impl Default for SpxCtx {
    fn default() -> Self {
        Self::new()
    }
}
