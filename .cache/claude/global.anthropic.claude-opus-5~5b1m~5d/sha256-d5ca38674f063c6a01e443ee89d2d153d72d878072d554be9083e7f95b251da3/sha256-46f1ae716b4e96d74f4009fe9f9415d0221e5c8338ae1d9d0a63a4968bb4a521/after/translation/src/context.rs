//! Translation of app/include/context.h

use crate::params::{SPX_N, SPX_SHA512};

/// The C `spx_ctx` only declares `state_seeded_512` under `# if SPX_SHA512`, so
/// for the sha2-128s/128f parameter sets the struct is 72 bytes, not 144.
/// Keeping the field but giving it length 0 in that case preserves both the
/// exact C `sizeof(spx_ctx)` (part of the FFI ABI) and the ability to keep the
/// `if SPX_SHA512 == 1` runtime guards in `hash_sha2.rs` / `thash_sha2.rs`.
pub const SPX_STATE_SEEDED_512_LEN: usize = if SPX_SHA512 == 1 { 72 } else { 0 };

#[repr(C)]
#[derive(Clone)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],

    #[cfg(feature = "sha2")]
    /// sha256 state that absorbed pub_seed
    pub state_seeded: [u8; 40],

    #[cfg(feature = "sha2")]
    /// sha512 state that absorbed pub_seed (only used when SPX_SHA512 == 1)
    pub state_seeded_512: [u8; SPX_STATE_SEEDED_512_LEN],

    #[cfg(feature = "blake")]
    pub _blake_unused: [u8; 0],

    #[cfg(feature = "haraka")]
    pub tweaked512_rc64: [[u64; 8]; 10],
    #[cfg(feature = "haraka")]
    pub tweaked256_rc32: [[u32; 8]; 10],
}

impl SpxCtx {
    pub fn new() -> Self {
        SpxCtx {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
            #[cfg(feature = "sha2")]
            state_seeded: [0u8; 40],
            #[cfg(feature = "sha2")]
            state_seeded_512: [0u8; SPX_STATE_SEEDED_512_LEN],
            #[cfg(feature = "blake")]
            _blake_unused: [0u8; 0],
            #[cfg(feature = "haraka")]
            tweaked512_rc64: [[0u64; 8]; 10],
            #[cfg(feature = "haraka")]
            tweaked256_rc32: [[0u32; 8]; 10],
        }
    }
}

impl Default for SpxCtx {
    fn default() -> Self {
        Self::new()
    }
}
