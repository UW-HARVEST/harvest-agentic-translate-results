//! Translation of app/include/context.h

use crate::params::SPX_N;

#[repr(C)]
#[derive(Clone)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],

    #[cfg(feature = "sha2")]
    /// sha256 state that absorbed pub_seed
    pub state_seeded: [u8; 40],

    /// sha512 state that absorbed pub_seed.
    ///
    /// `context.h` guards this field with `# if SPX_SHA512`, i.e. it only exists
    /// for the 192/256-bit sha2 parameter sets.  The `any(192*, 256*)` cfg is
    /// exactly equivalent to `SPX_BIG_HASH == 1` given the secpar precedence in
    /// `params.rs` (256f > 256s > 192f > 192s > 128f > 128s), so the Rust
    /// `SpxCtx` has the same size and layout as the C `spx_ctx` in every one of
    /// the 48 configurations.
    #[cfg(all(feature = "sha2", spx_big_hash))]
    pub state_seeded_512: [u8; 72],

    // `blake_offsets.h` defines no extra ctx state; the zero-sized field just
    // documents that.  The cfg conditions below mirror the backend precedence
    // in `src/backend/mod.rs` exactly (sha2 > shake > blake > haraka), so the
    // struct layout always agrees with the backend that is actually compiled in.
    #[cfg(all(feature = "blake", not(any(feature = "sha2", feature = "shake"))))]
    pub _blake_unused: [u8; 0],

    #[cfg(not(any(feature = "sha2", feature = "shake", feature = "blake")))]
    pub tweaked512_rc64: [[u64; 8]; 10],
    #[cfg(not(any(feature = "sha2", feature = "shake", feature = "blake")))]
    pub tweaked256_rc32: [[u32; 8]; 10],
}

impl SpxCtx {
    pub fn new() -> Self {
        SpxCtx {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
            #[cfg(feature = "sha2")]
            state_seeded: [0u8; 40],
            #[cfg(all(feature = "sha2", spx_big_hash))]
            state_seeded_512: [0u8; 72],
            #[cfg(all(feature = "blake", not(any(feature = "sha2", feature = "shake"))))]
            _blake_unused: [0u8; 0],
            #[cfg(not(any(feature = "sha2", feature = "shake", feature = "blake")))]
            tweaked512_rc64: [[0u64; 8]; 10],
            #[cfg(not(any(feature = "sha2", feature = "shake", feature = "blake")))]
            tweaked256_rc32: [[0u32; 8]; 10],
        }
    }
}

impl Default for SpxCtx {
    fn default() -> Self {
        Self::new()
    }
}
