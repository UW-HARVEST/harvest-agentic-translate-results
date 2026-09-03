//! Translation of `app/include/context.h`.

use crate::params::*;

/// `spx_ctx` from context.h.
///
/// The layout is parameter-set dependent exactly as in C:
///  * the SHA-2 backend appends the pre-seeded SHA-256 midstate, and for
///    `SPX_SHA512 != 0` also the pre-seeded SHA-512 midstate;
///  * the Haraka backend appends the tweaked round constants.
#[repr(C)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],

    // #ifdef SPX_SHA2
    #[cfg(feature = "sha2")]
    /// sha256 state that absorbed pub_seed
    pub state_seeded: [u8; 40],

    // # if SPX_SHA512  (i.e. SPX_N >= 24)
    #[cfg(all(
        feature = "sha2",
        any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")
    ))]
    /// sha512 state that absorbed pub_seed
    pub state_seeded_512: [u8; 72],

    // #ifdef SPX_HARAKA
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
    /// Zero-initialised context, matching a C stack object that is fully
    /// written before use.
    pub const fn new() -> Self {
        SpxCtx {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
            #[cfg(feature = "sha2")]
            state_seeded: [0u8; 40],
            #[cfg(all(
                feature = "sha2",
                any(
                    feature = "192s",
                    feature = "192f",
                    feature = "256s",
                    feature = "256f"
                )
            ))]
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
