use crate::params::SPX_N;

/// SPHINCS+ context structure. Mirrors the C `spx_ctx` exactly.
///
/// In C, the layout depends on the backend:
/// - `pub_seed[SPX_N]`, `sk_seed[SPX_N]` always present
/// - `state_seeded[40]` and `state_seeded_512[72]` if SPX_SHA2
/// - `tweaked512_rc64[10][8]` and `tweaked256_rc32[10][8]` if SPX_HARAKA
#[repr(C)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],

    #[cfg(feature = "sha2")]
    pub state_seeded: [u8; 40],
    #[cfg(all(feature = "sha2", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
    pub state_seeded_512: [u8; 72],

    #[cfg(feature = "haraka")]
    pub tweaked512_rc64: [[u64; 8]; 10],
    #[cfg(feature = "haraka")]
    pub tweaked256_rc32: [[u32; 8]; 10],
}

impl SpxCtx {
    pub fn zeroed() -> Self {
        // SAFETY: SpxCtx is composed entirely of integer arrays, so zeroed bits
        // form a valid Rust value.
        unsafe { core::mem::MaybeUninit::<SpxCtx>::zeroed().assume_init() }
    }
}
