//! Translation of `app/include/context.h`.

use crate::params::SPX_N;

/// `spx_ctx`
///
/// The conditional members mirror the `#ifdef SPX_SHA2` / `#ifdef SPX_HARAKA`
/// blocks of `context.h`; `SPX_SHA2` and `SPX_HARAKA` are defined by the
/// backend's `*_offsets.h`.
#[repr(C)]
#[derive(Clone)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],

    /// sha256 state that absorbed pub_seed
    #[cfg(backend_sha2)]
    pub state_seeded: [u8; 40],

    /// sha512 state that absorbed pub_seed
    #[cfg(all(backend_sha2, spx_n_ge_24))]
    pub state_seeded_512: [u8; 72],

    #[cfg(backend_haraka)]
    pub tweaked512_rc64: [[u64; 8]; 10],
    #[cfg(backend_haraka)]
    pub tweaked256_rc32: [[u32; 8]; 10],
}

impl SpxCtx {
    /// A zeroed context, matching a `spx_ctx ctx;` automatic variable whose
    /// members are all assigned before use.
    pub const fn new() -> Self {
        SpxCtx {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
            #[cfg(backend_sha2)]
            state_seeded: [0u8; 40],
            #[cfg(all(backend_sha2, spx_n_ge_24))]
            state_seeded_512: [0u8; 72],
            #[cfg(backend_haraka)]
            tweaked512_rc64: [[0u64; 8]; 10],
            #[cfg(backend_haraka)]
            tweaked256_rc32: [[0u32; 8]; 10],
        }
    }
}

impl Default for SpxCtx {
    fn default() -> Self {
        Self::new()
    }
}
